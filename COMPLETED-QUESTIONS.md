# Research questions closed

The lines [OPEN-QUESTIONS.md](OPEN-QUESTIONS.md) has closed, each with its
answer, its counts and its control, and a link to the doc that now holds it —
under the same sections the queue keeps. A remainder a closed line names is
queued there as a line of its own. The file opens with the record of the rounds
that closed them: what each took, its negatives and their controls, where the
engine changed, and every premise of an earlier round it corrected.

## The rounds

The two files were one until 2026-09-30, so where a round below says *this
file* it means the queue as it then stood, closed lines and all.

The console's three commands were worked on **2026-09-30**, raised by checking Campaign 2
Mission 02 against a recording. All three closed, with the engine made to answer function 57:
`create` makes a unit the way the mission loader does and files it with its clan before the script
goes on; `bcreate` makes a building standing finished; `death` fells scenery. The negatives each
carry their control. `bcreate` runs no construction sphere, against the builder's call that passes
create bit 1 and gets one. `death` kills no unit or building, against the construction sphere's
kill, whose `0x414` takes units through the same query. And the two dropped fields are read nowhere,
against the four fields beside each that are read. **This work corrects two premises.**
[04-missions](docs/04-missions.md#the-start-flag-keeps-a-building-at-its-file-height--read-and-measured)
closed the start flag on 2026-09-18 as changing nothing, from a pass that found no caller of
`IBuilding` slot 13. The landscape insertion calls it twice through an answer it keeps on the stack,
and the flag keeps a building at its file height, which accounts for 12 of the 16 buildings
[03-terrain](docs/03-terrain.md) had standing off their contour's mean. The earlier entry is rewritten
where it stands. And [34-progression](docs/34-progression.md)'s table of every clan-table read
followed by a slot-4 filing was two short, the console's own. A sweep of all 17 reads finds ten, the
eight it had among them. [36-factory](docs/36-factory.md)'s *derived* "the construction is the only
robot the game adds at run time" was wrong for the same reason, and what raises its ready voice is
now read as a building's own callback. One thing ran the other way. The engine's `place_building`
gave every building a builder puts up the same logical id, `0x80000000`, since scenery's −1 topped
its count. It now counts past it.

A round of twelve was worked on **2026-09-18**, four areas of three. Seven
closed outright, four are answered as far as the shipped files allow — each
with a control showing the search that found nothing would have found
something — and one narrowed. A thirteenth, contact flag 2, closed as a side
effect. The engine changed in two places: a tracked warbot's belts now lie
along the ground under them, and seven of the formula evaluator's thirteen
operators were wrong and are fixed. Three doc corrections came out of it —
the `.fml` operator table's address, the Large builder's module socket, and
the eight `b?` prefixes, which are rounds and never were buildings — and two
premises in this file were wrong and are rewritten above: the six turrets'
gate is measured over 17 player trees, not 11, and the "five labels and the
label inside a block" is five plus one, the one being no anomaly.

A second round of twelve, the same four areas of three, was worked the same
day. **All twelve closed**, one of them because an earlier round had already
answered it and this file was never ticked. Three are negatives carrying their
controls: no module outside `Terrain.dll` reads `ForceSWFog`, not one of the
275882 shipped land faces carries either bit the ground search excludes, and
nothing reads an `.exp`'s two 1.0 floats. The engine changed in five places,
and elsewhere it was already right, which the docs now record as read rather
than assumed: a tree and a stone carry node life like any other
agent and are built at their placement scale, so a stone scaled 21 holds
billions rather than thousands; a vital node's death now takes node 0 with it;
a mine's lode and its total meet half way, so a lode of 600 yields 350; a chord
falls through to the plain row by the read rule rather than by luck; and a
walker's feet are laid along the ground by the pose its state ends in.

That last one **corrects a premise of the round before it**, and both entries
are rewritten where they stand. `CONTACT_PLACE` is not only the authored flag 2 on twelve
belt contacts: flag `0x20` makes the engine work the flag out afresh each time
a machine takes a state, from the contact's own axis in the state's end pose,
and it comes out set on **2229 of the install's 2634 contacts** rather than
twelve. Every walker's feet conform to the terrain, not just the tracked
chassis' belts. Two smaller corrections came out of the round: the flag is
worked out at each state, not at load ([13-control](docs/13-control.md)) — the
*ground contact's* own pass runs once a frame, as the round after this one
established — and the
derivation behind "a clan orders a bot it has no mind for" was reading `op5` as
`==` when it is `!=` ([23-economy](docs/23-economy.md)).

A third round of twelve was worked the same day, the same four areas of three.
**All twelve closed**, one of them — how the shade lights a type-1 light — leaving
a named remainder that is now a line of its own. Three carry negatives with their
controls: an effect light's attenuation triple is read by nothing, a SuperAI's
object list has no erase and no clear, and three of the four bits a round's ground
query excludes are on no shipped face. Four more queue lines went with them, all
four **stale** rather than researched — the message box, `info_system`, the hero's
death and the sound falloff were answered in the docs and never ticked here; the
first three are struck, and the fourth is rewritten to the one thing still open,
how DirectSound pans. The engine changed in a dozen places across four crates, and
the largest is not a stand-in at all: the ground contact's second sphere r₂ had
been taken as r and is the node sphere's, which differs from r on **all 148** unit
models the campaign places, so the up pass was wrong on every unit in the game.

Two premises of earlier work are corrected where they stand.
[20-resources](docs/20-resources.md) read a sound descriptor's `type` as *streamed
against sampled*: it is not, and the withdrawal came with a replacement — types 4,
5 and 7 build the same object and differ only in the flags their binding is made
with, `0`, `2` and `0x200`, of which bit 1 survives into
`IDirectSoundBuffer::Play` as `DSBPLAY_LOOPING`, so an ambient theme loops
**because** its descriptor is type 5 and a variation does not because it is type 4.
And [29-weapons](docs/29-weapons.md)'s charge level of a gun with no capacity was
said to be set by a shot; **nothing** sets it, all three writers sitting behind a
`value 1 > 0` gate, so the five such components keep their constructor's 0.

Two of this round's own claims were overturned **before they landed**, both by the
coordinator's spot-checks rather than by the agent that made them. The ambient
count was reported as 170 variations and is **171 lines binding 170 names**,
because `Single.02` writes `DAY_VARIATION1` twice and the loader keeps the later;
and chasing that duplicate through the loader is what turned up the type-5 loop
flag above, which had been published as a negative — *"type 5 does not loop"* — on
a read that had stopped one table too early. Which way that ran is worth recording:
the negative was not wrong for want of evidence but for want of a second question,
and it is the spot-check, not the research, that asked it.

One earlier reading is **refined rather than corrected**. The round before this
established that `CONTACT_PLACE` is worked out from the state's end pose, and that
stands — but it happens once per state, while the ground contact's own pass runs
once a frame, on message `0x1c`, with the frame's milliseconds for dt, and reads
each point at the frame's **interpolated** pose.

A fourth round of twelve was worked on **2026-09-19**, the same four areas of three.
**Eleven closed and one narrowed**, and five further queue lines went with them, all
five **stale** — the `0x25` push, the active-row handler, the mount vector, the
AI-set target and the AI turret's `0x400` were answered in the docs and never ticked
here. Four carry negatives with their controls: **nothing** reaches `Ngi32.dll`'s
`SetLight`, where the same sweep one slot over finds `LightEnable` four times;
**nothing** calls the controls manager's slot 11, against an enumeration at three
other slots of the same vtable that finds their callers; **nothing** sets a collision
entry's skip flag; and header flag `0x10000` has no reader, against the sweeps that
find bits 11, 12, 13 and 15. The engine changed in four places, the largest being a
pair of units sharing a push by mass squared rather than the mover taking it whole.

The round's own **largest result overturns a premise this file has carried since M1:
the sun does not stand still, it travels.** `CSun`'s takt lerps an angle from −0.1π to
1.1π across the body's lifetime and writes `(cos θ, 0, −sin θ)`, so the two constant
angles place an **arc** and the directions [10-sky](docs/10-sky.md) had documented are
its **top**; at its ends the sun is 15.5° and the moon 11.5° below the horizon. The
light record's `+0x24`, which this file said nothing wrote, is written twice a takt by
`CLightManager` slot 9 — the earlier round had mis-assigned that slot. Both entries are
rewritten where they stand.

Three more premises of earlier rounds are corrected in place. `Terrain.dll:0x1001f178`
is **not a call into the effect manager** and there is no stack mismatch: it belongs to
a different interface entirely, whose slot 3 takes exactly the arguments pushed, so the
"one argument fewer" this file asked about for two rounds was a phantom. `SetLight` is
at the render interface's `+0x94`, not slot 29 — the vtable base had been read eight
slots off, at `0x10031600` where the bytes are data rather than code pointers. And the
skip flag's setter and clearer were recorded as slots 7 and 6, counted from the wrong
vtable base; they are 4 and 3.

**Which way this round's own correction ran is worth recording.** The guns agent
published *"the vector the outer camera adds 0.75 of is the struck face's normal"*, and
fixed the engine to lift along it. The coordinator's spot-check could not follow two of
its addresses: one instruction wrote a field the chain did not name, and the plane-
equation proof stood on a record whose offsets did not match. Sent back, the agent
**repaired both links** — each was a missing base, one routine returning `this+0xd4`
and another `this+0x18`, so the two offset pairs were one record seen twice — and then
**withdrew the claim anyway**, for a reason neither it nor the coordinator had raised:
**nothing writes `+0x34`**, so the pointer whose floats make that plane equation is
itself unread, and the face's own normal is demonstrably built elsewhere in the same
block. The engine is reverted to its stand-in. That is the third round running in which
a spot-check, not the research, asked the second question — and the first in which the
answer to the coordinator's objection was *"you are right, and it does not save the
claim"*.

A fifth round of twelve was worked on **2026-09-20**, the same four areas of three.
**Ten closed, two narrowed**, and two further queue lines went with them, both **stale** —
the map and radar display and the four writers of `+0xa2` were answered in the docs and
never ticked here, and both were caught by the coordinator's own check rather than spent
on an agent. Three carry negatives with their controls: the effect view test's interval
float `0x10026a7c` has **two references in its module and both are reads**, against a
generator state three instructions away that the same sweep finds written twice; a
particle's **place jitter is `(0,0,0)` on all 923** effects, against the time jitter in the
same header non-zero on 56 of its 58; and **no shipped object answers world class 2** under
three separate sweeps, though a class computed at runtime would still escape them. The
engine changed in seven places, the largest being that **every celestial body was drawn at
43 % of its size** — camera slot 27 is pixels per radian, so a body's half-width is
`extent × 0.1625` radians and not the guessed 8° at extent 1 — and that **clan attitudes are
modelled for the first time**.

**This round corrects two premises of the fourth round, and they run in opposite
directions.** The sky's pass descriptor at `0x100a3828` was published as *read by nothing*;
it is read, from a live duplicate at `0x100a1d68` that `CShade` hands both prim buffers, and
the copy that round found is the dead one. That negative failed for a reason worth keeping:
**its control was the wrong kind of object** — `0x100a3800` is the quad's vertex format, not
a camera descriptor — and a control that is not the same kind of thing as its subject proves
nothing about it. Running the other way, the face record's `+0x34`, whose claim the fourth
round published, defended, and then **withdrew** on the ground that nothing writes it, is
**reinstated**: the landscape's constructor writes it, in the most-derived object's
coordinates rather than the interface's, which is why the sweep for it came back empty. Both
entries are rewritten where they stand, and the coordinate-space trap is now a bullet in
[09-method](docs/09-method.md)'s *Searches that do not discriminate*.

**Which way this round's corrections ran is worth recording**, because it breaks a run of
three. The last three rounds each turned on a coordinator spot-check asking the second
question; this time the **agents' own research** overturned both premises, and the
coordinator's checks confirmed rather than broke them — the pass tables re-read off a
continuous decode of `0x10040b10`, the `+0x34` writer off one decode from the constructor's
entry to its first `ret`, the attitude constant off `ai.dll:0x10001fe0`, and the generator's
cycle of **1 065 353 089** recomputed from its recurrence rather than taken. Two smaller
corrections came out of the round: [29-weapons](docs/29-weapons.md) had a round passing a
lake by its excluded world flags `0x208` when it is the excluded **class** `0x24` that
carries the water (the flags fall on 0 of the 275882 faces, the class on 3630), fixed here;
and this page's own "the 33 missions place 864 objects" mis-attributes a count — the install
has **33 maps but 29 missions**, and it is the 29 that place them.

A sixth round of twelve was worked on **2026-09-20**, four areas of three: the font, the AI
and clan brain, files and formats, and motion and controls. **Eleven closed and one is partly
answered**, and two further queue lines went with them, both **stale** and both caught by the
coordinator's check rather than spent on an agent — the retaliation messages, read out in
[31-packages](docs/31-packages.md), and `TRF1`'s directory flag, already narrowed in
[16-research](docs/16-research.md) to a near-settled negative. The whole of **milestone M5's
text** closed: four stand-in rows became one, and the engine's text was wrong in three separate
ways at once — it mapped every character through CP866 and drew `?` for the rest, it moved the pen
by the advance alone so glyphs overlapped by a pixel, and it keyed the atlas on black with alpha
blending instead of alpha-testing it and modulating.

Five results are **negatives carrying their controls**, and two of them are doubles. Nothing reads
the fire control's 0.5 and **neither fire-mode lock can ever be set** — `0x10023fd0` has no caller
anywhere, and the three hits a byte sweep does return are **image-base collisions**, every module
being based at `0x10000000`, one of them a string that genuinely lives at that address in another
DLL. Nothing reads class 3's value 0, the camera's values 3–5 or the section-5 record's int 8,
each against a control that finds the neighbouring ids' readers in the same sweep. The texture
**exporter is not identifiable**: the one writer of a `Texm` header the install contains is the
engine's own, which zeroes the two fields in question. And **nothing in the install opens
`gamefont.rlb` or `sprites.lib`**, against the control that every one of the other twenty
archives' names occurs somewhere — they are the software renderer's 2D half.

**The engine changed in four places**, the largest being the font. A hall-way vertex now gates a
unit by its size class, which measured out at 165 vertices open to any size, 7 to the building's
own and **884 to size class 2 or less, all 21 control pods among them**; an object's strength is
now priced by the read formula `(guns + 0.8) × hit points × 1e-5` rather than counted; and the
walker's path search filters shut and flyer-only links. Notably the hall-way fix **changes nothing
about the shipped game** — the engine feeds only bridges' hall ways into the search and all 37 of
their vertices are open to any size — which is itself the result that the stand-in was right *for
what the engine models*.

**Three premises are corrected.** [21-briefing](docs/21-briefing.md) had the byte the briefing
tests before running as *a saved game being loaded*; it is **mode 3, the attract-mode demo**, which
plays its mission with no briefing — and game mode 4, a long-standing *guess*, is read as **a
mission of campaign 0, the training campaign**, from five branchless instructions. An earlier
round's **recording measurement of the pen advance was miscalibrated**, having assumed a 640 × 480
frame where the recording is 1024 × 768; refitting at the right size puts every run within 0.56 px
with `advance + 1`, where the advance alone is out by up to 4 px. And [02-texm](docs/02-texm.md)
said *"no texture sets either bit"* when the measurement behind it had been taken over
`Textures.lib` alone: **all eleven font atlases set the alpha-surface bit**, and that colour key is
exactly how a glyph's background is cut out. The scope was the error, not the count — a failure
mode worth naming beside the "control of the wrong kind" the round before.

Two smaller corrections: the toolkit claimed a hall-way link's eight tail words are `0xFFFFFFFF`
throughout the shipped data, and **62 of the 1096 links** carry one that is not; and the 65
textures whose payload exceeds their mip pyramid are **not truncated tails**, the leftover being
the `Page` chunk to the byte. **Which way this round's corrections ran**: all four agents found
their own, and the coordinator's spot-checks confirmed rather than broke them — the strength
constants, the 1251→866 table round-tripped through Python's own codecs, the font metrics over
every archived font, the mip box filter over all 81 marked textures, the hall-way counts, and the
mode-4 arithmetic all re-derived independently. One agent reported an error of its own unprompted:
its first sweep missed a `dCurrentProblem` setter by skipping `ebp` operands, and the doc records
that rather than quietly fixing it. Finally, **the merged tree failed `ruff` where all four
branches had passed on their own** — two lint errors that exist only in combination — which is
what verifying the merge is for.

One methodological note, since it has now cost two agents a wrong address: **a
disassembly started at a guessed boundary decodes garbage**, and garbage that looks
like plausible instructions. Both slips this round came from listings that did not
start at a function entry; both dissolved on a continuous decode. The controls agent,
warned mid-round, re-decoded all 78 addresses in its write-ups and found one of its own
bookkeeping errors.

A seventh round, on **2026-09-21**, was an **audit rather than a batch**: the whole of the
**Mission 02** section, 27 open lines in four areas — the factory screen, the designer,
boarding and the medusas, and walking into the Large Factory — each checked first for an
answer already on record before anything was researched. **Two were stale outright**: the
capture and the factory screen on one frame, read in [27-ownership](docs/27-ownership.md)
the day before the line was queued, and what commander pages 1–4 and 6–8 show, which
[41-commander](docs/41-commander.md) had. **Seven more were stale in part** — `regener`,
the recent projects' count, the medusas' attack circle, the ray walker's batch word, what
joins a hall way's groups, the bot's escape before 236 s and a detached hero's collision
manager. With research on top, **nineteen closed and eight narrowed or are partly
answered**, and Mission 04's *which unit holds no mind* closed with them: every placed
robot takes a mind as it is made and a unit the hero's Enter takes holds none, so it is
the HQ.

Five results are **negatives carrying their controls**. The pause byte `+0xe8` has two
writers and neither is on the factory screen's or the designer's path, against the scan
finding the setter's own store; `Movement_FlyHeight`'s only reader is **called by
nothing**, against all 7 calls of the ground routine beside it; batch flag `0x200` is on
**none of the 15153** batches where 8 is on 633; **no module** sets `SPECULARENABLE`,
against state 28 at 15 sites; and no receiver of `0x3f1` or `0x3f2` is found, against the
four senders. **The engine changed in some twenty places**, most of them where an earlier
stand-in had been chosen to match a recording: boarding reads the turret's body, node 1,
not its 1-hit-point socket; the hero leaves a bot facing the read (F.x, −F.y) rather than
the bot; the medusas graze their pasture; a building's screen draws the ARROW cursor and
its production bar; the designer prints `%6.1f` and its real `Epower`, keeps a row per
tab and lights its previews in the model's own frame; doors, portals and passable faces
are found by the batch word and a door is reached by its capsule; a walker's route drops
flyer-only links; and a door's sound plays at its node's origin, as the game plays it —
30.8 m from the factories' side doors, and so all but inaudible there.

**This round corrects six premises, and most of the corrections came from the triage, not
the research.** The largest is this file's own: the README's *Read since* table held **61
stand-ins**, M6 to M15, appended to the wrong table since 2026-09-14, and the rule above
kept most of them out of the queue — 19 of a sample of 24 have no line here. They are
refiled (`78e745f`), and one row among them was stale, `fDifficulty`'s, which `72c72aa`
had replaced without removing. [27-ownership](docs/27-ownership.md) had a captured unit
taking a mind "the way any placed or captured bot takes one"; it takes none.
[16-research](docs/16-research.md) had the robot `Type`s' roles shuffled and
[22-settings](docs/22-settings.md) called `ROBOT_TRANSPORT` the hero, both corrected
against `varset.var` — the second is why that unit's default order is 6,
`ORDER_ROBOT_TRANSPORT`. The designer's *"driven unit's property flag `0x8`"*, in this
file and in [37-designer](docs/37-designer.md), is the game camera's flag word, and its
bit 8 pauses nothing: it draws no world behind the designer. And two lines stood on
recordings that **did not discriminate**: the hero's leaving heading, whose recorded exit
is from the one place where facing the bot and the read agree, and the Large Factory's
front door, where the engine's west-door way was right for a walker but its search had
ignored the gates that make it so. The medusas' *not modelled* is now *modelled, and
stuck*: the walk asks 13 m/s of a hover whose moving states stop at 10.

**Which way this round's checks ran**: the coordinator's spot-checks confirmed rather than
broke — the uncalled reader re-swept for a pointer in every shipped DLL, the facing's
elements 1, 5 and 9 and the unchanged sixteen-word copy re-decoded, the 633 of 15153
recounted by material, `"%6.1f"`'s one reference and the `+0x30` store re-read. What the
coordinator found instead came from checking the queue itself against the README and the
docs, which is where the stale table was. And again **the merged tree failed `ruff`**: two
long lines came from the one branch whose checks left the linter out, and an unused
variable had stood on `main` since `619e948`, before the round began.

An eighth round, on **2026-09-22**, took **the nine lines the audit left open in Mission 02**,
in four areas — flyers and the ground, the designer, who sends and who receives, and how
`Terrain.dll` lays a building down and draws it. **Seven closed and two narrowed**, and four
new lines hold what the answers left. It began with a bug the user met in play: the warbot
the Large Factory built escaped onto the factory's roof and would not answer Follow me,
because the walk planner took a unit on the roof for one inside and sent it every second to
a hall-way vertex 32 m below (`0715e11`, pinned by a test in the user's own words). The
round then found why it was on the roof at all: the engine's flyer heights were a stand-in,
and the read ones climb it clear. Three of the four agents stalled on the stream watchdog
mid-change and were resumed from their worktrees with nothing lost.

Five results are **negatives carrying their controls**: no unit in play loses a part from
its list — five senders of the removal, all the designer's, against the 7 part loads and 21
sends of 6/7 the same sweep finds; no file in the install names a `diff_*` profile, against
`prof_war` and `chas_fly` found by the same byte search; nothing opens the `fr_` and `a_`
designers, against the holder's constructor found once; the camera flag's bit 1 has one
reader, against the other bits' tests; and flags `0x8000` are on 0 of 275,882 landscape
faces, against `0x2000` on 6,102. **The engine changed in eleven places, every one where it
had been wrong**: a boarded hero is reported to the routes from the bot less its node
sphere in y, and put out once the bot's turret body is gone; the designer's preview turns
clockwise about its own centre, the designer takes no input once the mission is decided,
and save and load are built — saving to the player's own folder, never the install, at the
user's asking (`03e9eb3`, and `parkan_formats::userdir` for all such files, `6f57c55`); the
basement is the read constrained Delaunay triangulation; portal quads fade by distance, and
the recording's green signs to the pod now show; a flyer's walk points stand 15 over the
ground and 100 over a building's top; the Wizard steers by its read heading curve; the
slope brake passes over a building's floors, so the hero climbs Mission 04's chamber; and
the repair thresholds are the compiled 0.5 and 1.

**This round corrects five premises.** [39-boarding](docs/39-boarding.md) had a boarded
hero *not ticked* and its position *frozen*; the game frame writes the bot's matrix into it
every frame and its behaviour reports on. [24-motion](docs/24-motion.md) had `PortalNearDist`
and `PortalFarDist` *read by nothing*; they are settings entries the portal fade reads. The
engine's M14 repair row stood on *which difficulty profile a unit holds*, with
`diff_strong`'s 0.8 and 0.9; no unit holds one. [37-designer](docs/37-designer.md) had the
load list skip names *beginning* with the designer's own prefixes; the test is `strstr`. And
the M11 line's own framing — a flyer *pushed onto the roof* by the floors — was the old
height stand-in's doing as much as the floors'. The coordinator added one carry-over the
agents did not: the unit record's `+0x94`, open since M5 in
[25-sensors](docs/25-sensors.md), had been read in docs/39 all along.

**Which way this round's checks ran**: every coordinator spot-check confirmed — the
component test's sense and the matrix's element 7 less `+0x94`, the designer's state-4
gate, the portal settings' writes and reads, the 15 and the 100 in `Behavior.dll`, the
brake gate's class bit and the batch record's first dword. The merged tree passed every
check at once, `ruff` included: 387 tests, 981 of 981 checks, and 139 install tests.

A ninth round, on **2026-09-22**, took **twelve of Mission 03's thirty open lines**, all
[M12] stand-ins or the recording's, in four areas — the construction sphere and the site
test, command mode and the commander, the pick and what the cockpit marks and names, and
the economy. **All twelve closed**; six new lines hold what the answers left, among them a
consequence of the site test's own fix: Mission 04 has no Small Research Center site at all,
which no play depends on because the mission captures its centre — **corrected by the
eleventh round**: the 20 m grid was too coarse, and a 5 m grid at 64 turns finds twelve. A
thirteenth line, the bunker roof's `PLACE`, narrowed on the pick's read — *inferred* to be
the ray passing outside 0.7 of the bunker's radius, which **the eleventh round overturned**:
the ray passed well inside it, and the eye stood inside the bunker's sphere.

Four results are **negatives carrying their controls**: nothing in `Control.dll` calls an
effect's slot `0x1c`, against 22 calls at 13 slots the same sweep finds through the
controller's manager; no placed object carries logical id 0, which is what lets the takt's
sweep free a reservation, against 401 carrying −1; property `0x208` is none of the 15
names the 864 objects carry, so it is a word, not data; and on Mission 03 the site test's
path refuses no storage site the exit test does not, against the posing putting Tut_2's
exit 67 where docs/24 has it. **The engine was wrong on all twelve and changed on all
twelve**: the pick, the markers and the building names; the site test, the sphere's codes,
frame and kill, and its effects' time; Esc's order, telepresence's levels and the Upgrade
row; a build's mind, a building's batteries and the timers' random sources, and the ore a
place moves by itself.

**This round corrects six premises.** [42-selection](docs/42-selection.md) and the engine
had the pick's shares the wrong way round — it is a *building* taken at 0.7 of its radius
and a unit at all of it — and the nearest centre is nearest the eye, not along the ray. The
engine held a factory build's mind for the whole build; it lasts until the clan's next takt,
which is the recording's return to 4 at 221.5 s. [23-economy](docs/23-economy.md)'s
building batteries, the root's 19.5 to 20 held and 50 to 52 a second, are on 53 of 76
assemblies the fitted `i_pws_f_*` parts' 16 to 32 and 1,000 to 2,000 — resting on an
*inferred* step, which the engine now marks and a new line holds. The engine named buildings
by the size letter of their path, wrong on 31 of 167. [31-packages](docs/31-packages.md)
had Esc leave the wingman menu open; it closes it (`0x10070f81`, 740), and the handler six
docs called the *character* handler takes key-downs. And [13-control](docs/13-control.md)'s
action 21 killed *every unit*; it takes classes 2, 4 and 10, trees and stones among them,
and spares the invulnerable. One derivation was **confirmed rather than corrected**: the
Upgrade row's demand that the next level be researched whole, derived from the recording, is
now read.

**Which way this round's checks ran**: every coordinator spot-check confirmed — the pick's
0.7 and 1.0 on classes 3 and 4, the kill mask's `1 << k` table, the Upgrade row's five
refused Types and `BUILDING_MAINTELEPORT`, `Behavior.dll`'s `rand()` constants and the Esc
case's 740. One labelling gap was the coordinator's to close: the economy branch dropped the
batteries stand-in with the root-only one while its basis is inferred, and it is restored in
`47bd99e` with the cross-doc corrections. Three branches conflicted only in the engine's
stand-in table, each resolved by keeping the newer row. The merged tree passed every check:
387 tests, 986 of 986 checks, 490 engine tests and 148 install tests, `ruff` after one
renamed loop variable.

A tenth round, on **2026-09-28**, took **twelve more of Mission 03's lines**, seven of them
[M12] stand-ins and an eighth the engine's own, in four areas — the pick, the construction
sphere and the site, a building's parts and guns, and command mode's panel. **All twelve
closed**, and two lines went with them, both **stale**: what an unfinished building shows
before the dome (`85c6f00`) and the held `,` or `.` (`de98f00`), each answered on
2026-09-22 after the ninth round's write-up and never ticked. A third batch line was stale in
the same way: [29-weapons](docs/29-weapons.md#how-the-ai-fires--read) had read a building's
fight-module bar on 2026-09-19, and only the engine's stand-in and this file still asked it.
Six new lines hold what the answers left.

Two results are **negatives carrying their controls**: the attack-target mode and Capture
building are never opened — no 2 among the ten stores to a pending kind, and no command 4 or
5 among the HQ table's 22 rows, against the executor's own stores of 1, 5, 4 and 3; and no
object of the shipped game answers class 2 — none of the 864 placed objects is a `WPNS`
agent and the five `WPNS` records name `weapon.rlb` members that are not there, against the
same byte search finding the `BULL` record `bld_l_01`. **The engine was wrong on eight and
changed on eight**: the cursor's ray, the hero's pick and Guard's radius; the sphere's
timing, its hiding and showing of the building, and its kill; a slow unit's fire bar; the
auto-driver level, the panel's icons and the commander's map. It was **right on four** — a
building's parts, the `+0xe4` rule, when a built mine is counted, and a fixed seed as a fair
model of one no two runs share — and those stand-ins became notes.

**This round corrects seven premises.** The queue's own fire-bar line had the Small Bunker's
flamers never firing at a hovering flyer; the flame's frame flag 8 scores 1 before the
height term, and the bunker does fire at one — in the engine too since before this round, so
the line described an M12 engine that had moved on. The ninth round's "65 other callers" of
`Behavior.dll`'s `rand()` are **111 sites in 41 functions**. The ninth round's sphere entry,
and the engine, called actions 1 and 2 properties `0x200` and `0x201`; they hide and show
the building's node tree, which is also what keeps an unfinished building out of the draw.
[30-turrets](docs/30-turrets.md#what-the-outer-cameras-line-meets--read)'s two-round
negative on class 2 was **the wrong kind of search**: the class is stored from a tag, which
neither a comparison sweep nor a return-stub sweep can see, and
[09-method](docs/09-method.md#searches-that-do-not-discriminate-so-nobody-repeats-them) now
says so. [42-selection](docs/42-selection.md) called the game's `+0xe5` a pause; it is the
auto-demo byte, mode 3, written beside `+0xe4`'s mode 2 (`0x1005c75a`), and docs/42 and the
engine had the cursor's ray stop on units. [11-effects](docs/11-effects.md) wrote an
effect's view-test mask `0x40a` in three places while listing the four classes it ORs; the
entries give `0x41a`. And [31-packages](docs/31-packages.md) said the Route row does not
chain waypoints and nothing fills a unit's point list, stale since docs/42 read the Route
pick on 2026-09-15. Two were **confirmed rather than corrected**: the ninth round's
*inferred* attach of a building's parts is read, and docs/34's *derived* network game is
read.

**Which way this round's checks ran**: the coordinator's spot-checks confirmed the pick's
`[0xa, 0 × 7]` and its slot-7 call, `ngiGetClocks` behind ordinal 52 and `Control.dll`'s
thunk to it, the icon tints' two jump tables and the `+0x30` store from property `0x201`,
the `WPNS` → 2 store beside `BULL` → 9, and `+0xe4` as mode == 2. **One ran the other way**:
the selection branch reported docs/11's `0x40a` as proof that an effect's ray passes units,
reading the hex rather than its entries; `Effect.dll:0x10007f14`–`0x10007f47` ORs to
`0x41a`, which admits them, so the correction went to the hex (`0c04b53`), not the prose.
One branch conflicted, in the engine's stand-in table alone, resolved by keeping its new row
and dropping the two rows answered. The merged tree passed every check: 387 tests, 999 of 999
checks, 498 engine tests and 190 install tests, `ruff` after three wrapped lines.

An eleventh round, on **2026-09-29**, took **the rest of Mission 03's lines**, seventeen,
in four areas of four — command mode and telepresence, the commander's panel and the Build
row's site, selection, and Mission 03's patrol, hints and doors. One was **stale**: the
designer's save field and load list, read and built on 2026-09-22 (`6bed1b9`, `03e9eb3`)
and never ticked. **Fifteen closed and one narrowed** — the joined byte, network play
only, where nothing but the network rule reads the byte and what is left is why the game
spares a joined clan. Five new lines hold what the answers left: the game menu's save page
and shell, a *Tiny Tower*'s −1 battery, what still reaches a building going up, the HQ
camera's ride under a patrol, and a debug key.

Four results are **negatives carrying their controls**: no double click reaches the game —
its one window class lacks `CS_DBLCLKS` and every dispatcher sends `0x203` and `0x206` to
its default, against the same dispatchers routing `0x201`, `0x202`, `0x204` and `0x205`;
nothing but the network rule reads the joined byte, against the sweeps finding its writer
and that reader; a building's sphere reads no node flag, against the sweep finding the
draw's two tests; and nothing on the way into a bunker's command view sends the bunker the
Wizard's pair, against the four entries into a tower's *Manual* that do. The hero never
stood in routes 4 or 5, measured off the radar's two still marks, which agree within 6 m.
**The engine changed on nine lines**: the command-mode pick's sphere; a lone go's patrol;
the Guard pick a band leaves open; a flying follower's height, beside a patrol's that was
right; the map's line; the *Tiny Tower* icons and a clan's unit count; the view after the
hero dies, and the press and the edge turns under a band; a level-0 hit; and the game menu
and tooltips, now built. It was **right on six** — the route reports, the shot door,
property `0x200`, the site test, what a hidden building is left out of, and a bunker's
guns — and in part on three more: the patrol's height, the band and double clicks, and the
Guard's place. The go task's 30 m across the ground, a stand-in no branch owned, is now
read.

**This round corrects seven premises.** The ninth round *inferred* that the bunker's roof
showed `PLACE` because the ray passed outside 0.7 of its radius; it passed 11.6 from the
centre, well inside, and **the eye stood inside the bunker's sphere** — which the engine
had as the box over its node spheres, 87.23 against the agent's own 52.58. The ninth
round's Mission 04 had **no** Small Research Center site; it has **twelve**, on a grid the
first count was too coarse to meet. The tenth round *derived* that a Guard pick could
inherit another pick's points; the executor's tail **empties the list** before every
pick. This file's patrol line still carried the chassis's authored speed, which
[31-packages](docs/31-packages.md) had already replaced, and docs/34 with it; the 85 s is
84. [24-motion](docs/24-motion.md) put the hero at the bunker ramp's top at 164.0 s,
where it stood at the exit, and said the recording could not tell the shot from the
approach; it can, and the hero shot both doors open before reaching them.
[13-control](docs/13-control.md) gave argument 1 to six of the mode handlers' sends
where four carry it, and its table of 21 sites listed 14. And this file had a building
going up kept "out of everything"; the pick still takes it, as kind 2, in the engine and
the game alike. One was **confirmed rather than corrected**: docs/40's *guess* that the
pair a bunker is sent as it is left hands its guns to the AI — they were never taken from
it, and the pair says so again.

**Which way this round's checks ran**: the coordinator's spot-checks confirmed the lone
go's `PATROL` of `0x96` at `0x1002b8a8`, property `0x200`'s `+4` at `0x1002bc1f`, the
capacity loop's early answer at `0x1002b519`, the install's `FORCE_SOFTWARE_CURSOR=1` and
the setter's 7 and 8, and the go task's length helper as the root of x² + y² alone, which
closed a stand-in no branch owned. **One ran further than the branch**: the command-mode
branch's correction to docs/13 was right, and the coordinator's byte scan found the table
three sites short of its own 21 besides. One branch conflicted, in the engine's stand-in
table alone, resolved by keeping both sides' rows. The merged tree passed every check: 387
tests, 1015 of 1015 checks, 509 engine tests and 201 install tests, `ruff` clean.

## Sky and rendering

[10-sky](docs/10-sky.md#not-resolved), [02-texm](docs/02-texm.md),
[07-objects](docs/07-objects.md), [06](docs/06-open-questions.md)

- [x] ~~[M1] Where the sun object's two lights point: nothing writes the light record's `+0x24`.~~ —
  closed 2026-09-19, and it **overturned a premise of this page**: the sun *travels*. `+0x24` has a
  writer after all — `CLightManager` slot 9 (`Terrain.dll:0x10080550`), a `SetDirection(id, space, x,
  y, z)` whose space-0 arm writes the record's `+0x24` from its three arguments (`0x100805a8`) and
  whose space-2 arm first turns them through the manager's world placement. `CSun` calls it **twice
  every takt** from its object slot 3 (`0x1007ed40`): the first light takes the vector at `this+0x78`
  (`0x1007ee2a`), the second takes it **negated** (`0x1007eea8`) — the counter light the never-read
  `ContrLightOn` is named for. *Measured*: of the **72** sites in `Terrain.dll` that stride a light
  record by `0x5c`, exactly **three** write `+0x24` — slot 9's two branches and the manager's
  constructor at `0x1007fde1`, whose default is (1, 0, 0); the control is the same sweep, which finds
  the colour at `+8`–`+0x14` on 2 sites and the position at `+0x38`–`+0x40` on 1. The earlier round
  had mis-assigned slot 9, which is why it read as "no writer found"
  ([10-sky](docs/10-sky.md)).
- [x] ~~What `CSun` does with a body's lifetime, for a body that started before the clock's start.~~
  — closed 2026-09-19 as a **side effect** of the line above: the lifetime is the **span of the
  crossing**. Slot 3 takes the fraction of it that has run — `(now − this+0x28) ÷ this+0x2c`, clamped
  to 1 at `0x1007ed4c` — lerps θ from **−0.1π to 1.1π** (the constants 1.1 at `0x1009bb48` and −0.1 at
  `0x1009c1ac`, × π at `0x1009c084`, all four re-read by the coordinator), writes `(cos θ, 0, −sin θ)`
  to `this+0x78` and turns it through the `Rz(A)·Rx(B)` matrix at `this+0x38`. So the two constant
  angles place an **arc**, not a body: the directions this page had documented are its **top**, and its
  ends put the sun 15.5° and the moon 11.5° below the horizon. Mission 01's sun crosses for 525 s of
  its 900-second day, its moon for 300. The lens flare's second gate reads better for it too — the
  negated height peaks at `cos B`, so the sun *reaches* the top of its ramp at zenith instead of
  sitting there always ([10-sky](docs/10-sky.md)).
- [x] ~~[M1] Which pass descriptor the sky's draw is given~~ — closed 2026-09-20, and it **overturns a
  premise this page published on 2026-09-19**. That round found the descriptor **near 700, far 50000, z
  range 1.0 to 1.0** at `0x100a3828` and reported that *nothing reads it*. It is read: the live copy is
  the 14-entry array at **`0x100a1d68`**, and the run at `0x100a3828` is a **dead duplicate**, written
  field by field by absolute address (`0x1007c450`), which is why no reference to its base exists.
  Nothing calls `SetPasses` by name because it is **slot 0 of `CPrimBuffer`** (vtable `0x1009ae18`,
  installed `0x10032a25`): `CShade`'s render setup (`0x10041370`) makes two prim buffers and calls it on
  each with `(4, 0x100a6060, 14, 0x100a1d68)` (`0x1004217a`, `0x100421e0`). Both tables are zero on disk
  and filled at load from the floats at `0x1009b0ec`–`0x1009b120`. **Re-read by the coordinator** off a
  continuous decode of `0x10040b10`: entry 0 takes near from `[0x1009b104]` = 700, far from
  `[0x1009b108]` = 50000 and both z from `[0x1009b11c]`/`[0x1009b120]` = 1.0, and the sky files **group
  1, layer 0** on all four draws. The second named handle is a clean negative: the item-pool reset
  (`0x100332b0`–`0x100336ea`) holds **no `call` at all** and only zeroes item `+4` and the pool counts,
  so it does nothing to the fog — control, its sibling `CPrimBuffer::Draw` (`0x10032c60`) has six calls
  in its first 130 instructions. **Why the negative failed is the lesson**: its control, `0x100a3800`,
  is not a camera descriptor at all but the **quad's vertex format**, bound for the draw item's `+0x194`
  (`0x100286ab`). A control has to be the same kind of thing as its subject
  ([10-sky](docs/10-sky.md#the-dome)).
- [x] ~~[M1] Whether `ForceSWFog` does anything outside `Terrain.dll`~~ — closed 2026-09-18: **no
  module reads it**, and it is entry 0 of `Terrain.dll`'s 36-setting page. A setting is read as
  `[index * 4 + 0x100a6cac]`, an address the loader relocates, so the readers are countable rather than
  searchable: 41 relocation sites name the page, all 41 carry a constant index, none is 0, and between
  them they cover 32 of the 36 — entries 1 `LightingOn` and 2 `SpecularsOn` either side of it are the
  control. Across modules the page is interface `0x1e` of `World3D.dll`'s settings registry, keyed
  `(index << 16) | id`, and the game makes exactly one cross-module settings read in its life:
  `RobotBestLOD`, from `AniMesh.dll`. `Ngi32.dll`, which would have to implement a software fog, does
  not import the registry at all. The engine's per-fragment fog stands ([10-sky](docs/10-sky.md)).
- [x] ~~[M1] The fog heading's world axis: that the camera matrix's first column is the view
  direction is a guess.~~ — closed 2026-09-19: **it is the view direction, and the engine was already
  right.** Camera slot 28 (`0x100850f0`) asks the camera's attached `CGameObject` for placement type 2
  (`0x1008ae50`, the world matrix at the object's `+0x60`) and takes `m[0]`, `m[4]`, `m[8]`; the same
  call with the same type is what reaches the renderer, through the view's placement getter
  (`0x10082f60`) and a 100-byte camera block (`0x10081a58`) the device copies verbatim
  (`Ngi32.dll:0x100073a5`) before `0x10009450` builds the Direct3D view — whose **depth is `(m[0],
  m[4], m[8])·(p − E)`**, x is `−(m[1], m[5], m[9])` and y is `(m[2], m[6], m[10])`, with `E = (m[3],
  m[7], m[11])`. So the columns are forward, left, up and the eye, and the bottom row is (0, 0, 0, 1).
  The second, independent witness is `Control.dll` building a unit's first-person frame with columns
  look/side/up/eye and handing it out as the same placement type 2 ([10-sky](docs/10-sky.md)).
- [x] ~~[M1] Whether water draws see-through~~ — closed 2026-09-18: **opaque**, and the engine was
  already right. A mesh batch takes its blend mode from its material's flags byte through `CShade`'s
  translate table, but the ground does not: the ground surface takes `CShade+0xbf4`, which its init
  fills from translate index 0 — mode 0, `ONE`/`ZERO`, no blend and no alpha test — and the
  `REFLECTION_SHIFTED` water surface writes mode 0 outright. Measured over all 275882 faces of the 33
  maps, `WATER` (1006 faces on 7 maps), `WATER_BOT` (2015) and every `L*` ground material carry flags
  byte 0. The control is the 2624 `ENV_NLAVA` faces, whose flags byte 4 *does* ask for
  `SRCALPHA`/`INVSRCALPHA`: they draw opaque anyway, because the ground draw never looks
  ([03-terrain](docs/03-terrain.md)).
- [x] ~~[M2] Whether a blended material writes depth, and the alpha test's reference value~~ — this
  line was **stale**: an earlier round answered it (`27172f2`), a blended batch drops only alpha 0 and
  writes depth, and the engine implements it. Re-read independently 2026-09-18 and it holds:
  `ALPHAREF` is 1 and `ALPHAFUNC` is `GREATEREQUAL`, written by two agreeing writers, and the six
  blend-mode records are 30 `{state, value}` pairs over five states, among which neither `ALPHAREF`
  nor `ZWRITEENABLE` appears ([07-objects](docs/07-objects.md)).
- [x] ~~[M3] What draw layers 10 and 9, which a fifth slot is filed under, do
  (`Terrain.dll:0x1004553b`); and `CShade` slot 15.~~ — closed 2026-09-20: **a layer is the index of the
  pass inside its group**, so a layer is a frustum and a depth range, not a sort key. The mesh draw picks
  both together (`0x1004552a`–`0x100455bc`): an ordinary opaque surface gets group 0 layer 0 (near 0.5,
  far 700, z 0.1–0.99), a see-through one group 1 layer 5 (the same frustum), and **a fifth slot group 1
  layers 9 and 10 — near 0.05, far 10, viewport z 0.0 to 0.1**. So the depth buffer is cut in three: the
  first-person geometry owns the front tenth, the world 0.1–0.99 and the sky 1.0, and a cockpit closer
  than the world's own near plane is neither clipped by it nor ever occluded. *Re-read by the
  coordinator* at `0x100a1e40` and `0x100a1e58`. **`CShade` slot 15 is `0x100437a0`** (vtable
  `0x1009b17c`, installed `0x10041f94` — and `0x1009b13c` one table over is the *item manager's*, the
  trap that has cost this file two slot numbers): it writes one byte to `CShade+0xca8`, which the mesh
  draw reads at `0x100455fe` and uses to set the item's flag bit 0 **only when it is zero**, so
  AniMesh's "slot 15 with 1" clears that bit on the fifth-slot draws. What bit 0 then does is **not
  established** — the renderer masks it off at `0x1003056a` and no test was found, against the control
  that bits 3, 4 and `0x400` all turn up at once. The engine still draws the fifth slots as ordinary
  scene instances; the row moves to the README's *Read since the stand-in was written*
  ([07-objects](docs/07-objects.md)).
- [x] ~~The sun sprite's extent unit, camera slot 27~~ — closed 2026-09-20, **and the engine was wrong
  by a factor of 2.3**. `CCamera` slot 27 (`0x100851c0`, vtable `0x1009c620`) is the viewport's **width
  in pixels over the view's field of view in radians**: *re-read by the coordinator*, it fetches the view
  at `+0x19c`, asks slot 15 for the viewport rectangle, takes `right − left`, `fild`s it and `fdivr`s by
  slot 17's answer (the float at view `+0x234`, which `Ngi32` halves into a cotangent projection at
  `0x10007055`; the game sets 1.7 rad at `0x1001ffd7`). Pixels per radian — and since the sprite's
  corners are screen pixels at z 0.999, the width and the field of view cancel, leaving a body's
  half-width at **`extent × 0.1625` radians**. Extent 1 is **18.62° across** and the shipped 0.4–3.3 span
  **7.45° to 61.4°**, against the engine's guessed 8°, so every body was being drawn at 43 % of its size.
  Fixed and pinned by two tests, one re-deriving the constant from the pixel rule at three screen and
  field-of-view pairs; a new `verify` check measures across ≥ up on **656 of 656** keyframes
  ([10-sky](docs/10-sky.md#the-sun-and-the-moon-are-drawn)).
- [x] ~~Texture header bit `0x4000000`: which batch of exports it marks~~ — **partly answered**
  2026-09-20: the batch is characterised, the exporter is a negative with its control. The run is
  members **66–154 of 393** and it is coherent — **all 89 are ARGB8888, all 89 mip-mapped** (`+0x10` =
  32, chains of 4–7), all 89 carry `+0x18` = 0, and their names are machine and building skins (19
  `S*`, 14 `RL_*`, 13 `PG*`, 7 `GEN_*`…), which is why 78 of the 81 are worn by a lit material.
  **Nothing but the bit tells the run's 89 apart**: the eight unmarked ones match on format, mip flag,
  filter, `+0x18` and size, and interleave by name (`PG03/04/06/07` marked, `PG05` not). A new axis
  *does* discriminate a tool: every mip level *k* is the **truncated mean of the matching 2^k × 2^k
  block of level 0**, `sum >> 2k`, in the stored components — **81 of 81** marked pass (*re-measured
  by the coordinator*), and the control is the same test failing on a coherent block of unmarked ones,
  the `L*M.0` and `L*.0` families. It passes on 166 unmarked too, so it names the **tool, not the
  batch**. The **exporter is not identifiable**, a negative with its control: a byte sweep of all 22
  binaries for the literal `Texm` returns **one** hit, `Ngi32.dll:0x10007e7c`, which *builds* a header
  for a texture made from nothing and zeroes both `+0x14` and `+0x18` — so the search can find a writer
  of this header, and the one it finds is the engine's own; no `.tga`, `.bmp`, `.pcx`, `.psd`, `3ds` or
  `Photoshop` string occurs in any of the 22. Two corrections fall out: the 65 textures whose payload
  exceeds the pyramid are **not truncated tails** (the leftover is the `Page` chunk to the byte, and
  all 518 `Texm` members account for every payload byte), and `+0x18` is a **colour**, non-zero on 47
  of 393, on the palettised ones an index whose entry is near the image's own mean
  ([02-texm](docs/02-texm.md)).
- [x] ~~What IAnimation node mask bit `0x10` does~~ — closed 2026-09-18: it lays the node along the
  ground under it. Slot 8 mirrors the bit into the node record's byte `+0x113` — nothing tests the mask
  word against `0x10`, so the byte is the whole path — and the pose walk then turns the node's world
  matrix by the rotation slot 31 left on it and writes the translation back, so the node tilts where it
  stands. Set by the ground contact on a `CONTACT_PLACE` contact's carrier. ~~Set on twelve nodes in the
  game: the belts of the three tracked chassis.~~ **Corrected later the same day**: `CONTACT_PLACE` is
  worked out from the state's end pose as well as read from the file, so it reaches 2229 of the 2634
  contacts and every walker's feet, not twelve belts (see *Contact record flag `0x20`* below).
  Implemented ([28-chassis](docs/28-chassis.md), [24-motion](docs/24-motion.md)).
- [x] ~~Who sets an object's material track (`ILifeSystem` slot 16), and who calls IAnimation slot 27~~ —
  closed 2026-10-01, **read**, and **seen** for a building, a unit and a captured unit. The published
  "no caller of slot 16" was false.
  - **Three callers, all in `iron3d.dll`**, each on the `ILifeSystem` a record keeps at `+0x48`. The
    building record's step (`0x10033020`, at `0x10033072`) and the unit record's takt (`0x10075680`,
    at `0x10075727`) write the clan record's `+0x14`, its **sign**, of the clan the record holds at
    `+0x24`, **every game frame**; the unit record's bind (`0x10074d30`, at `0x10074da0`) writes that
    clan's index, which the takt replaces a frame later. A single-player game sets clan *i*'s sign to
    *i* (`0x100a2420`); a network game takes it from the session table (`0x100a23e1`).
  - **A capture changes it**: it rewrites the record's clan (`0x10033012` for a building; the unit
    record's slot 2, `0x100355a0`, which Enter calls at `0x10072035`), and the next frame writes the
    new sign.
  - **The draw** reads it at one place, the mesh draw's slot 15 call (`AniMesh.dll:0x10014dee`), and
    both manager fetches turn a track past the material's count into 0 (`World3D.dll:0x1000322f`,
    `0x10003709`), so only the two eight-track materials change with the clan.
  - **Slot 27** (`AniMesh.dll:0x10005970`) is camouflage: the ground contact calls it with the face
    under the machine while its detection shield's state `0x600` reads `0x1000`
    (`Control.dll:0x1001a95d`), and with an empty reference otherwise.
  - **How it was missed, and the control.** The pointer comes from `QueryInterface`, which a search
    from a stored global cannot follow. `analysis/slotcalls.py` searches from the request: 116
    `call [reg+0x40]` in the install, 10 on a pointer that may be interface `0x16`, 3 of them the
    writers. The control is slot 10, where the same search finds `MBehaviour::Capture`'s three calls
    and the bind's among 223.
  - *Measured*: 905 materials, 860 of one track, 43 of two, 2 of eight (`B_LBL_01`, `R_LBL_01`, both
    on `PG27.0`, tracks 0–7 naming cells 0, 6, 5, 4, 3, 2, 1, 7). 157 of 15153 batches draw a
    multi-track material. Placed in the 29 missions: 133 of 167 buildings and 209 of 254 robots wear
    one, 0 of 42 heroes.
  - *Seen*: Enemy 1's Medium Mine wears cell 6 and the player's Small Bunker cell 0 ("Let's Play -
    Parkan: Iron Strategy, Part 6.5", 37.4 s, 2:15); the vacant LWW-2, of the neutral clan 3, wears
    cell 4 (3:37) and the arrow once captured (Part 6, 7:30). Campaign 02's Outpost emblem (Part 3,
    1:22) is cell 6 in the emblem's own diamond quad, the enemy's sign.
  - The engine drew a building on its owner's track, a unit on track 0, and changed nothing on a
    capture: units now wear their clan's sign and a capture re-looks the target
    ([07-objects](docs/07-objects.md#who-picks-an-object-meshs-material-track--read),
    [27-ownership](docs/27-ownership.md)). Remainders, queued: a captured building's emblem is unseen,
    a network game's signs, and what camouflage wears.
- [x] ~~How the weather is drawn~~ — closed 2026-10-01, **read**, and **seen** for the dust and the
  lightning; the rain stands on the read alone.
  - **Rain and snow** are `CRain` (`Terrain.dll:0x10075d00`) and `CSnow` (`0x10075560`), each owning
    one particle system (`0x10072b40`): **up to 1000 points** kept in the camera's box, from 2 to 50
    ahead and as wide and high as the view is at 50 (`0x100759dc`), the count 1000 × intensity ×
    min(1, V ÷ V₀). Each draw moves them by the velocity — **(0.5, 0, −4) a second for snow** plus a
    flutter of 0 to 1 on every axis, **(0.5, 0, −60) for rain** — wraps what left the box in at the far
    side (`0x10073410`) and spawns or drops to the count.
  - **The draw** is pre-transformed quads, 600 at most, depth test and write off, group 1 layer 8, in
    the `0x1000` pass after the world: a snowflake a square or a diamond 0.0195 × viewport width ÷
    field of view, shrinking with depth to a tenth; a raindrop 0.0065 wide, stretched from where it
    stood a frame before. Its colour is the keyframe's slot 19 with each channel held at `0x50` and
    alpha `0x96` (`0x1006ce63`, `0x1006d120`), unfogged; its material the start's `sky.wea` slot.
  - **The one gate is the camera's `ICamera2` mode, which must be 0.** Neither draw asks the
    landscape or a building anything, so it falls in the briefing, in command mode, through the
    cockpit and indoors — which is the "black field with stars" a doorway shows (Part 6, 28:15): the
    dust over the portal's fade quad, layer 5.
  - **Lightning** draws nothing itself (`CLightning`, `0x10071990`; its slot 4 is empty). Six seconds
    after a strike it draws the next one's time, rand ÷ 32767 × (1 − intensity) × 60000 ms on with the
    intensity held at 0.95 (`0x10071920`), then plays `env_lightning` at a point drawn anywhere over
    the landscape's box, 300 above the ground, sized (40, 40, 600), mirrored one time in two
    (`0x10071d70`). A bolt strikes the ground and does no damage. `env_lightning` is three blocks over
    0.75 s: a mode-1 sprite along (0, 0, 1), a kind-7 point light (7, 7, 10) of range 100 × the
    frame's 40, and `atm_light1.wav`. The flash is that light on the vertices, through the shade's own
    lighter; neither the sky nor the scene colour flashes.
  - *Measured* over the 29 atmosphere files: the snow slot is `SNOWFLAKE` on 23 and `DUST_ADD` on 6,
    and all 6 `DUST_ADD` files snow the whole day; the rain slot is `RAIN_DROP` on all 29, of which 8
    rain; 15 of 29 start some weather.
  - *Seen*: C03 M02's dust matches the engine's in count, size, shape and colour (about 35 against
    about 40 in a 500 × 320 patch of sky; Part 5 1:55, Part 6 2:00); the ground lights at 9:09.7,
    9:17.5, 9:25.6, 9:32.1, 9:46.9 and 9:54.9 of Part 6.5, gaps of 6.1 to 8.1 s against the read's 6
    to 9, each flash bright for about 0.4 s.
  - **This corrects [10-sky](docs/10-sky.md)'s own inference**: the world's view is not mode 1. The
    scene camera is mode 0, and the weather returns at any other mode yet is seen everywhere, so the
    sky's screen-wide quad is drawn there, first of all.
  - The engine drew no weather and now draws all three as read
    ([10-sky](docs/10-sky.md#the-weather)). The engine's flash stops at the texture's own colour
    where the recording's goes on to a pale lilac-white, the stand-in of the lit colour held at 1.

## Effects and sound

[11-effects](docs/11-effects.md#not-resolved)

- [x] ~~[M4] The rest of the emitter floats~~ — closed 2026-09-18, and the guess was right: the
  triples shape **position and size**. Every drawing emitter builds one particle class (`Effect.dll`
  vtable `0x1001eb08`) holding three lerped `float32[3]` channels, of which the draw reads the first
  as the particle's position (`0x100093fc`) and the third as a per-axis scale on its billboard
  (`0x1000d0c0`); and a channel turns out to be **four consecutive triples — low, high, jitter,
  exponents**, a reading that checks itself, because each run ends exactly where the block's
  `(archive, member)` pair begins. So the sprite powers at `+64` and `+124` are the position and
  size exponents. *Measured*: **6818 of 7142 channels carry (1, 1, 1)**, 324 are bent over 208
  effects and **192 triples differ across their axes** — the shipped data does bend them (counted
  here independently). A bolt's `+24`/`+28` are its width at the two ends of its **window**, not of
  its beam (`0x10002944`, `0x10002fb4`): on all 31 bolts 26 write the same width twice and 5 halve,
  none widens, and `hero_laser_bullet`'s are a constant 0.4 and 0.1 — so the recording's broad beam
  is the additive blend, not the width. And **a fade scales alpha, not colour**: no device material
  is ever built for an effect sprite, because `CStridedPrimitive::RenderVB` gates the material path
  on draw flag `0x10` and an effect item carries 4, so it takes the unlit path; the item builder
  overrides the streams with two `D3DCOLOR`s at **stride 0**, so all four vertices carry one pair,
  which the Shader component makes at `0x1004f710` from the entry's **ambient rgb** — kneed (c ≤ 1
  kept, then c/6 + 5/6 up to 7, then 2; the constants read out here) — with the **ambient alpha**,
  where the fade is written, as its alpha. *Measured*: all 1598 entries of the 243 materials the
  effects draw carry ambient alpha 1.0, so the fade is the whole of a sprite's alpha, and none has a
  component above 1, so the knee never fires. Fixed: a burst's particle had been modelled as a
  velocity from the origin and is a lerp in place ([11-effects](docs/11-effects.md)).
- [x] ~~[M4] How the shade lights a type-1 light (falloff, attenuation) and the manager flags
  `0x80000000` and `0x20000000`~~ — closed 2026-09-18: **there is no analytic falloff at all.**
  `EmulatePointLights` (`Terrain.dll:0x1002a130`) reads six fields of the 0x5c-byte record and no
  more — type `+4`, manager flags `+0x50`, colour `+8`, position `+0x18` and range `+0x30` — cuts on
  the range **twice** (the plane distance, then any vertex), and then **projects the light onto the
  triangle's plane as a disc of radius sqrt(range² − d²)** (`0x1002a804`–`0x1002a994`), clipped and
  queued as an extra pass whose colour is normalised into the ambient rgb and whose ambient alpha is
  that length × 0.25 held at 2. **The attenuation triple `+0x38`..`+0x40` is never read**, a
  negative whose control is that the same walk does find the range and the colour the routine
  plainly uses. Both manager flags are now enumerated: `0x80000000` at `0x10047a52`, `0x10047c96`
  (a second loop skips such a light outright) and `0x100808c1`/`0x10080914`; `0x20000000` only at
  `0x1002a200`. The engine still lights with none of it, and its row now says why rather than
  promising `1 / (a₀ + a₁d + a₂d²)`: a texture on a projected disc is not an attenuation term
  ([11-effects](docs/11-effects.md)).
- [x] ~~What an effect light's attenuation triple `+0x38`..`+0x40` is for, which nothing reads, and
  whether anything reaches `Ngi32.dll`'s `SetLight`~~ — closed 2026-09-19 as a **negative with its
  control**, and a slot number is corrected with it. The render interface's vtable is installed at
  **`0x100315e0`** (`Ngi32.dll:0x10005eec`), not `0x10031600`, so `SetLight` (`0x10008b50`) sits at
  **`+0x94`** and `LightEnable` (`0x10008b20`) at `+0x90` — "slot 29" had been counted eight slots into
  the table. (Re-read by the coordinator: `+0x94` → `0x10008b50`, while the old base yields data, not
  code pointers.) Across all sixteen modules **no call at `+0x94` lands on a render interface**; the
  control is the same sweep at `+0x90`, which finds it **four** times, each a
  `for i in 0..7 { lights[i] = 0; LightEnable(i, 0) }` in `CShade`. So the shade **switches all eight
  device lights off and never sets one**, and with `EmulatePointLights` not reading the triple either
  it is **dead data**. *Measured* over all **618** light blocks: (0, 1, 0) on **447**, (0, 1, 1) on
  **170**, (0, 0, 1) on **1** (`env_lightning`) — never a constant term, and no component but 0 or 1.
  The artists wrote Direct3D's linear falloff on 617 of them for a renderer that never asks
  ([11-effects](docs/11-effects.md)).
- [x] ~~[M4] Header flag `0x800` and the rest of the draw path~~ — closed 2026-09-19, and **a premise
  of this line was false**. The manager's draw is slot 3 (`Effect.dll:0x10004050`, `ret 0x10`): four
  arguments, key −2 drawing every instance, and pass 0 making the manager return at once unless its
  flag 2 is set and the instance's draw skip a `0x800` effect. **Nothing anywhere passes 0** — the two
  callers that certainly hold a manager both push **1** (`CLandscape`'s draw at `Terrain.dll:0x1001c93e`
  and `CAtmosphere`'s at `0x10070cec`, the only two `push -2` before an indirect call in any module),
  and the one computed pass (`AniMesh.dll:0x100151ea`) is 1 while the object's sphere clears the
  camera's six planes. **`Terrain.dll:0x1001f178` is not a call into the manager and there is no stack
  mismatch**: it belongs to the `ISystemArealMap` interface `CLandscape` installs at its `+4`, whose
  slot 3 takes exactly the three arguments pushed — this file had been chasing a phantom. **Draw flag
  4** (header `0x2000`, `env_lightning` alone of the 923) is not a texture choice but a **fog
  override**, holding the sprite's factor at 1.0 (`0x1004f841`) instead of the distance curve.
  **Header `0x10000`** (`aim_tail_S` alone) has **no reader**, controlled by the same sweeps finding
  the readers of bits 11, 12, 13 and 15. ~~who sets the target point a bolt starts from~~ was
  **stale** — answered in an earlier round: the muzzle on the shooter's node 0. Still open, narrowed:
  which draw reaches a **unit's own** manager, the one an agent makes at `AniMesh.dll:0x100033ba` and a
  controller drives at its `+0x3c` ([11-effects](docs/11-effects.md)).
- [x] ~~The four settings groups, the group floats `+0x1084` and `+0x1294`, and the page's
  `+0x14a4`.~~ — closed 2026-09-19: a group is the switch id's **high byte**, and the four are **detail
  classes**, not categories, since the switch names repeat across them and only these numbers differ.
  The two floats are the page's own named settings — the getter answers `0x?f0` with
  `page + group×4 + 0x1084` and `0x?f1` with `+0x1294` (`0x1000daf8`, `0x1000dad6`) — so `+0x1084` is
  **"LOD distribution"** and `+0x1294` is **"High quality LOD"**. One call reads them (`0x1000ec50`):
  `rand16 × distribution[group] ÷ 65536 − highQuality[group]`, drawn once per particle **per channel**
  as a burst or a stream spawns, and taken as that particle's lerp parameter only where the block's
  exponent is **negative**. *Measured*: **0 of the 21426** position and size exponents are negative, so
  **no shipped effect takes that path**, and the 923 effects fall **282, 146, 144 and 351** by group.
  The page's **`+0x14a4` is 50, 250 or 1000 by preset** and **has no reader** — control: the same sweep
  over the page's fields finds both the writes and the reads of `+0x1084` and `+0x1294` beside it
  ([11-effects](docs/11-effects.md)).
- [x] ~~[M4] The effect manager's random generator and jitter, the owner values of time modes 5–15, and
  a phase's animated frames.~~ — closed 2026-09-20, all four parts, **and the engine was wrong on three
  of them**. **The generator** is `Effect.dll:0x10002220`, a pair of 16-bit shift registers — `lo =
  (lo << 1) ^ hi`, then `hi = (hi >> 1) ^ lo`, returning the new `hi` — the engine's house generator,
  already read inlined in `Control.dll` and the patroller and reached as a routine for the first time.
  **Verified independently by the coordinator**: as a GF(2) map it has **rank 31**, so one transient
  step, and its cycle is exactly **1 065 353 089 = 127 × (2²³ − 1) = 127 × 47 × 178 481**, with none of
  the three prime quotients closing it. Thirteen copies of the state sit in `.data`, **seven ever read,
  six seeded and never drawn from**, and nothing gives a manager or an instance a state of its own, so
  the streams are shared process-wide. **Flag 1** adds a uniform in ±half of the header's `+0xc` to *t*
  and clamps, on **58** effects; **137 more carry a spread the flag never reads**. Flag 8 is the same
  shape on the *place* on 57 effects, and that triple is `(0,0,0)` on **all 923** — a negative whose
  control is flag 1's spread in the same header, non-zero on 56 of its 58. **Time modes**: 5–8 are
  velocity over the per-axis top speed, 9–12 the spin, **13** is 1 − the attach point's value, **14** is
  1 − the owner's **life fraction**, 15 the larger of 5 and 9. *Measured* over the 923: mode 5 on **46**,
  15 on **31**, 14 on **8** (the five `tree_flame*` among them) and **modes 6–13 on none** — so a burning
  tree's fire runs forward as it burns down, which the stand-in's "all read speed" got wrong. **A phase
  is not a second clock**: its fractional part is **where its material's own animation track stands**,
  multiplied by the track's whole length by `GetMaterialPhase` (`World3D.dll:0x10003680`), a fraction
  outside 0..1 becoming 0.5. *Measured*: **2062 of 3577** material references name a material with more
  than one key, and the phase moves on **all 2013** type 3/4/9 blocks, **832 of them clocked in
  seconds**. That frames advance at all was already right — that half of the row was stale
  ([11-effects](docs/11-effects.md)).
- [x] ~~[M4] How often an effect tests its point's view, and what that ray meets.~~ — closed
  2026-09-20, **and the engine was wrong about the ray**. *How often*: the draw keeps a deadline at
  `+0x30` and after each test sets the next to `now + I + uniform(±I × 0.25 / 2)`, with `I` from
  `0x10026a7c` — **a zero `.data` float that nothing writes**. *Re-checked by the coordinator*: it has
  exactly **two** references in the module, `fld` at `0x10007fa4` and `fadd` at `0x1000800b`, both
  reads, against the control of the generator state `0x10024110` three instructions away, which the same
  sweep finds with **six** references including two plain writes. So the interval and its jitter are
  both 0, the deadline is set to *now*, and "every frame" was right and is now exact. *What it meets*:
  the ray goes into **`IWorld` slot 7**, the **sight ray's** entry, not the slot-6 mesh test a round's
  ground query uses, with the record `[0x40a, 0, 0, 0, 0, 8, 0, 0]` against a round's `[0x41e, …,
  0x208, …, 0x24]`. It **excludes no face class at all**, so the water sheet a round passes through
  (3630 of 275882 faces) **stops an effect's view test**, and its one excluded world flag, 8, is the
  landscape flags word's `0x20`, on **0 of the 275882**. The engine had cast a round's query; fixed, and
  pinned by a test that the same ray is stopped by water where a round's is not
  ([11-effects](docs/11-effects.md)).
- [x] ~~[M4] What a building answers for a strike's material, and a node's wear base.~~ — closed
  2026-09-20: **a building answers exactly as a unit does, and the engine was half wrong.** A
  `CBuilding`'s own `QueryInterface` (`Terrain.dll:0x10057d50` → `0x10057c20`) answers only five ids —
  0, 6, `0x11`, `0x17`, `0x18` — through a byte table at `0x10057d2b`; everything else, **`0xd` among
  them**, falls to the default arm, which forwards to the agent it aggregates at `+0x2c` (`0x10057cf6`),
  whose own table serves it. *Measured* over `fortif.rlb`: its **34** building models carry **1034**
  wear materials — class 5 on **898**, unset on **92**, 8 on **25**, 6 on **18**, 10 on **1** — so a hit
  on a building plays slot 6 `mt` on 898 of its skins and slot 0 only on the 92 unset. The engine's
  `struck_class` turned a building away and played slot 0; that early return is gone, and an install
  test drops 48 rays through Mission 01's two bridge halves and asserts class 5 (it fails with the early
  return restored). **The wear base was already right**: the material id is `[mesh+0x10] | batch
  material byte`, and `+0x10` is written from an argument shifted 16 up (`0x1000a726`), which the
  manager splits back as `id >> 16` and `id & 0xffff`, so the base has a zero low word by construction
  and **the batch's material byte alone indexes the wear** ([11-effects](docs/11-effects.md)).
- [x] ~~The colour and range jitter of an effect light (block `+96`, `+120`)~~ — raised and closed
  2026-10-01, **read** and **measured**. `Effect.dll`'s light update (`0x1000f6e0`) returns at once
  when the manager's record is off (`0x1000f75a`) and otherwise takes **five draws on every instance
  update**: the colour's alpha, blue and green on the light emitters' own state (`0x1000f8c7`,
  `0x1000f90a`, `0x1000f94d`), its red through the bursts' state (`0x1000f99f`), and the range's
  (`0x1000fa8e`). Each is a uniform in ± half of its spread, taken whether the spread is 0 or not;
  the update is the instance's, every 100 ms (`0x100081a7`), so a jittered light steps ten times a
  second. The range's floor is 0.01 where it is not above 0, not "at least 0.01". Of the 618 light
  blocks 107 carry a colour jitter and 58 a range jitter; the 22 point lights among them are the
  construction sphere's three, four explosions and twelve `tree_*` fires and glows, and no gun's
  flash. The engine left both out and now draws them in the game's order, held 100 ms
  ([11-effects](docs/11-effects.md)).
- [x] ~~Whether an object takes an effect's light, and what writes a batch word's `0x800`~~ — raised
  and closed 2026-10-01: **nothing writes it, and objects are lit all the same**, on their vertices.
  - **The negative, with its control.** The word the mesh draw tests for `0x800`
    (`Terrain.dll:0x10045d47`) is loaded once (`0x1004502f`) from the record the mesh's slot 3
    answers; the one implementation (`AniMesh.dll:0x100134d0`) stores the file's stream-13 dword and
    ors in `0x20` alone. The stream is the archive's bytes in a view no call maps writable. Over the
    15153 batches the word takes 14 values over nine bits: `0x800` on none, against 633 with 8, 2953
    with `0x100` and 972 with `0x2000`. So no object takes the emulated **disc**.
  - **The lighter.** Every draw item carries the shade's gathered list (`0x100457c5` for a mesh,
    `0x1004482a` for a landscape cell), and `RenderVB` hands it to `CShade::ShadeIndexedStrided`
    (`0x1004df70`): nothing past the range or on a vertex turned away, otherwise material diffuse ×
    light colour × cos × (a₀ + a₁x + a₂x²), x = (range − d) ÷ range, the block's own three terms.
    Neither gather asks `0x20000000`, which only keeps a light out of the disc.
  - **Three statements of docs/11 are taken back**: kinds 6 and 7 do not "light nothing"; the
    attenuation triple is not dead data; and a Direct3D light *is* built (`0x10030620`), but only
    under `UseDXLighting`, which is 0 as compiled (`0x1005fcce`) — docs/11 had taken the descriptor's
    type word, 2, for its default.
  - *Seen*: C03 M02's Medium Mine is washed violet by `mineglow`'s blue light over the red scene
    (Part 6, 0:52–0:56), and the engine now is. The engine lit no vertex by an effect's light; it now
    carries the frame's 64 nearest and lights model and landscape vertices by the read falloff
    ([11-effects](docs/11-effects.md#what-a-light-does-to-a-surface--read-and-measured)).
- [x] ~~Which way a type-9 dome's pole points, and what a sprite's mode is~~ — closed 2026-10-01,
  **read** and **measured**. A type-9 block is type 3 with one more step (`Effect.dll:0x100138c0`),
  and `CShade::RenderEffect` (`Terrain.dll:0x100288b7`) draws it as a mesh built once: **a true half
  sphere of radius 1, its pole at (0, 0, 1) of the sprite's own space**, 8 × 3, 16 × 6 or 24 × 9,
  drawn from both sides. The sprite's own z is set by the **sprite mode, the block's `+4`**
  (`Effect.dll:0x100103d2`): mode 0 faces the eye, mode 1 is a streak along the block's direction
  channel, mode 2 is square to it (`0x1000d110`). Of 2013 sprite blocks 983 are mode 0, 251 mode 1
  and 779 mode 2; of the 266 type-9 blocks 258 are mode 2, and **122 have a twin of the opposite
  direction in the same effect**: balls. `f_gener_ball`'s two blocks differ in two bytes, the sign of
  the direction's x. A block whose `+204` is set gives every facet the whole texture
  (`Terrain.dll:0x10028bbe`), which is the construction sphere's cells. *Seen*: the generator's whole
  gold ball (Part 6, 64:50) and the construction dome's opaque blue cells (33:32). The engine stood
  both halves on one axis and wrapped one texture round a dome
  ([11-effects](docs/11-effects.md)).

## Motion, ground and controls

[24-motion](docs/24-motion.md#not-established), [13-control](docs/13-control.md#not-established),
[14-controls](docs/14-controls.md)

- [x] ~~[M3] How interface `0x25` slot 3 turns level-0 triangles into a push, and what slot 2 does
  with its 0.5.~~ — this line was **stale**: an earlier round answered it and this file was never
  ticked. The push accumulates over the unhidden faces within the sphere, nearest first, and is held to
  4r; the 0.5 is how far off a triangle, in x and y, a point may still find it
  ([24-motion](docs/24-motion.md#collision-between-objects--read)). Whether any `Terrain.dll` class
  answers `0x25` is still not checked.
- [x] ~~[M3] Which scene nodes are types 1 and 3 ... plus a machine's type-3 parent and what message
  `0x201` returns.~~ — closed 2026-09-19. The types half was **stale** (a `CBuilding`'s slot 11 is 3,
  so a bridge is ground), and all three remainders are now read. **Message `0x201` is the size class,
  and the guess is confirmed**: interface `0x10` is the `MBehaviour` the agent build files at
  `AniMesh.dll:0x1000361f`, its slot 26 answers `&[this + 0x960]` (`Behavior.dll:0x1000a533`), and that
  field comes from the machine's slot 54 (`0x1000cee0`), which reads **a different letter of the root
  component's member by the node's kind** — kind 3, a building, its **fourth** (`0x1000cf43`, behind a
  `cmp ecx, 4` guard: b 4, e 5, l 2, m 3); kind 4, a unit, its **third** (`0x1000cfb1`, behind
  `cmp ecx, 3`: b 4, h 2, l 2, m 3, t 1). Both branches re-decoded by the coordinator. *Measured over
  the whole install, and re-counted independently*: of the **458** assemblies with a root, all **382**
  unit roots carry a size letter third (t 21, l 149, h 21, m 89, b 102) and all **76** building roots
  one fourth (l 37, m 21, b 15, e 3); **none** falls to the default, and a unit's fourth letter is `_`
  on every one of the 458, so neither branch could ever read the other's. It is the same class
  [31-packages](docs/31-packages.md) already had, with the fifth class and the building half nobody had
  written down. The **masses property `0x7c`** is what the machine *weighs* — case 22 of the control
  property table, `&control[+0x538]`, the "Weight" the stat panel shows and **not** the file's +124 —
  271 kg (`s_arah`) to 4,804,877 kg (`m7_tow`) over the 382, while **a static object answers nothing**
  and never needs to, having no contact record. And **nothing sets a collision entry's skip flag**:
  slot 4 sets it and slot 3 clears it (the doc's old "slot 7 / slot 6" was counted from the wrong
  vtable base), slot 3's one caller is the object's own detach, and of every no-argument indirect call
  at that offset in the install none has a collision object for a receiver — control, the same search
  one slot over, which finds `AniMesh.dll:0x10001850`. So **no entry is ever skipped**
  ([24-motion](docs/24-motion.md), [13-control](docs/13-control.md)).
- [x] ~~[M3] The ground contact's timing and dt, the pose its contact points use, the second
  sphere's radius r₂, and what lifts a sphere with no face under it~~ — closed 2026-09-18, all four.
  It runs **once a frame** for every unit, a flyer included — message `0x1c` to slot 24
  (`0x10007d03` → `0x1000cb80` → `0x1001a450`), gated only on the agent's kind being 4 — and its
  **dt is the frame's own milliseconds** at machine `+0xe8` (`0x1001b41b`), not the state step's
  length. Its points are read at the **frame's interpolated pose**: the pass makes no slot-25 call
  anywhere, every virtual call in it enumerated, and asks the node's current matrix, which the
  machine tick has just set by playing the mesh at that frame's time. That **refines the round
  before this one** rather than overturning it — the state's end pose is used once per state, to
  work `CONTACT_PLACE` out, and the run-time pass reads the live one. **r₂ is the node sphere's
  radius**, held to 7.5 only when it is under 20 *and* the object carries flag `0x1000000`
  (`0x1001a51b`–`0x1001a58a`; the constants at `0x1003c044`/`0x1003c048` are 7.5 and 20.0, read out
  here) — and it differs from r on **all 148 unit models the campaign places**, r₂/r from 0.42 to
  2.34, larger on 26 and smaller on 122, so the engine's `r₂ = r` was wrong on every unit in the
  game. **A sphere with no face under it is lifted by its whole r**: the failure path copies the
  centre into the ground point and the constant (0, 0, 1) into the normal, so the gap is 0, above
  −r, and the lift is r every frame; a contact point with no face is handled the same way and lifts
  by 0, its default normal still joining an average taken over the contacts' count **plus one**.
  Fixed and tested ([24-motion](docs/24-motion.md)).
- [x] ~~Which `Land.msh` faces carry the world bit `0x8` and class bit 8 that the ground search
  excludes~~ — closed 2026-09-18: **none of them do**. The missing step was naming the fields. The
  ground search excludes world `0x208` and class bit 8; `Terrain.dll` folds each pair into one
  landscape mask, and the landscape's mask is the file's face record unconverted — the flags word low,
  the surface word high. So the two bits are the flags word's `0x20` and the surface word's `0x04`, and
  **0 of 275882 faces across all 33 maps** carry either, at both levels of detail. The reading is
  cross-checked on two bits of the same word that faces *do* carry: landscape `0x2000` is the flags
  word's `0x2000` on exactly the 6102 bed faces, and landscape `0x20000` the surface word's `0x02` on
  exactly the 3630 water faces — which is also the control. The engine's ground index was already
  exactly this filter. What would ever set them is not established
  ([24-motion](docs/24-motion.md)).
- [x] ~~Contact record flag `0x20`~~ — closed 2026-09-18, and it **corrects the flag-2 entry below**.
  `0x20` does not mean "place": taking a state poses the object at the state's end, asks each contact
  point for its own axis, and then *sets* `CONTACT_PLACE` where that axis stands up — z above 0 and
  1 − z below 0.05 — and clears it otherwise. So the flag is worked out afresh **each time a machine
  takes a state**, not read from the file. Measured over all 2634 contacts: 2410 carry `0x20` and every
  one of them is a **foot** — nothing with a wheel or a belt has it, and none of the 2410 is authored
  with flag 2. Posed at each state's last frame the axis stands up on 2217 of the 2410, so **2229
  contacts in the game lay their node along the ground**, not the twelve this file had counted. A
  walker's feet conform to the terrain in every state whose animation ends with the foot flat (20 of the
  hero's 210 states withhold it); the tracked belts are simply the case with nothing to work out. All
  2410 axes are unit length, which is why the game's unnormalised compare against 1 is a cosine. Read,
  measured, implemented and tested ([28-chassis](docs/28-chassis.md), [24-motion](docs/24-motion.md)).
  (~~flags 2 (slot `0x7c`)~~ — closed 2026-09-18: flag 2 is `CONTACT_PLACE` as authored, on 12 of the
  2634 contacts, four each on the three tracked chassis, every contact of the wheeled chassis being
  `0x5`. Right about the flag; wrong that those twelve were all that place.)
- [x] ~~[M3] How often `World3D.dll`'s input update runs (it paces the cruise ramp), and which screen
  states set the 0.5 mouse sensitivity.~~ — closed 2026-09-19; the sensitivity half was **stale**
  (while the view is zoomed — states 1, 2 and 6). The input update (`0x1000f100`) is slot 4 of the
  manual manager and its **only caller** is the manager's own message handler, on message 1, down a
  chain now read end to end: `iron3d.dll`'s mission loop → `stdCalculateGame`
  (`World3D.dll:0x100139a0`) once a pass → the game object queue's slot 4, which reads `timeGetTime`
  into the game clock and sends every object `send(6, 1, clock)` → the agent's id-6 arm → the Wizard →
  the manual manager at its `+0x64`. **The loop is not capped** — body `0x1005e713`–`0x1005ef9e` ending
  in `Sleep(0)`, with `Sleep(100)` taken *instead* of rendering when the render flag is clear, no timer,
  no frame count and no `iron_3d.ini` setting — so the update runs **once a rendered frame, at whatever
  rate the machine draws**, and the keypad's 31-step cruise ramp takes about half a second at 60 fps
  and a second at 30. Two gates sit on it: the manager passes the whole update over while the clock
  stands on the same whole millisecond, and it clears its state and calls `stdClearKeyboard` when more
  than one `stdCalculateGame` has gone by since its last. **This closes
  [30-turrets](docs/30-turrets.md)'s "how often the game frame runs" with it** — the game frame is one
  pass of the same uncapped loop. Note that the recording's zoom-in fitting 60 steps a second is
  *consistent with* but does not prove a 60 Hz tick: it says the recording ran at 60 fps
  ([14-controls](docs/14-controls.md), [24-motion](docs/24-motion.md)).
- [x] ~~[M3] What handlers do when an active row runs again each update, and who calls the second
  walk/turn ramp's setter (slot 11).~~ — closed 2026-09-19; the first half was **stale** (a row with no
  ramp time runs once per key transition, and only a ramp row runs again while its key is held). The
  second half is now a **controlled negative replacing an uncontrolled one** — the doc had said plainly
  that its scan "had no positive control, so it proves nothing". **Nothing calls slot 11.** Its body
  (`0x1000b380`, `ret 0x14`) is referenced nowhere in `World3D.dll` outside the manager's fourteen-slot
  vtable `0x10020b14`, so every call must go through that offset; of the **345** indirect calls at it
  across all sixteen modules, **none** has a receiver that could be this object. The control is the
  identical enumeration at other offsets of the same vtable: slot 2 finds all three places a manual
  manager is sent a message, slot 5 finds the AI poking keys in, slot 4 finds the input update. So
  **the second ramp is dead code**, and every walk and turn takes the row's own ramp time from the
  `.tbl`. *Measured*: of the 116 shipped rows exactly **six** carry a ramp time — two per table, always
  the keypad's `+` and `−`, always 0.05 over 1000 ms — and the other 110 carry 0. The engine was right
  ([14-controls](docs/14-controls.md)).
- [x] ~~[M3] A state's use count `+0x94`, the state a machine starts in, the game's jitter random
  source, and a controller's request code before any is sent~~ — closed 2026-09-18, all four, and
  the engine was wrong on three. **A use count of 0 means the state never applies again**
  (`0x10001132`, tested before the code), and the planner spends one on its destination anchor
  unless it is −1 or already 0; *measured over the install's 1690 states*: −1 on **1607**, 30 on
  **80**, 20 on 2 and 10 on 1, every finite one on an anchor and on its controller's state 0, the 80
  being `static.rlb`'s one-state controllers of a 500 ms step, so 30 uses is fifteen seconds of life
  (counted here independently). **The starting state is index 0** (`0x10006d19`), but the record a
  machine starts with is the constructor's zeroed copy, whose use count is 0 — so it does not apply,
  and the first plan runs from 0 to the cheapest anchor that does. **The jitter is neither `rand()`
  nor a 32-bit xorshift**: a pair of 16-bit words inlined at `0x100057de`, `s0 ← (s0<<1) ⊕ s1` then
  `s1 ← (s1>>1) ⊕ s0`, seeded once at load from `ngiGetClocks` through the module's `_initterm`
  table — which matters, because all-zero is a fixed point and an unseeded generator would hand
  every jittering step the same −12.5%. **The request code starts at 0, not −1** (`0x10006ecf`):
  *measured*, 1510 states ask for no code and the other **180 are the 30 `fortif.rlb` buildings' six
  apiece**, codes 0, 1, 2, 6, 8 and 10 (counted here independently), so a finished building applies
  its code-0 state — stop the construction ray — from its first tick, which the old stand-in
  prevented ([24-motion](docs/24-motion.md)).
- [x] ~~[M3] The vector that righting bits `0x30` stand the hull toward (`+0x348`, no writer
  found)~~ — closed 2026-09-18, and **no negative was needed**: the field no search for a writer had
  ever found is one the project already had under another name. Control `+0x348` **is the motion
  body's `+0x194`**, the averaged ground normal of the last landing — the body sits at control
  `+0x1b4`, and the lift writes the normal there through the body's own `this` (`0x10015e55`, inside
  `0x10015d60`, called with `lea ecx, [esi+0x1b4]` at `0x1001b440`). Body `+0x194` has exactly two
  writers, the body constructor and that lift, and two readers, the mode-2 brake and the righting at
  `0x1000c439`; the same arithmetic already underlies `+0x21c` = body `+0x68` and `+0x254` = body
  `+0xa0`. So the **390 states with bits `0x30`** — the six wheeled and tracked chassis, 91 Large
  Walking and 13 Tiny Spider states, two animals, 15 stones and 21 trees — stand their hull along
  the ground they last landed on. The engine already kept the right field and now knows what it is;
  righting itself still waits on a body with more than a yaw ([24-motion](docs/24-motion.md)).
- [x] ~~[M3] Which way across a slope the mode-2 brake acts, and which way a positive lean tips the
  model~~ — closed 2026-09-18: the brake acts **uphill, or exactly along the contour**, which is
  what the engine had but for the contour case, now included. The integrator builds C = up × N and
  D = W × up and skips on a negative D·C (`0x100156ae`–`0x10015799`); that dot is −(W·N) over x and
  y alone and (Nx, Ny) points downhill, so the sign does not depend on which way the global up
  points. The lean's sense falls out of the rotation itself: the turn triple goes to three
  axis-angle quaternions (`0x100141c0`) and `Ngi32.dll:0x10014450` builds not R but **S·R·S with
  S = diag(1, 1, −1)**, so **a positive pitch tips the nose down, a positive roll the top to the
  left, a positive yaw the nose to the left** — which is what the righting needs, since its settle
  angles are *added* to the spin at `0x10014b83` and could not otherwise converge. Every prose
  reading in the existing lean table survives: the flyers' `0x03` banks *into* a turn and the
  wheeled `0x83` leans *out* ([13-control](docs/13-control.md), [24-motion](docs/24-motion.md)).
- [x] ~~[M3] A chord with no row of its own, such as Shift+W~~ — closed 2026-09-18: **the plain row**,
  unless another row of the same key claims that modifier. There is no best match and no search. A pass
  after loading appends each modifier's scan code to every plain row of any table with the same key; a
  key event then activates a modified row only while its modifier is held, and a plain row only while
  **none** of its listed modifiers is. Measured over the three tables' 116 rows: `SCAN_LSHIFT` is the
  only modifier any of them uses, on 4 rows each, and the pass writes exactly two exclusion entries per
  table — the plain mouse X and mouse Y rows. Every other plain row's list is empty, so Shift+W walks.
  The engine's stand-in agreed with the rule on all 116 shipped rows; it now applies the rule
  ([14-controls](docs/14-controls.md)).
- [x] ~~What behaviour flag `0x800` changes besides clearing the walker.~~ — closed 2026-09-20:
  **it is the unit's leave to be ordered at all**, not a walker detail. Every access to `MBehaviour
  +0xa04` anywhere in the fifteen modules is in `Behavior.dll` — 51, all at that one displacement — and
  bit `0x800` sits at five of them in three routines: the unit takt (slot 56, `0x10005110`) sets it at
  `0x100051e9` and clears it at `0x100052b0`, and the two readers are **`MBehaviour::AddOrder`** (slot
  3, `0x10004a90`), which with the flag clear returns 0 and never reaches `MakeNewOrder`, and the
  **self-given task factory** (`0x10034579`), which then builds no attack and no reload task. Both
  escape on the Type word carrying `0x80000000`, a building. Control: the same sweep finds the readers
  of bits `0x1`, `0x2`, `0x4`, `0x10`, `0x20`, `0x40` and `0x1000`. *Measured*: **94 of the 531**
  controllers can ever carry it; three of the 24 `bases.rlb` chassis cannot — `r_h_01`, `r_h_03` (the
  shooting-range targets) and `r_b_06` (the **Small Tower**) — and the missions place **33** of them,
  all Type `0x01008000`, a warrior, so **a Small Tower can never take an order or a task of its own**.
  Its shooting is the fire control's, not this path, which fits what this file already holds about a
  tower's reach. The engine models no behaviour flags, so nothing was wrong
  ([24-motion](docs/24-motion.md)).
- [x] ~~The remaining `.ctl` values~~ — closed 2026-09-20, **all four**, and three of them are
  negatives with controls.
  - **Class 3's value 0, the camera's 3–5 and the arms' 1 and 4 are read by nothing.** A value leaves a
    component only through vtable slot 4 (`0x10021d00`, `[this + 4*(id & 0xff) + 0x54]`); the camera
    class's constructor and all sixteen slots of its vtable touch that range only through the getter,
    and it asks *itself* for ids 0, 1 and 2 alone. Across fourteen modules ids 3/4/5 are pushed at 18
    sites — all radar, detect, shield, gun or the two `IDeviceManager` summaries, **none a camera** —
    against **100, 196 and 28** pushes for ids 0, 1 and 2, which is the control. *Measured*: all **61**
    cameras carry (0.1, 1000, 1.3, 1, 150, 1); class 3 is 72 records with value 0 at 0.5 on eight.
  - **The section-5 record's int 8 is read by nothing**: the interpreter `0x10002800` strides the
    records by 100 and reads `[esi]`, ints 1–2, `+0xc`, `+0x10`..`+0x1c`, `+0x24` and `+0x44`, while
    all three `[esi + 0x20]` follow an `add esi, 0x24` and address the member name. *Measured*: **2923
    of 2925 are zero**, the two others action-3 records naming `eng_rb_07_snd` and `eng_rb_08_snd`.
  - **Control message 7 says who simulates the object.** `0x10007bdc` sets `+0x618 = (arg == 2)`;
    argument 2 comes from `CreateMirror`, `AddNewMirror` and the queued `ChangeOwner`, and 0 and 1 from
    `LoadObject`, the owner-change handler and `iron3d.dll:0x10074ff0`. While the byte is set the
    ground contact does not lift, node damage is put back, and the camera makes no view unless the
    owner is a building. **Still open: what separates argument 0 from 1** — `Control.dll` treats them
    alike.
  - **`IDeviceManager` ids 5 and 6 are the unit's damage a second**: the sum over class-2 components of
    `+0x174 × 1000 ÷ max(1, value 3)`. `+0x174` has two writers — the constructor zeroes it, and the
    round-creating routine stores the new round's property `0x35`, the nodes' hit points plus their
    explosions' damage — so **a gun that has not fired answers 0**, and a salvo counts one round.

    The engine had none of these wrong ([13-control](docs/13-control.md),
    [14-controls](docs/14-controls.md)).
- [x] ~~[M14] How a walker drops the points a unit has passed, who calls `MHallWay` slot 11, the
  hall-way vertex size gate and the two link flags~~ — closed 2026-09-20 as a **named slice** of the
  large walker line below, **and the engine was wrong**. `MWalker::ClearMoverReachedPoint`
  (`0x1003cfd0`, self-named) releases the hall-way place the walker held, takes *n* records off the
  **front** of the trajectory, adopts the last of them as the walker's own place, and hands back every
  vertex any of the *n* had booked. **`MHallWay` slot 11** (`0x1000b390`) has exactly two callers,
  `OnAddStatic` and `OnRemoveStatic`, each over every scene object of kind 3 — so a building's exits
  are relinked at load and whenever a tree or a stone moves. **The size gate** (`0x10042d08`, in
  `MWorldGraph::AddNeighbourToFront`): the unit's `+0x960` is its **size class** (T 1, S 2, M 3, L 4),
  measured against the vertex's flag word. *Re-measured by the coordinator* over the 29 shipped hall
  ways and their **1056** vertices: **165** carry `0x10000000` (any size), **7** carry `0x20000000`
  (the building's own — all seven on the three factories) and **884** carry neither (size class ≤ 2).
  **All 21 control pods are in the 884**, the same bound the capture order applies, enforced a second
  time by the path graph. **The link flags**: over **1096** links, **1078 carry neither, 18 carry
  `0x10000` and none carries `0x20000`**. A by-catch corrects the toolkit: its claim that a link's
  eight tail words are "`0xFFFFFFFF` throughout the shipped data" is **wrong** — *re-measured*, **62 of
  the 1096** links carry a tail word that is not, and `openparkan/mesh.py` is fixed. The engine's "every
  vertex passes" stand-in is gone and the search now gates by size; nothing about the shipped game
  moves, since the engine feeds only bridges' hall ways into the search and all 37 of their vertices
  carry `0x10000000` — which is itself the result that the stand-in was right *for what the engine
  models* ([24-motion](docs/24-motion.md)).
- [x] ~~**A warbot the player drives himself blocks on the first ramp down past a factory's
  door.**~~ — raised and closed 2026-09-20 from play on C02 M03, *The Lost Key*. It is the
  **ground contact's lift**, not the collision, though the collision is what freezes it. The lift
  is the largest rise over the flag-1 contacts, whatever its height, and each contact searched
  with the **agent sphere's r** as its up-pass bound. An SWW-X Warrior a metre past the Large
  Factory's door, nose over the ramp: its two front wheels found the ramp 2.8 m below, its two
  rear wheels found the floor they had just left 2.9 m **above**, and the max hoisted the whole
  machine 2.9 m back up the ramp into the structure over it — where the collision's segment
  test ran the move against a face and put it back at its start. Every tick, for good, at speed
  17.9 m/s. The bound is **r2**, the body sphere's, now: what a contact's own up pass tests
  against is read to be a triple built from control `+0x2ec`, `+0x2fc` and `+0x30c`
  (`0x1001aba7`, `0x1001ae12`) and is **still not read**, so r2 is a stand-in like r was — but r2
  is the one bound the pass *is* read to use (**read** 2026-09-30: it is r2 itself, copied into
  the fourth word of that record at `0x1001acb8` and compared with alone — docs/24, "A contact's
  up pass reaches the node sphere's radius"), and it is the smaller on 122 of the 148 unit models
  the campaign places. *Measured* over 18 AI captures on C02 M03, every small chassis against
  every enemy building: 15 taken before, 14 after, the Large Factory 5 s faster for a wheeled bot
  and 15 s for a walker, and one lost — below ([24-motion](docs/24-motion.md#holding-the-body-on-the-ground--read-and-measured)).

- [x] ~~C02 M03's Small Bunker (`l_bunk1`) cannot be captured by anything on wheels, and C02 M04's
  plateau Light Tower (`mtow02`) by its small wheeled warrior (`24swele1`).~~ — closed 2026-09-30:
  the hull **rights itself along the ground**, which the engine had left out. The read was already
  in [24-motion](docs/24-motion.md#the-hull-leans-and-rights-itself--read-and-measured): a state
  with bits `0x30`, every state of the six wheeled and tracked chassis, turns the hull each step by
  triple 5's share of the angle to the averaged ground normal the lift last took, pitch and roll,
  and the contact points are placed through that turn. Held level, a wheeled bot going down a
  building's ramp dipped its front wheels under a floor overhanging the way down, the up pass took
  that floor within r₂, and the lift put the machine on it: `22swel1` held 45.8 m short on top of
  the bunker's ramp and `24swele1` circled the tower's entrance for good. Nose down, `22swel1` takes
  the bunker in 17 s and `24swele1` in 33 s, and `24swele1` takes the tower in 41 s (*measured*,
  each sent from 60 m out with the building's guns off; the control is the same run with the
  righting taken out, which gives back the stall). The sign is checked on the running gear: over
  the 12 bots standing tilted at three mission starts, the tilt as computed leaves the smallest
  spread in their wheels', tracks' and feet's height over the ground in 11, the negated tilt the
  largest, and the one exception is a tracked bot in motion whose tilt lags the ground. The lean
  (state `+0x08`, triple 6), which only the drawn body takes, is still left out. What stops a small
  walker at the bunker is another question and stays queued in
  [OPEN-QUESTIONS](OPEN-QUESTIONS.md).
- [x] ~~C02 M03's Small Bunker (`l_bunk1`) stops a small walker short of its pod~~ — closed
  2026-10-01 by the sphere the pair pushes out, **read** and **measured**.
  - **A collision object keeps two spheres.** Interface `0x18` slot 9's argument is a space, not a
    choice of sphere (`AniMesh.dll:0x10014587`). A collision object of kind 4 or 3 keeps a second
    sphere record (`Control.dll:0x1001f2ee`–`0x1001f332`), which message 1 fills (`0x1001fec0`): for
    a unit its **node sphere**, interface `0x20` slot 3. The pass sweeps and orders pairs by the
    first, the agent sphere, but the push-out's mover is the second (`0x1001dc9d`–`0x1001dcb0`), its
    radius **held to 7.5 when the mover's Type carries `0x1000000`**, a robot
    (`0x1001df7a`–`0x1001dfb2`).
  - *Measured*: the node sphere is over 7.5 on 27 of the 143 placed robots, all held; against the
    Large Factory's 22.1 × 15.4 m door the agent sphere is wider on 20 of the 37 placed
    large-chassis models and the pair's on none.
  - The engine pushed out the agent sphere. With the pair's, `21swlk1` takes the Small Bunker's pod in
    61 s and `22swlk1` in 14 s, where neither did; a large walker made in C03 M02's Large Factory is
    on the landscape 3.5 s after it is made, where it stood at the door all mission. The ground
    contact's hold of r₂ under 20 to 7.5 is the same Type bit (`0x1001a540`), a stand-in closed with
    it ([24-motion](docs/24-motion.md#collision-between-objects--read)).
  - **Nothing excludes a pair** inside a building: no child relation, property or door is asked
    before the push-out's face-by-face door test. Only the sphere differed.
- [x] ~~What keeps a walker upright on a building's floor under a low ceiling~~ — **narrowed**
  2026-10-01 by the same read: the ceiling presses C02 M03's `22mwlk1` 0.28 m, not 2.85. What is
  left, where the frame is drawn in the takt, is queued.
- [x] ~~What the eye the sprite draw is handed has been transformed by~~ — closed 2026-10-01,
  **read**: `g_FastProc` slot `0x68` (`Ngi32.dll:0x100248f0`), the projection on each axis of the
  matrix over that axis's squared length; the instance's scale is in the matrix
  (`Effect.dll:0x10007c90`). The engine's assumed transform was the right one in kind and is now the
  read's ([11-effects](docs/11-effects.md#a-sprite-is-drawn-through-its-frame--read)).
- [x] ~~Which axis of a frame about one control point takes the point's direction~~ — raised and
  closed 2026-10-01, **read** and **measured**. `Control.dll:0x10003ef0` writes the matrix's axes as
  **the direction, its side, direction × side**, so the direction is the first axis. The side is the
  level perpendicular (−y, x, 0); where x is exactly 0 it is the x axis, and where y is and x is not,
  the y axis (`0x10003f65`–`0x10003fa8`), so a vertical direction never degenerates. 690 load-group
  action-4 records on 101 objects name one point three times; the direction is a unit long on 689,
  and the one other is `fr_l_gener`'s `Sign_Type1`, (0, 0, 8.98)
  ([13-control](docs/13-control.md)).

## Turrets, weapons and camera

[29-weapons](docs/29-weapons.md#not-established), [30-turrets](docs/30-turrets.md#not-established),
[28-chassis](docs/28-chassis.md#not-established), [25-sensors](docs/25-sensors.md#not-established)

- [x] ~~[M4] Whether the landscape is among the objects the sight ray walks~~ — closed 2026-09-18:
  **it is, and it is walked first**, because the world's root object *is* the landscape.
  `CLightning::Init` fetches the root through slot `0x30`, reads its class through the same slot
  `0x2c` the walk uses, and panics *"Root object is not a landscape"* unless the class is 1
  (`Terrain.dll:0x10071c44`, read out here verbatim); the walk admits `1 << class` against the
  query's mask through the table at `0x1009a5f0`, and the sight's `0xfff`, a round's `0x41e` and the
  camera's `0x41a` all carry bit 1. The landscape answers interface `0x18`, whose slot 6 is
  `GetFirstIntersectedFace`. The control is that the same enumeration finds the child lists the walk
  descends into (`0x100254e4`, `0x10025520`). A correction falls out with it: the sight's query
  excludes **nothing** (`0x1002a68e`, zeroed at `0x1002a6c1`), so unlike a round it **stops on a
  lake's surface**. The engine had given the sight ray the round's water-excluding index; fixed and
  tested ([29-weapons](docs/29-weapons.md)).
- [x] ~~[M4] What a falling round's mount solves for with no target, which way its lift turns on a
  hung turret, and whether a player's turret is in `CIS_MANUALCONTROL`.~~ — closed 2026-09-19; the
  headline was **stale** (the traced point less `TurretCenter`'s position; a manual turret gets no lift
  and its gun is ready) and the remainder is now read — **on a false premise**. A gun with no mount
  keeps the **1** its constructor writes into `+0x118` (`Control.dll:0x100295b3`), the mount step being
  the only other writer, so it is ready for good and never lifted. But **the Small Bunker is not an
  example**: `e_bnt_lt_01` has no follower channel of its own, yet each `e_gun_fc_08` bolted to it
  **brings its own** (node 2, flags `0x8`), and a fitted part's follower joins the turret's list at load
  (`0x10009120`) — so the bunker's flamers are mounted and lifted like any other lobbing gun.
  *Measured* over the 458 assemblies: **1,015 followers against 1,034 guns**, and the **19** assemblies
  that fit more guns than followers are three animals' own guns, six guns filling a builder's battle
  slot and ten second beams of a builder module — **every one of the 19 fires a mode-0 round**, so no
  lobbing gun is ever left unmounted. Report 7 is **two** refusals, not one: the ready byte
  (`0x10029d27`) and the range (`0x10029e37`), whose value 8 is the round's `.ctl` +108, already read
  ([29-weapons](docs/29-weapons.md)).
- [x] ~~[M4] Whether an AI-set target survives the player taking over, and what sets the hero's
  target field (`+0x38`).~~ — this line was **stale**: an earlier round made it moot and this file was
  never ticked. The player's target list sets the turret's target whenever the player's target changes,
  so there is nothing for an AI-set target to survive into
  ([29-weapons](docs/29-weapons.md#the-players-target-reaches-the-turret--read)).
- [x] ~~[M4] How a gun's capacitor refills (the power tick)~~ — closed 2026-09-18: **by the power
  tick alone, 250 ± 31 ms** (`0x1000c756`), the same tick a building runs, out of the carrier's
  batteries. Slot 5 asks for `power × dt` plus `value 1 − charge` and slot 6 banks `level × ask −
  power × dt` (`0x10029a40`, `0x10029a90`); nothing else writes the charge but the parse, a shot and
  a dock's rearm. *Measured*: **all 158 shipped class-2/30 components have `power` = 0**, so the ask
  is the lack exactly and a full tick refills outright; capacitors run 0.1 to 6400 over 133 records,
  and **every one of the 153 with a shot energy holds at least two shots** (2.00 on `e_gun_bl_17` to
  6666.67 on `e_tur_bt_11`), so no gun is ever stopped by charge on a single shot — only on
  sustained fire, at 0.04 to 31.22 a second against a bot battery's 5 to 34.5. The engine was
  already right for a unit with a battery; the stand-in only ever covered a *building's* guns and
  now says so. A neighbouring claim is **corrected**: the charge level of a gun with no capacity is
  set by **nothing**, not by a shot — all three writers of `+0x4c` sit behind a `value 1 > 0` gate,
  so the five such components keep the 0 their constructor leaves
  ([29-weapons](docs/29-weapons.md)).
- [x] ~~[M3] How the camera builds its frame when its up is parallel to the look.~~ — closed
  2026-09-20, **and it is not a fallback but a built frame**. The branch at `Control.dll:0x10023769`
  calls `0x10003ef0`, which takes the look's **horizontal perpendicular**, `(−look y, look x, 0)`, for
  its side — normalised where the look's z is non-zero and left alone where it is — and `look × side`
  for its up, with the bottom row (0, 0, 0, 1). Two axis cases take a stored triple instead, the world x
  axis where the look's x is exactly zero and the world y axis where its y is, **dropping the sign the
  perpendicular would carry**, so half the looks on each axis get a frame rolled 180°. The test is read
  too: `g_FastProc` slot `+0x98` normalises the side in place and returns its old length, compared
  against **0.0** — an exactly zero cross, not a near-parallel pair. The stand-in's measured half holds
  and is now in `verify`: posing every camera's control points over every frame its own and its
  turret's channels play, the look's closest approach to its up over the **63** camera components is
  **9.99°** (`e_tur_lt_01`), 54 no nearer than 10° and the two animals' cameras square at 90°. The
  engine took an arbitrary orthonormal pair; fixed and pinned ([30-turrets](docs/30-turrets.md)).
- [x] ~~`e_gun_bl_03` and `e_gun_tl_02` carry a follower and no gun: are they never fitted, or does it
  pair with a later gun?~~ — closed 2026-09-18: never fitted. Both are on 0 of the 458 assemblies and are
  two of the four `e_gun_*` absent from the 395-entry part list every `.trf` carries and from
  `objects.dlb`; the install names them nowhere but `objects.rlb`. Their controllers hold one channel and
  no components, and their `.cpt` is empty, so there is no muzzle either. Over all 458 assemblies no
  follower is left without a gun ([29-weapons](docs/29-weapons.md)).
- [x] ~~The AI fight module~~ — closed 2026-09-19; its two aim factors and tasks 2, 3 and 5 were
  **stale** (`1 − θ × d ÷ R`; go, attack and search). **The lead is the turret's fastest gun.** The
  turret record's `+0x1c` is an index into its own list of 88-byte gun records, and the refresh
  (`0x1001c1a0`) zeroes it and keeps the index whose gun record `+0x14` — property `0x54`, the round's
  top speed — is **strictly greatest** (`0x1001c37d`–`0x1001c398`, re-derived by the coordinator), so
  ties keep the first fitted; a third copy of the loop at `0x1001bb30` is dead code, with no call, no
  jump and no vtable entry in the module. *Measured* over the 458 assemblies: 407 turret records, 312
  with two guns or more, **160 carrying guns of differing round speeds**, 85 where the fastest is not
  the first fitted, and **91 led by a 10,000 m/s beam** — which is no lead at all, so the missiles
  beside a laser are fired at where the target stands. **`MBehaviour+0x614` is the unit's live forward
  top speed in m/s**, so the bar is 0.5 m/s; *measured*, **19 of the 382** assemblies with a root
  controller are authored below it, all at 0.2, and they are exactly the fixed gun towers and the two
  practice targets — units, not buildings, so nothing lifts their bar and **an AI tower fires at 0.45
  where a warbot needs 0.85**. **The nibble is the object id's class**: `iron3d.dll` switches on
  `(id >> 24) & 0xf` and hands class 3 to the level's building list and class 4 to its unit list, so a
  winged SSM is held for **buildings** (*derived*) — which is what a 45–60 m blast at 700 m is for. The
  damage bar is `> 10,000` and 3 of the 62 `e_gun_*` with a round clear it, the next gun down doing
  3,000. Negative with control: the nibble is in **no shipped file** — of the 864 objects the
  **29** missions place it is 0 on the 463 owned and `0xf` on the 401 scenery, never 3. (*Corrected
  2026-09-20*: this line said "the 33 missions". The install has **33 maps but 29 missions** —
  `gamedir.maps()` returns 33 and `gamedir.missions()` 29 — and it is the 29 that place objects. The
  864 and its split are unaffected; only the attribution was wrong.)
  ([29-weapons](docs/29-weapons.md)).
- [x] ~~An AI turret's state word while it fights (`0x200` or `0x400`).~~ — this line was **stale**:
  it is `0x400`, set on every aiming pass, so the turret's unguided guns keep the target and the range
  gate ([29-weapons](docs/29-weapons.md#how-the-ai-fires--read)). Whether the relink runs between that
  set and the gun's shot is still not followed.
- [x] ~~What the landscape face record's `+0x34` points at — the vector the outer camera adds 0.75 of
  to the point its line meets~~ — closed 2026-09-20, and it **reinstates a claim this page recorded as
  withdrawn on 2026-09-19**. That round published the normal reading, was sent back by a spot-check,
  repaired both broken links and then withdrew the claim anyway on the ground that **nothing writes
  `+0x34`**. That withdrawal was wrong. The landscape's constructor (`Terrain.dll:0x100166a0`) writes
  `this_outer + 0x7c48` into `this_outer + 0x170` (`0x10018dd0`–`0x10018ddb`) and installs the interface
  vtable `0x1009a3c4` at `this_outer + 0x13c` (`0x10016e9b`) — so `+0x170` **is** the interface's
  `+0x34` and `+0x7c48` **is** its `+0x7b0c`, the very buffer the flag-2 block fills from the face
  record's three `int16` at `+0x14`, scaled by 1/32767. **Re-read by the coordinator** on one continuous
  decode from the constructor's entry to its first `ret` at `0x100190f9`, which contains both sites; a
  second install of the same vtable at `+0x138` (`0x10019463`) lies *past* that `ret`, in another class,
  and does not govern this object. **The sweep missed it because a multi-interface object's constructor
  writes the pair in the most-derived object's coordinates, not the interface's** — a bullet is now in
  [09-method](docs/09-method.md)'s *Searches that do not discriminate*. So the camera stands at the
  point met **plus 0.75 × the struck face's own normal**, 0.75 m out along the surface; the engine had
  brought it back along its own line, and is fixed and pinned. Read for the landscape; the same field on
  a mesh object was not traced. **The outer view's flag `0x20` closes with it**: it is the camera's
  **infrared**, read off the game's view (slot 20) and or-ed into the outer view (slot 13) at
  `0x1003895c`–`0x10038990`, set and flipped by `CIS_INFRARED_ON/_OFF/_INV`
  ([30-turrets](docs/30-turrets.md)).
- [x] ~~What writes the design row's node field, inside the fits' loop over a part's nodes~~ — closed
  2026-09-20; `+0x00` and `+0x24` are **narrowed**. The fits' loop is
  `iron3d.dll:0x10052809`–`0x1005290c`: the catalogue is asked how many nodes the part has, the counter
  runs **1 up to that count**, each node's socket label is fetched and a node that answers none is
  passed over — and the counter itself is written into the prototype record at the offset the list
  insert copies to the row's `+0x04`. So **the field is the node's own index, not an ordinal, and node 0
  never opens a row**. *Measured* over the 33 `.dat` files the designer writes: all **138** external
  attachments are node ≥ 1, all 138 name a node carrying a stream-10 label, all 138 rise in node order
  under their host, and **only 60 of the 138** would also fit an ordinal over the host's sockets — which
  is what separates the two readings. `+0x24` **names the part the row holds** and `+0x00` **the part
  whose socket the row is**, both filled from a three-dword request the project answers (query 6 with
  `0x80000020`); what that handle counts was not read. The unit writer's test is confirmed exactly: a
  row is emitted only where `+0x20` ≠ −1 **and `+0x00` == 0**. Left open with it: the turret add appends
  socket rows to the **fourth** row array while the writer reads the **third**, so which tab each array
  is stays unread. (~~The Large transport's second slot and the Large builder's module socket~~ — closed
  2026-09-18: a socket's kind is its **stream-10 label**, not its node name, and the gun page is the
  label's last two letters after `e_gun_`. The Large transport has one socket, a cannon one; the Large
  builder's module is `Base_LU_02` and its cannon `Base_LU_01`, reversing the rule the small and medium
  builders gave. `objects.dlb`'s hanger rows agree on 48 of 55 turrets; its free text does not and was
  the source of "two battle slots".) ([37-designer](docs/37-designer.md),
  [38-designs](docs/38-designs.md))
- [x] ~~What stops the player building the six free turrets, and which state bits the design screen
  tests~~ — closed 2026-09-18: the screen tests `RESEARCHED` and `IN_TREE` and **not** `AVAILABLE`, which
  it parses and never reads. Five of the six are out of the tree in all **17** trees a player clan loads —
  not 11; `full.trf` is loaded by nobody and `data.trf` is the player's in nine missions. The hero's
  turret passes both bits in six of the 17 and is stopped one level further out: its page never opens,
  because `r_h` is not a chassis-page prefix at any factory grade. Zero cost is not the gate
  ([30-turrets](docs/30-turrets.md), [38-designs](docs/38-designs.md)).
- [x] ~~What carries a winged SSM over the ground to a building~~ — raised and closed 2026-10-01: nothing
  carries it. The engine read two rules short and fired from too close.
  - The raider in "Let's Play - Parkan: Iron Strategy, Part 6" (-yNnsqudMzw, 13:49.5–13:54.2) stands
    345 to 292 m off C03 M02's Small Bunker, and its winged SSM lands on the bunker.
  - **An attack never nears a building** (*read*). The attack tick asks for the nearest contact only
    while the flag at `+0x64` is set (`Behavior.dll:0x100272c9`), and a building target's move clears
    it (`0x1002777d`), so the fire control holds the building from the first pick on. The engine let
    the attacker near it until within 200 of its point, where the radar, which never lists a building,
    gave it nothing to fire on.
  - **A round of more than 10,000 is held for a building**
    ([29-weapons](docs/29-weapons.md#how-the-ai-fires--read), read long since and not in the engine),
    so the raider stopped spending its missiles on the hero it passed.
  - With both, the gate opens at 500 m, the 7 s lock runs out short of 400, and the missile lands with
    the raider 289 m off, as the recording has it. The flight itself, straight at the node-sphere
    centre, needed nothing.
  - The remainder, whether a blast spares the object that fired it, stays open
    ([31-packages](docs/31-packages.md), [29-weapons](docs/29-weapons.md)).
- [x] ~~Whether a blast spares the object that fired it~~ — closed 2026-10-01, **read**.
  - A hit carries its firer's object id at `+0x18`: the exploding object's property `0x7f`, or the
    object's own id when that is 0 (`Control.dll:0x10011766`–`0x1001177b`); the gun sets the property
    on the round it has just made (`0x1002a398`–`0x1002a3ed`).
  - On the target, `ILifeSystem` slot 8 (`0x1000ebc0`) runs a blast in this order: message `0x19`
    with the firer; a building's door; the shield step; nothing left; invulnerable; **the target's
    object id against the hit's `+0x18`, end if equal** (`0x1000ed4e`–`0x1000ed59`); the nodes. A
    direct hit on a node makes the same compare (`0x1000ee0c`).
  - So it is **id against id, with no clan**: friendly fire is on. Every node of the firer is spared,
    its shield still pays, and it is told of the hit. A dying object's own explosion names the object
    itself. **A hit whose firer no longer answers its id is worth 0** (`0x10011783`–`0x100117b8`), so
    a round in the air when its firer is deleted is a dud.
  - *Measured*: 35 of the 66 readable rounds are blasts, 2 to 60 m; no minimum range exists in the
    data; outside the rounds 1 node of 1,828 explodes as a blast, on a turret no shipped design
    carries.
  - The engine's blast reached its firer as it did any object: on C03 M02 the raider's winged SSM
    bursting 25 m off killed it. It now leaves 7,643 of 7,643 points, and the same round fired by the
    bunker onto the same spot leaves 0 ([26-damage](docs/26-damage.md)).
- [x] ~~What killed the hero in C02 M04's valley Light Tower~~ — **narrowed** 2026-10-01: a blast is
  stopped by nothing but distance. The hit queue (`Control.dll:0x10012ce0`) puts one sphere round the
  tick's blasts and asks `CWorld` slot 3 (`Terrain.dll:0x10025f40`) with class mask `0x61c`; the
  collector walks the object tree and takes an object on its class bit and its sphere alone, and
  asks every object, taken or not, for its children near the sphere. **No line or occlusion test
  exists**: none of the queue walk, slot 8, the shield step or the falloff calls the world's segment
  query. The control is the same enumeration over all of `Control.dll`, which finds slot 7 once, at
  the gun's sight ray. So a hero in a pod room is reached through the building's own children. The
  tower's own death is ruled out as the killer, every node of a building naming a kind-1 explosion.
  Which gun's blast it was stays queued ([26-damage](docs/26-damage.md#what-a-blast-reaches)).
- [x] ~~How the camera applies the colour it is handed in mode 2~~ — closed 2026-10-01 but for one
  figure, **read**, **measured** and **seen**. Raised by two frames of "Let's Play - Parkan: Iron
  Strategy, Part 6.5" (8:13.0, 8:15.5) in which a building's placement ghost is one flat colour, red
  (253, 23, 42) then green (129, 253, 42): the ghost's colour over (129, 23, 42), C03 M02's scene
  colour at that hour.
  - **Mode 2 makes the camera's colour every batch's self-light and takes its texture away.**
    `CShade`'s mesh draw (`Terrain.dll:0x10044ea0`) asks the camera its mode per draw item
    (`0x1004570f`); on 2 it writes the colour over the item's material ambient
    (`0x1004574c`–`0x1004576e`) and sets both textures to none.
  - **The scene colour is a floor, not an addend.** The shade's lighter calls `g_FastProc` `+0x50`
    (`0x1004f225`–`0x1004f23d`) with the vertex's light sum, the material ambient and the scene
    colour, and all four builds add the first two and keep the larger of that and the third, channel
    by channel (`Ngi32.dll:0x100248a0`, `0x1001ffc0`, `0x1001bdd0`, `0x1001d980`: `addps`, `maxps`).
    [11-effects](docs/11-effects.md) and the engine's world path had the three summed.
  - **A panel's view is lit by the shown unit's own two lights and nothing else.** Draw flag `0x200`
    gives the shade's gather flags 1 and 4 (`Terrain.dll:0x100478c0`): the object's own light manager
    alone, and only lights flagged `0x4000000`. A mesh makes two such at set-up
    (`AniMesh.dll:0x100070c9`–`0x1000723b`): (−1, 1, −1) in grey 0.25 and (1, −1, 1) at 0.35 of it.
    **The ghost is flat because its `0x5f0` lacks `0x200`.**
  - *Measured*: an unlit patch of the hero's own figure at 1:52.5 of Part 6 reads (151, 127, 57)
    under a scene colour of (156, 40, 59) — green is the node's 0.5 alone, where an addend would give
    167. Docs/35's old (39, 162, 41) on C00 M01 is (0, 128, 0) over that mission's (40, 40, 40).
  - The engine drew both lifted toward white with no scene colour; the same patch is now (154, 127,
    58) and the dummy (40, 163, 40) ([35-hud](docs/35-hud.md), [32-builder](docs/32-builder.md)).
    Queued: what lights a unit of more than one part, which the recordings show about twice as
    bright as one pair gives.

## Damage, sensors and ownership

[26-damage](docs/26-damage.md#not-established), [25-sensors](docs/25-sensors.md#not-established),
[27-ownership](docs/27-ownership.md#not-established)

- [x] ~~[M4] The hit test's point-in-triangle test (`0x10011090`) and the landscape's own cell
  size~~ — closed 2026-09-18, and the engine was wrong twice. The test borrows both halves from
  `Ngi32.dll`. The plane half is one-sided with **no epsilon** — `n·v < 0`, `d(p₀) ≥ 0`, `d(p₁) ≤ 0`,
  each against exactly 0.0 — and the containment half is **`mrnPointInPoly`** (ordinal 202,
  `0x10001e60`): **not barycentric** but a dominant-axis 2D projection, whose axis comes from two
  comparisons rather than a maximum, with three edge cross products each multiplied by the normal's
  component on the dropped axis and required `≥ 0`, no epsilon, a point on an edge inside. That
  product is the full dot with the normal up to a positive factor, so it is an edge test signed by
  the face's **own stored normal**, not by its winding — and the two differ on **8 of 241887** mesh
  faces. New with it: **`0x1001110c` makes a batch flagged bit 1 two-sided**, rerunning the plane
  test with the segment reversed, on **1477 of 15153 shipped batches** — `static.rlb` 946,
  `fortif.rlb` 440, `turrets.rlb` 31, `weapon.rlb` 28 and 32 across three more archives, the trees
  and the buildings (counted here independently). And the landscape's cell is **the map's extent
  over the grid its own file states**: cells across from the square stream's NRes link-count field,
  cells down from its element count over that (`0x100178e6`–`0x1001794f`), the cell itself from cell
  0's box. *Measured on all 33 maps*: the stated grid equals the corner-derived grid **33 of 33**
  (16 × 16 on 28, 8 × 8 on 5), cell 0's box equals extent ÷ grid **33 of 33**, and the cell runs
  **49.90 units (map 41) to 311.28 (SC_3)** over 22 distinct values — never the engine's flat 16 m.
  Both fixed and tested ([26-damage](docs/26-damage.md), [03-terrain](docs/03-terrain.md)).
- [x] ~~[M4] Which node flag makes a node vital (is AniMesh query `0xe` the mesh node's flags?)~~ —
  closed 2026-09-18: **yes, and the bit is `0x200`**, which is what the engine already had. Query `0xe`
  reaches the mesh node's own stream-1 record — pointer at the runtime record's `+0x12c`, indexed by its
  `+8` at a stride of 38, the first `uint16` — where query `0xa`, four ids along, answers the *runtime*
  word instead; the life loader tests bit 9 of the former. Measured: 90 of 1845 mesh nodes on 9 of the
  435 meshes, and a coherent 90 — every one a geometry-bearing segment of an articulated limb or body
  chain (the hero's body and legs, the animals' necks, heads, tails and wings, the hero turret's two
  arms), 0 of `fortif.rlb`'s 273, and none on a wheeled or tracked chassis. Named against its neighbour
  as asked: mask `0x10` is broad, 861 of 1845 over all nine archives, where `0x200` is narrow; they
  coincide on 8, each the leaf of a flagged chain. Why the bit falls on those nine models and not on the
  other walkers' legs is not established ([26-damage](docs/26-damage.md)).
- [x] ~~[M4] Whether vegetation and rock carry node life~~ — closed 2026-09-18: **they do**, and the
  engine was wrong. A placed tree or stone goes into the game as agent type 10, and the agent build
  gives every agent a control system and queries its `ILifeSystem` before any branch on the kind; the
  life loader then branches on kind 10 to set the low-life mark at 0.3 of the total rather than 0.2,
  which is only reachable if scenery has a life system at all. Measured: all 81 `STAT` records name a
  `.ndp` and a `.ctl`, 123 node rows, 1 to 1500000 hit points, **none 0**, with `explode_tree` and
  `explode_stone` to play. The control is the same query by tag: 63 of 63 `BTLU`, 146 of 146 `EXTO`,
  **0 of 34 `FORT`**. And scenery is the one thing built at its placement scale, the tick rescaling
  every node's life by the three factors multiplied, so Mission 01's `s_tree_04` at 3 has a trunk of
  81000 rather than 3000 and Mission 02's `s_stone_10` at 21 holds 4630500000. Fixed and tested
  ([26-damage](docs/26-damage.md), [04-missions](docs/04-missions.md)).
- [x] ~~[M4] Whether a round's ground test strikes the water surface~~ — closed 2026-09-18: **it
  passes through, and the engine was right for a reason it did not have.** A round's ground query
  (`Control.dll:0x1001d9d0`) is eight dwords excluding world flags `0x208` and class `0x24`, which
  `GetFirstIntersectedFace` folds inline into the landscape's own mask (`Terrain.dll:0x100209a9` for
  the flags, `0x100208e3` for the class): the flags word's `0x20`/`0x80` and the surface word's
  `0x02`/`0x01`. Surface `0x02` is the water sheet, on exactly **3630** faces; the other three are
  on **0, 0 and 0 of 275882** — the control being the same scan on the same two fields, which does
  find the 3630. So a shot into a lake splashes on the bed, while the sight ray, which excludes
  nothing, stops on the surface ([26-damage](docs/26-damage.md)).
- [x] ~~[M4] What a dead unit leaves (wreck, damage stages), and what `iron3d.dll` does with owner word
  `0xfffe` (37 compares).~~ — closed 2026-09-20, **the wreck half stale and the `0xfffe` half read**.
  The wreck was already in [26-damage](docs/26-damage.md): a unit is deleted the controller's `+92` ms
  after death, a building becomes a shell, and stages are `N − ceil(N·life/max)` drawn from the mesh's
  slot index. What was left was that doc's own *guess* that a radar passes over wrecks, and the owner
  word is what settles it: it lives at the control system's `+0x550`, reaches the game through
  `ILifeSystem` slot 11 and `IGameObject` slot 17, and has two reserved values — the constructor writes
  `0xffff` (`0x100070c7`) and **death writes `0xfffe`** from two inlined copies (`0x10011098`,
  `0x10003354`). It is **one-way**: the setter (`0x1000f1f0`) refuses to write when the word already
  reads `0xfffe` and refuses to write `0xfffe` itself. The **37 compares are one idiom, not 37
  decisions** — 30 through slot 11, 3 through slot 17, 3 from the record's cached `+0x24`, 1 from a
  target-list contact — guarding lists and pickers (13), the HUD (6), the target list (9), the view and
  the player's own unit (6), two component tests and a rebind. **7 of the 37 stand within a dozen
  instructions of an `imul …, 0x68`**, the clan-record stride, which is what the value means: `0xfffe`
  is *not a clan index*. Control, *measured*: **0 of the 864** placed objects of the shipped missions
  carries `0xfffe` or `0xffff` in a clan word, against ClanIDs 0×124, 1×220, 2×69, 3×37, 4×11, 5×2 — a
  mission cannot author it. The engine was already right
  ([27-ownership](docs/27-ownership.md), [26-damage](docs/26-damage.md)).
- [x] ~~The `.exp` record's two 1.0 floats, for which no reader was found~~ — closed 2026-09-18 as a
  **negative with its control**: the thread the doc named ends, and nothing reads them. The record is
  792 bytes on all 144, exactly `0x18 + 12 × 64`, so nothing is unparsed. Its bytes reach one pointer,
  the `.exp` cache, whose only fetch is the known one — and no module in the install holds the string
  `.exp` besides. That pointer is stored at offset 0 of the hit record, and the hit's `+0` is
  dereferenced in four functions, each reading the first dword only, the kind. The control is that the
  same enumeration finds four of the six scalars and all twelve names at named addresses: it is not
  blind to two adjacent floats between the radius and the placement word. What they are *for* is left
  as a guess the layout supports and the code does not settle ([26-damage](docs/26-damage.md)).
- [x] ~~What the player's map and radar display show (`iron3d.dll:0x1003fb90` and `0x10073550`).~~ —
  this line was **stale**, found by the coordinator's own check rather than spent on an agent.
  [35-hud](docs/35-hud.md) reads out both: the radar's seven-step draw order at `0x1003fb90`, its view
  wedge as the camera's 1.3 rad field drawn always pointing up, the disc turning by the `atan2` of the
  camera matrix's first column, a contact drawn at `β − θ + π/2`, and the satellite map's load at
  `0x10073550` — minimap, `exit_icon`, `map_compass_icon` and string 5074 — with its `MAP_ALPHA` of 128
  and its ] and [ keys. What remains is only the **clan contact list** the maps mark by, which is its
  own line below and its own M13 stand-in row.
- [x] ~~What moves a SuperAI's attitude from one relation band to another.~~ — closed 2026-09-20 as a
  **positive and a controlled negative together**, and the engine gained a model it never had. The
  record is 16 bytes × 64 at the SuperAI's `+0x43c` — attitude, a decrease, an increase, the word. **The
  earlier search failed because the compiler folds the base into the index**: the write is `add eax,
  0x44; shl eax, 4`, so neither `0x440` nor `0x444` is a displacement anywhere. Sweeping all sixteen
  modules for the eight object- and interface-relative displacements *and* enumerating all 80 `shl reg,
  4` sites in `ai.dll` leaves exactly one non-zero writer: **`ai.dll:0x10001fe0`, slot 13 of vtable
  `0x100341b8`**, which adds a fixed **0.004** (`0x10034284`) to the *decrease*; its caller
  (`Behavior.dll:0x1000658c`) pushes an amount of its own, 1.0, that the slot never reads (`ret 0xc`).
  **Re-read by the coordinator**, instruction by instruction, including the bounds check against 64 and
  the constant. **The increase field is written only to 0** — control: the same sweep does find the
  writers of all three neighbouring fields. So **a relation can only ever fall**: 13 hits lose an ally,
  42 inside one takt make a neutral clan hostile, and a hostile one can never rise. Implemented in a new
  `relations` module with 8 unit tests and an install test
  ([25-sensors](docs/25-sensors.md), [31-packages](docs/31-packages.md)).
- [x] ~~The game view's states 1, 3, 4, 5 and 6, and what pods open on a generator, mine, storage or
  Outpost; also what `0x10033e40` refuses on a tower.~~ — closed 2026-09-20; the middle clause was
  already struck in [27-ownership](docs/27-ownership.md) (no screen: those two calls select the building
  and make it current). The switch is the **mode stack's**, an 8 × 8 handler table at `0x10104b18`
  indexed *front × 8 + new*, **40 of whose 64 cells are filled** — there is no way into a building's
  screen from mode 1, and none from mode 5 to 6. Modes 0–4 and 7 were already in docs/39 and docs/40;
  **mode 5** (`0x10064430`) turns the outer camera off, clears the selection, **hands the hero back**,
  makes the building current and sets the level's view-state word to **1, the cockpit**, so the page
  draws over the ordinary view; **mode 6** (`0x10063fd0`) clears the driven unit, sets the word to
  **6** — of the module's 39 calls of `0x100a4f90` only the four handlers into mode 6 pass 6 — a state
  the per-frame camera update has no case for, and sends the tower's object `IGameObject` slot 13 with
  **(6, 7, 1)**, byte for byte the message the takeover sends a bot. **So entering a tower drives its
  guns exactly as boarding drives a bot's.** **`0x10033e40`** refuses on: owner word `0xfffe`; no
  `IControl`; no class-1 item; or that item's node life ≤ 0 — confirming docs/41's reading, and it is
  the third copy of a predicate the boarding and builder-beam tests also use. *Measured*: **0 of the
  30** `fortif.rlb` building controllers carries a class-1 item, so it is always a fitted part; **24 of
  the 68** building assemblies carry one — the 15 bunkers and the **9 towers** — and on all 24 it is a
  turret with life (3,500 on the 6 medium, 6,000 on the 3 large). **All 9 shipped towers can be entered
  until their turret is shot off** ([27-ownership](docs/27-ownership.md)).
- [x] ~~The other four writers of a unit record's `+0xa2`.~~ — this line was **stale**, found by the
  coordinator's own check rather than spent on an agent: it is read in
  [39-boarding](docs/39-boarding.md#the-other-writers-of-0xa2--read) as a six-row table — the record's
  constructor and its binding to an object (0), the briefing's start (0) and end (1), and the help
  screen opening (0) and closing (1) — and was already struck where it stands in
  [27-ownership](docs/27-ownership.md).

## AI, scripts, packages and economy

[15-behaviour](docs/15-behaviour.md#what-is-not-read-here), [31-packages](docs/31-packages.md#not-established),
[23-economy](docs/23-economy.md#not-established), [32-builder](docs/32-builder.md#not-established)

- [x] ~~What the two floats an object's strength is made of are (`IControl` property `0x36` and
  interface `0x204`'s `+4`), and so what a `TAKE_BY_HITS` amount is worth~~ — closed 2026-09-20, **and
  the engine was wrong**. The formula at `ai.dll:0x1000fc70` is **`(guns + 0.8) × hit points × 1e-5`**,
  a five-instruction leaf — *re-read by the coordinator*, with the 0.8 at `0x10034600` and the 1e-5 at
  `0x100345f8`. This page could not name the two floats because **the property table it looked in is
  the wrong one**: the control system's base interface answers neither 38 nor 54, both falling to a stub
  that returns 0 — but `LoadControlSystem` runs a **derived** constructor for every agent kind but 9,
  whose `ILifeSystem` slot 5 is a **second dispatcher** (`0x1000e6c0`) covering ids 38–179. So
  **property 54 is the life an object's nodes *could* have plus the full shield**, and **property 38 the
  life they have *left* plus the shield now**. The second float, variable `0x204`'s `+4`, **is the
  unit's guns**; *which* two authored gun figures it divides is the one piece left open. A bonus falls
  out: the cached and live forms measure different things — a place is held against you by what its
  defenders have **left**, while your own group is worth what it would be **at full** — which also names
  [31-packages](docs/31-packages.md)'s unresolved patrol-score `a`, `b` and `c`. *Measured*: the 458
  assemblies sum 0–1,084,514 `.ndp` hit points (median 8,774); all 19 hero assemblies sum 7,362 and the
  largest bunker 66,010, so unarmed they are worth **0.0589** and **0.528** — an amount of 25 is 424
  hero hulls of armour, so **a two-digit `TAKE_BY_HITS` is a demand for guns**, not for armour. Of the
  74 such calls, 30 come from a problem's third parameter and 27 from a formula over the two protect
  variables. The engine counted objects; it now sums a priced strength, though nothing prices one yet,
  so the stand-in narrows to the gun total ([15-behaviour](docs/15-behaviour.md)). **Not taken**: the
  problem's action record and SuperAI `+0x40c`, the budget having gone to the two floats.
- [x] ~~A problem's two raise numbers, which handler runs when, and who writes `dCurrentProblem` and
  `dCurrentSender`.~~ — closed 2026-09-20. One clan takt (`ai.dll:0x10001780`, 7000 + rand % 1000 ms)
  walks the problem list **four times**: the drain, the `_Continue` pass (every `ST_SOLVING` problem),
  `Problems<n>`, then the `_Start` pass — which takes the **largest weight** among problems neither
  solving nor solved and is **repeated** while any handler leaves its problem unstarted, so one takt
  drains the list from the heaviest down. The two raise numbers are **a life counter and what each takt
  takes off it**, with the value a re-raise reloads by between them; at 0 or below the problem retires.
  A raise resolves `<code>_Start` and `<code>_Continue` **by name** and is abandoned when either is
  missing. `dCurrentProblem` has **two** setter sites and `dCurrentSender` one, out of 20 loads of the
  two pointers. *Measured* over **176 raises**: 72 carry 25/24 (two takts), 37 carry 5/2 (three), 28
  carry 1/0 (never expire), 26 carry a drain of 1 — and **21 raises across eight scripts are dead**
  because the script never defines the handler pair, so **four enemy clans ask for warbots they will
  never plan for**. Worth recording how the agent found its own error: its first sweep missed one
  setter by skipping `ebp` operands, exactly the trap this file's method notes warn about, and the doc
  records that rather than quietly fixing it. The engine raises no AI problem at all, as its stand-in
  already said ([15-behaviour](docs/15-behaviour.md)).
- [x] ~~Channel 2 of the message callback (function 57), and the count function 69 stores~~ — closed
  2026-09-18. **Channel 2 runs a line of the mission's own `object script` block** in `mission.cfg`:
  the callback takes the channel first, function 57 passes 2, and that arm formats its one value into
  the key `script%d`, looks it up and dispatches the first token against a ten-entry table which is
  **the game's debug console**, names and help text intact — `truth kill bkill cls summon ? create
  delete bcreate death`. Measured: 6 of the 29 shipped `mission.cfg` files carry a non-empty block, 20
  lines between them, three of the ten names used; 14 of 14 calls of function 57 name a line their own
  mission declares and 0 of 14 one it has not got. Control: 244 calls on channel 0. **Function 69's
  count is the AI design pick's spread**: the pick scores every design, sorts it best first, and then
  takes not the best but the candidate at a random index within that many of the top. All 7 call sites
  set it in `Init` from `fDifficulty` — an easy game makes the enemy clan build worse designs. The
  engine answers 0 to both, which is indistinguishable here: 0 of the 21 call sites names a destination
  ([15-behaviour](docs/15-behaviour.md)).
- [x] ~~Whether any script depends on a constant landing inside a false block~~ — closed 2026-09-18: no,
  and the executor says why — the constant arm is the only one of the eight that does not test the
  condition byte first. All 63 in-block constants write one of five scratch variables; over the 121
  (constant, enclosing `if`) pairs, 88 are overwritten before any read on every path and the other 33
  reach the end of their handler, where every engine entry point writes before reading. None of the five
  names appears in `ai.dll`, so the engine cannot see them either
  ([15-behaviour](docs/15-behaviour.md)).
- [x] ~~Five labels no goto aims at, and the label that sits inside a block~~ — closed 2026-09-18: five
  plus one, and **the one is not an anomaly**. The five are one label five times over —
  `PBM_BUILDING_PROTECT_Continue` node 19 of 20 — left behind because those five handlers write both
  exits as `return` and hold no goto at all; the same handler ships in two other scripts at 19 nodes with
  no label. The sixth, `c5m1p:PBM_BASE_DEFENCE_Start:9`, **is** aimed at, by the goto at node 3, and sits
  inside a block only because the handler's whole body is one `if`
  ([15-behaviour](docs/15-behaviour.md)).
- [x] ~~The `.fml` operators the corpus never uses~~ — closed 2026-09-18: the corpus uses **3 of 13**,
  `+`, `-` and `*`. All thirteen arms are now read rather than named, and seven were implemented wrongly
  here: `/` answers 0 when the divisor is 0 rather than dividing, `&`/`|`/`!` truncate through `_ftol`,
  and `N`/`S`/`B` are a clamped ramp, the positive part and a strictly-positive test. Fixed. The doc's
  table address was the dead copy; the live one is `0x10047c70` with its count at `0x10047c68`
  ([15-behaviour](docs/15-behaviour.md)).
- [x] ~~A fire-control request's 0.5, and what sets `+0x5c` and `+0x60` to lock a unit's fire mode.~~
  — closed 2026-09-20 as a **double negative with controls**. The **0.5** is the fire control's own
  `+0x30`, set by the constructor and rewritten with the same value by all **20** request sites, and
  **nothing reads it**; control, the same sweep over the class's code finds two readers of `+0x28` and
  two of `+0x2c`. **Neither lock can ever be set.** `+0x5c`'s only writer, `0x10023fd0`, **has no caller
  anywhere and sits in no vtable** — *re-checked by the coordinator*: sweeping all 22 modules for that
  address finds three hits and every one is an **image-base collision**, since every module is based at
  `0x10000000` — a `push offset "CICLS_SIMPLE"` in `World3D.dll` (the string really is at that address
  there) and two `je`/`jle` targets inside `iron3d.dll`. `+0x60` is set and cleared only inside
  `0x10025a00`, whose one caller is reached only when one of the two is **already** set, and the "is it
  locked" predicate has no caller either. Control: the same call scan finds `0x10025a00`'s one caller.
  So **0 of the 20 requests are ever refused**, and since the twenty cover every task in the mode table
  there is **no order in any shipped mission that could be turned away**. The engine has no fire-mode
  lock to get wrong ([31-packages](docs/31-packages.md)).
- [x] ~~Who sends `MBehaviour` messages `0x19` and `0x1a` (the retaliations).~~ — this line was
  **stale**, caught by the coordinator's own check rather than spent on an agent: it is already read and
  struck where it stands in [31-packages](docs/31-packages.md) — every hit's first step sends `0x19`
  with the firer's id, and so does a round passing through a shield, while **`0x1a` is never sent**, on
  a search of every binary.
- [x] ~~Whether a clan's AI re-orders a build refused for want of a mind~~ — closed 2026-09-18: **it
  drops it, at every level**, and the page's derived claim was backwards. `op5` is `!=`, not `==`, so
  all 9 `ORDER_BUILDING_CONSTRUCT` sites — one per script, every one in `PBM_ROBOT_NEEDED_Start` — mark
  the problem *solved* and return when the order was **not** taken, and nothing in the handler orders
  again. 8 of the 9 never try without a mind at all, opening on `if dFreeMindNumber <= 0`. Because the
  problem is marked solved rather than left standing, the duplicate test no longer blocks it, and the
  108 raises of `PBM_ROBOT_NEEDED` across 13 scripts bring it back when something wants a robot. The
  engine's factory already refuses correctly and queues nothing ([23-economy](docs/23-economy.md)).
- [x] ~~What a mine's ToMine does to its output~~ — closed 2026-09-18: it **bounds** the output and
  never scales it — the rate is `Mine_OrePerSecond × KPD` whatever the lode holds — but **the bound
  bites at half**. A takt digs, takes `ToMine − total` before either changes, then adds the dig to the
  total *and* takes the same dig off `ToMine`, ending when that pre-dig gap was no more than the dig. So
  the two meet half way and a lode of 600 yields 350. Measured over all 29 missions: 15 placed mines in
  8 missions, each with exactly one lode within 250, none carrying less than twice the 500 a mine may
  hold — so no shipped mine is ever bounded by its lode and the fix is fidelity, not behaviour. Control:
  6 of the 95 other buildings in those missions also stand within 250 of a lode. Fixed, with the final
  dig now banked on the ending takt ([23-economy](docs/23-economy.md)).
- [x] ~~**Nothing raises or runs an AI problem, so no clan ever builds a warbot or sends one to take a
  building.**~~ — closed 2026-09-21 as **M18**, the milestone it was called. `parkan-sim/src/planner.rs`
  is the problem list a SuperAI keeps — weight, life counter and drain, three parameters, attached
  units, groups and action records — and each clan's takt now walks it four times as the disassembly
  has it: the drain, the `_Continue` pass over every `ST_SOLVING` problem, `Problems<n>`, then the
  `_Start` pass from the heaviest weight down. Thirty-six more function slots answer it, `fn2`, `fn6`,
  `fn7`, `fn8`, `fn11`, `fn12`, `fn13`, `fn14`, `fn24`, `fn25`, `fn27`, `fn28`, `fn29`, `fn35`–`fn38`,
  `fn43`–`fn51`, `fn61`–`fn70` and `fn72` among them, and a raise resolves its `_Start`/`_Continue`
  pair by the **name** of its code's variable and is abandoned when either is missing. An
  `ORDER_BUILDING_CONSTRUCT` now reaches whichever clan owns the plant, out of that clan's own design
  store (`UNITS\UNITS\AI\`, 77 designs ranked by the order's `SELECT_*` over function 69's spread),
  and a building's own SuperAI runs `Fort_Captured` for what it has just lost. *Measured* on C02 M03:
  the enemy's Large Factory turns out its first *LSW-X Warrior* at 60 s and two more by 80 s, then
  stands idle with its clan's four minds held; the hero takes its Generator at 1.3 s and it answers
  with `PBM_BUILDING_INF_CAPTURE` at weight 0.74, a capture order at 7.5 s and the Generator back at
  58 s. Three checks in `engine/crates/parkan-world/tests/install/campaign.rs`
  ([15-behaviour](docs/15-behaviour.md#what-the-functions-do),
  [34-progression](docs/34-progression.md#what-the-scripts-ask--read-and-measured-1)).
- [x] ~~**What the design store scores, and what writes `fDifficulty`.**~~ — closed 2026-09-21 from
  play on C02 M03, where the enemy turned out nothing but the largest warbots in the store, for
  nothing. Three things came out of the binary. The **six arms** of the `SELECT_*` jump table
  (`ai.dll:0x10010bbc`) are read one at a time: `SELECT_BEST_WEAPON` scores the record's guns,
  `SELECT_BEST_ARMOR` its hit points, `SELECT_BEST_RANGE` a float the store's own fill never
  writes, `SELECT_FASTEST` its top speed, and **`SELECT_BEST_COMBAT` and `SELECT_SMALLEST` both
  call the strength formula** — so "best combat" is guns over armour and "smallest" is the
  weakest by it, and this page's earlier "ranks by hit points" was wrong. *Measured* over the 59
  warrior designs: by hit points the top five are all large chassis; by strength the top three
  are the size-2 `23_swlk1` class at 228.6 against `AI_LS_10`'s 207.2. **`fDifficulty` is the
  game level** — `ai.dll:0x10005d00` writes a six-float table indexed by it, 0 → 0.0, 1 → 0.5,
  2 → 1.0, from a `CreateSuperAI` argument, keeping the level at the SuperAI's `+0x384`; the
  engine had left the variable at `varset.var`'s declared 0.5, so the player's difficulty
  setting reached no clan. And the **factory-size gate on the pick is dead code**: `0x10006820`
  works out the clan's biggest plant and `or eax, 0xffffffff` at `0x10010af5` throws the answer
  away before the comparison. A fourth thing was the engine's alone: it priced a design by the
  parts the clan had *researched*, and the AI's store is gated by no research, so every large
  design cost 0 ore and 0 power. Fixed; the enemy now builds three small *SSW-X Warriors* it
  pays for ([15-behaviour](docs/15-behaviour.md#what-each-select_-scores--read)).

- [x] ~~**What the console's `create` does with its arguments.**~~ — closed 2026-09-30, **and the
  engine now answers function 57**. The console cuts the line at `(` and `)`, matches the name with
  `_stricmp`, and the handler cuts the fields at commas, spaces and `)` and does nothing unless all six
  are there; every number goes through `atol`, so they are **whole numbers**, and the **sixth is
  converted and dropped** (`iron3d.dll:0x1003d280`–`0x1003d4d4`). The file is `units/auto/<file>`
  (`0x10103ef4`): all **13** files the install's 19 `create` and `bcreate` lines name are in
  `UNITS\AUTO`, of the 23 there. **The third field is not a height but the heading**: `0x10077520`
  builds a turn about z from its cosine and sine, the unit's x axis to (cos *z*, sin *z*), as a placed
  object's `rotation` does, and stands the unit on the level's probe for the highest landscape or
  building surface **plus 2** (`0x10077562`; the loader's placer adds 1, `0x100774a7`). The clan is the
  index into the level's clan table. The id is new (−1 passed, create bit 2 clear), there is no host
  building, and **create flag 8 is set unless the game is the auto-demo** (`0x1007765a`), so the unit
  takes a free mind of its clan and is not made without one. The handler files it on the clan's SuperAI
  as slot 4's **event 1** (`0x1003d560`), the placed unit's event, **inside the call to function 57**, so
  function 31 counts it and `TAKE_ALL_FREE` takes it. C02 Mission 02's `Mission` handler counts `Enm2`'s
  robots in the same run as its three `create`s (`c2m2p` nodes 40–53). *Measured*: every shipped
  `create` finds a mind free at the start, the two `Enm2`s having none placed. The engine files each
  unit with its clan's list during the call and makes it as the run ends; the recording's
  reinforcements now arrive for `Enm2`, and the bonus objective waits for them
  ([15-behaviour](docs/15-behaviour.md#what-the-consoles-create-bcreate-and-death-do--read-and-measured)).
- [x] ~~**What `bcreate` makes.**~~ — closed 2026-09-30. **A building standing finished, for the clan
  the fourth field names, under a new id, set down on the ground whatever z it is given.** The matrix is
  unturned with translation (x, y, z) as given, handed to the mission loader's building maker
  (`0x10033cb0`) with the start flag 0 and id −1, and the handler files the building as slot 4's
  **event 2** (`0x1003dad8`). **No construction sphere**: `CreateObjectFromScheme` gives order 18 only
  on create bit 1 (`ArealMap.dll:0x10015df3`), which `0x10033cb0` never sets; the control is the
  builder's call, which passes 2 (`Behavior.dll:0x10029268`) and gets the sphere. Sent no code, its
  controller plans from the constructor's record to the code-0 anchor, and on **all 30** `fortif.rlb`
  controllers the first state on that way runs **action 20**, placing it in the landscape at once, and
  none runs action 1 (*measured* through the engine's planner). The insertion then **sets it down on
  the mean of its cut contour**, its start flag clear: C04 Mission 02 asks for z 10 over ground at 51.1
  (the map's lowest vertex 49.0), and the Teleport's origin comes to 47.82, its base on the mean, as
  Tut_4's placed Main Teleport stands 3.7 under its ground. **This corrected
  [04-missions](docs/04-missions.md#the-start-flag-keeps-a-building-at-its-file-height--read-and-measured)**,
  below ([15-behaviour](docs/15-behaviour.md#what-the-consoles-create-bcreate-and-death-do--read-and-measured)).
- [x] ~~**What `death(x, y, r, delay)` kills.**~~ — closed 2026-09-30. **Scenery alone, at once, by the
  life system's kill.** The sphere is centred on the probe's surface at (x, y), with nothing added, and
  has radius r. The world's slot 3 (`Terrain.dll:0x10025f40`) is asked for its objects with mask
  **`0x400`**, and each gets interface `0x16` slot 7, the kill (`Control.dll:0x1000eb70`)
  (`iron3d.dll:0x1003dcef`–`0x1003dda2`). The construction sphere's kill is the same with `0x414`. Bit
  *n* is class *n* ([42-selection](docs/42-selection.md) reads `0xa` and `0x41a` so), so `0x400` is
  **class 10, trees and stones**, taken when their own sphere meets the query's, and never a unit, a
  building or the hero. **The delay is converted and never read.** *Measured*, engine: C04 Mission
  02's line fells the stone on the site (radius 277, 20 off) and one tree (radius 88, 140 off), and
  none of the other 15 pieces of scenery
  ([15-behaviour](docs/15-behaviour.md#what-the-consoles-create-bcreate-and-death-do--read-and-measured)).
- [x] ~~What the AI's design pick takes as its candidates, and how it ranks them~~ — raised and
  closed 2026-10-01 from the names two recordings give the enemy's builds, **read** and **measured**.
  - *Seen*: on C03 M02's easy level every enemy build the target panel names is wheeled or tracked
    ("Let's Play - Parkan: Iron Strategy", Part 6.5: MTW-3, SWW-5, LWW-8, LWW-9, MTW-10, SWW-10,
    SWW-19, MWW-21; Part 6: MTW-3 to -5, LTW-6, some two dozen SWW). The engine built small flyers and
    walkers.
  - **The byte at a design record's `+0x104` is the clan's research.** The fill never writes it;
    `ai.dll:0x10010f90` does, walking each unmarked scheme's nodes against the clan's own tree and
    setting the byte when every part is researched and in the tree. It runs when the tree is handed
    over, when function 43 loads the store, and each time function 41 orders a research; not at the
    pick.
  - **`SELECT_SMALLEST` is not the last of the ranking**: the loop at `0x10010b73` takes the
    best-ranked design of size class 2 or less.
  - **The two floats.** Property 54 is the nodes' life ÷ the armour's linear factor, plus the
    deflector's values × the shield's (`Control.dll:0x100138b0`); the gun total is each gun's round
    damage ÷ magazine × rounds left (`Behavior.dll:0x1001ccb0`), a damage and not a rate. Both match
    the 77 records the game itself writes to `preload.lda`, 70 of 70 robots to 1e-7 and 77 of 77.
    docs/15's "the divide never happens" was wrong.
  - *Measured* over the nine by-name scripts' 11 clans and 59 warrior designs: a clan may build 7
    (`data`, `scream`), 11 (`c2m3e`), 27 (`c1m4e`, `c3m1e`), 28 (`c2m1e`), 43 (`c4m2e2`) or 44
    (`c3m2e`, `c3m2e2`). On C03 M02 Enemy 2's pick is `23_swhl1.dat`, a Small Wheel, and Enemy 1's
    seven on easy are four large tracked, two large wheeled and one medium tracked: no walker and no
    flyer. The control: unflagged, every clan's best would be `lwing1`.
  - **What the factory refuses** (`M_Task_Construct`'s start, `Behavior.dll:0x10029ba0`): no free
    mind; a scheme that will not open; a chassis larger than the factory or of no known size; and,
    for a paid bot only, a part not researched in the clan's tree. Never ore or power. No shipped clan
    is starved by a refusal.
  - The engine was wrong on all three — the candidates, `SELECT_SMALLEST` and the scores — and now
    builds SWW-5, SWW-6 and LTW-3, LTW-4, LWW-5, LWW-6
    ([15-behaviour](docs/15-behaviour.md), [36-factory](docs/36-factory.md),
    [16-research](docs/16-research.md)). Queued: why Enemy 1's first builds are medium tracked three
    times running, and function 41 in the engine.

## Mission progression

[34-progression](docs/34-progression.md#not-established)

- [x] ~~The clock unit that times route reports~~ — closed 2026-09-18: **game milliseconds**, so a
  takt comes 1984 to 4916 ms after the last. The takt's argument is the parameter of
  `MBehaviour::SendMsg`'s message 1, which the game itself names `NEW_GAME_TAKT`; `Terrain.dll`
  handles the same message with the same parameter and spends it against
  `CAtmData::GetTimeDiffInSec() × 1000` (`0x10070134`, `0x100701d2`). It is the *game* clock, not
  the wall's — `World3D.dll` exports `PauseGameTime`/`ResumeGameTime`, and `Behavior.dll` imports no
  clock at all. **`rand8` is the low byte of `Behavior.dll`'s own `rand()`** (`0x1004ce3c`, the CRT
  LCG), whose state at `0x10063c1c` is **1** in the shipped file and is written by nothing but the
  generator — no `srand` ever runs, so it is one deterministic stream, whose first draws are 41, 35,
  190 and 132 (recomputed here from the LCG). **The first takt runs at once**: the timer record's
  next-run dword starts at 0 and the scheduler fires on a 0. The engine was wrong on all three;
  fixed and tested ([34-progression](docs/34-progression.md)).
- [x] ~~How a destroyed or captured unit leaves function 31's list~~ — closed 2026-09-18 as a
  **negative with its control: it never does.** `ai.dll` names offset `0x8c` in 135 instructions; of
  the calls that follow one directly, 99 are `operator[]` and 23 the count, and the one insert
  (`0x10002da0`, add-if-absent) has exactly two call sites, slot 4's events 1 and 2. **There is no
  erase and no clear**, and the only write of −1 to a record's id is the constructor's
  object-not-found path, which slot 4 then refuses to insert. The control is that the same sweep
  does find these records' task slot `+0x14` cleared, at four named addresses. A **capture** files
  the object with the *new* clan's SuperAI as event 2 (`iron3d.dll:0x10032fd0`) and tells the old
  clan nothing, so it is counted by both. The engine declines to copy this — it still takes a
  destroyed unit off the count and moves a captured one — under a stand-in that now says plainly
  what the game does and why, since counting every object a clan was ever told about would leave
  several missions' objectives unreachable, which has not been watched in the game
  ([34-progression](docs/34-progression.md)).
- [x] ~~Where message text is drawn, and for how long~~ — this line was **stale**: the doc has
  answered it since M9. The message box stands at x 230, y 0, width 182 on the 640 × 480 screen
  (moving to x 374, y 352 in view state 2), holds at most six lines, and each frame deletes it once
  **20 seconds** have passed since it was made (`0x1007f4f0`); in the recording, two boxes that no
  later message replaced left 19.75 s and 20.0 s after their text appeared. Confirmed by the
  coordinator 2026-09-18 ([35-hud](docs/35-hud.md#the-message-box--read-and-measured)).
- [x] ~~What `info_system` changes on screen~~ — **stale** too, and answered in the same section: it
  makes the message kind 4 rather than 3, and the kind picks the box's header out of the table at
  `0x1007f9a0` — string 6214 *from: Information assistant* instead of 3057 *from: Training
  assistant*. Nothing else about the box changes
  ([35-hud](docs/35-hud.md#the-message-box--read-and-measured)).
- [x] ~~What follows `MISSION_COMPLETE`, and what leads to the next mission~~ — closed 2026-09-18:
  **nothing auto-advances; the player does.** Both shell branches parse the path of the mission just
  played and stop there — the campaign branch (`0x10009c10`) takes the mission's two digits less one
  and the campaign's as they stand, the single branch (`0x10009f40`) the mission's less one — and
  those are the **list positions of what was just played**. Neither adds one, neither writes the
  parameter block, and each posts shell events to the screen `+0x54` names, which case 2 has just
  set to the main menu. **`dispatcher.ini` is progress, not order**: the order is the directory
  numbering, walked when a list opens (campaigns from 0, missions from 1, each stopping at the first
  index whose `descr` will not open), while the record's `+0x18` is *complete* and `+0x19`
  *available*, and `[COMMON] ALL_AVAILABLE` unlocks everything. *Measured*: the install's file is
  **935 bytes with one `[COMPLETE]`, 21 keys all 1 and no `[COMMON]`**, and six campaigns of
  4/4/4/4/2/2 with no gaps (checked here independently); all 34 directories the enumerators walk
  carry a one-line `descr`. The same read settles the rest of the paragraph: **mode 3 is the
  auto-demo** and **mode 4 the training campaign** (`0x1005c748`–`0x1005c766`), and **game message
  3** is `World3D.dll`'s network layer on `DPSYS_SESSIONLOST` or a failed send. The engine has no
  shell, so its row stays, reworded ([34-progression](docs/34-progression.md)).
- [x] ~~Whether the hero reports its route from inside a boarded bot~~ — **stale**, closed
  2026-09-30: it does, and [39-boarding](docs/39-boarding.md#boarding--read) already has it *read*
  and *seen*: the game frame keeps the hero's object on the bot, at its x and its y less its node
  sphere's radius, so its route report follows the bot (engine since `a29793a`). Seen again in "Let's
  Play - Parkan: Iron Strategy, Part 3": C02 Mission 02's *"Reach the last marker left by Ballen"*
  completes at 19:25 with the hero aboard the LFW-2 over the last buoy, having been told *"Risk
  area! Landing impossible."* at 19:23; the engine completes it the same way.
- [x] ~~The ambient variations' schedule~~ — closed 2026-09-18: **one every 10 to 19 seconds, from
  the first frame.** Each frame the game asks `services.dll`'s `ITimer` for the seconds since its
  stamp and, once that exceeds the wait, restamps, draws a new wait of **10 + `rand()` % 10**
  seconds and plays one variation (`iron3d.dll:0x1005eb49`); both stamp and wait start at 0, so the
  first frame plays one. Only the pause byte and state word 3 gate it, so the variations play during
  a briefing too, which closes [21-briefing](docs/21-briefing.md)'s line with it. The list is the
  `DEFAULT_` one when the mission gathered no `DAY_`/`NIGHT_` name, else picked by a day/night flag,
  and the index is drawn by the same pair of 16-bit words as the motion jitter, redrawn while it
  equals the last — so **a variation never repeats immediately**. *Measured* over all 29
  `mission.cfg`: every one names a type-5 theme with exactly one `THEME` and a type-4 variation
  list; **171 variation lines binding 170 names**, 2 to 13 each; 17 use `DAY_`/`NIGHT_` and 12
  `DEFAULT_`, none both; three themes serve all 29. The odd line is **`Single.02`, which writes
  `DAY_VARIATION1` twice** — `atm_bees.wav` then `atm_bird2.wav` — and the loader's
  `std::map::operator[]` keeps the later, so `atm_bees` is built and dropped and never plays in that
  mission; it is the only repeated key in the 29. Implemented and tested
  ([34-progression](docs/34-progression.md), [20-resources](docs/20-resources.md)).
- [x] ~~A failure on the hero's death~~ — **stale**: the game fails the mission itself when the
  player's clan's hero (`Type` `0x1020000`) is lost, driven or not, putting the view on it first
  (`0x100a4e50`) and then failing (`iron3d.dll:0x10075619`), after which the panel reads *"MISSION
  FAILED..."*. `CLAN_HERO_KILLED` does nothing in this build and Mission 01's script never fails,
  which is what had kept the line open. Confirmed by the coordinator 2026-09-18
  ([34-progression](docs/34-progression.md#after-the-outcome--read-and-measured)).
- [x] ~~What `OBJECTIVE_PROGRESS` does~~ — raised and closed 2026-10-01, **read** and **seen**:
  it puts an objective whose state is not 0 back to 0, open, with no string, no voice and no
  completion test (`iron3d.dll:0x10060e44`–`0x10060e5c`). The callback's switch (table
  `0x10061038`) sends kinds 3, 4 and 5 to cases that all work the objective record's `+0xc`
  through `0x1006b440` and `0x1006b450`. Read alongside it: `OBJECTIVE_COMPLETE` completes only an
  objective at state 0, so a failed one stays failed; and `OBJECTIVE_FAILED`, on one not failed
  already, shows string 5041 with no voice and sets −1 (`0x10060f8c`, `0x10060ffd`), where the
  engine had changed no state. *Seen* in "Let's Play - Parkan: Iron Strategy, Part 6"
  (-yNnsqudMzw). C03 M02's generators objective completes at 53:57. Nothing shows as Enemy 2
  retakes its generator, and *"Objective is completed"* shows again when the player takes it back
  (59:17). Its bonus "Part 6.5" (9SBZOCWv_vE) does the same at 24:35 and 32:21. Implemented and
  tested, and the engine README's stand-in row went with it
  ([34-progression](docs/34-progression.md#objectives-and-the-end-of-a-mission--read-and-measured)).

## Files and formats

[17-saves](docs/17-saves.md#not-established), [16-research](docs/16-research.md#what-is-not-read-here),
[19-descriptions](docs/19-descriptions.md#what-is-not-read-here), [18-vocabulary](docs/18-vocabulary.md),
[21-briefing](docs/21-briefing.md#not-established), [06](docs/06-open-questions.md), [22-settings](docs/22-settings.md),
[12-rsli](docs/12-rsli.md#not-resolved)

- [x] ~~[M5] How the game turns a string's characters into the font's glyph indices~~ — closed
  2026-09-20, **and the engine was wrong**. **The game does not map characters at all**: `Ngi32.dll`'s
  text-out (`0x10010e40`) loads each string byte zero-extended and indexes the 256 records at
  `font+0x38` by it, and the width routine does the same. **Nothing is ever replaced by `?`**, which the
  stand-in had guessed. The one transformation is a 256-entry `int16` table at **`0x10036e50`**, applied
  only while the font's flag at `+0x104c` is 1, and it is **Windows-1251 → CP866** — *re-read and
  round-tripped by the coordinator through Python's own codecs: **0 disagreements** across 0xC0–0xFF
  plus Ё/ё, and **190 of 256** entries are the identity.* The constructor sets the flag and
  `services.dll` clears it on every font it makes. *Measured*: the nine `ui/font.lib` fonts draw **79**
  glyphs above 0x7f while `sys.lib`'s and `gamefont.rlb`'s draw **64** and **56**; and of **24740**
  strings in 112 shipped text resources **none** carries a byte above 0x7f, this being the English
  release ([12-rsli](docs/12-rsli.md)).
- [x] ~~[M5] How tall a glyph is drawn, how far apart lines are, and how far the pen moves after a
  glyph~~ — closed 2026-09-20 for the glyph and the pen, **narrowed for the spacing**, and **the engine
  was wrong on the pen**. The header's four words are the metrics, copied into the font object *out of
  order*: the fixed-pitch cell, the glyph **height**, that height in texture coordinates, and **what the
  pen adds after every glyph's advance**. *Re-measured by the coordinator over every font the archives
  hold*: the pen extra is **1 on all of them**, and the texture-coordinate height × the atlas height is
  **exactly height + 1** on all of them (8, 10, 12, 13, 16, 20, and 18 on `gamefont.rlb`'s) — which is
  also the atlas's row pitch, so the font is drawn **one texel to the pixel**. The pen therefore moves
  **advance + 1**, and the engine had been moving it by the advance alone, overlapping glyphs by a
  pixel. **A premise of an earlier round is corrected**: the recording measurement that justified the
  old row was **miscalibrated** — it assumed `mf_640` at 640 × 480 where the recording is **1024 ×
  768**; refitting against `gf_1024` places all twelve runs of "TSW-1 Warrior" to within **0.56 px**
  with `advance + 1`, while the advance alone is out by up to 4 px. **Line spacing is not the font's**:
  the routine draws one line at the y its caller names, and that is the one row left standing
  ([12-rsli](docs/12-rsli.md)).
- [x] ~~[M5] How the game draws its font: its 8-bit blend table and the text's colour~~ — closed
  2026-09-20, **and the engine was wrong**. Text draws through **phase 8**, whose state record sets
  stage 0 colour to **`MODULATE(TEXTURE, DIFFUSE)`**, stage 0 alpha to the texture's, point filtering
  and no mip, and the alpha test on at `GREATEREQUAL` against reference 1 — **so the run's colour
  multiplies the atlas, it does not replace it**, and the key is the atlas's own alpha. All eleven fonts
  set `0x1000000` in their `Texm` header's `+0x14`, the alpha-surface bit, where **palette index 0 is
  cleared to alpha 0** and 1–255 made opaque; an atlas uses 2 to 5 indices, 0 always its corner, so 7 of
  the 11 draw a white body and a shadow a quarter as bright. Nothing converts out of display space.
  **And the `Ipol` blend table is dead**: it is the 8-bit renderer's, and **79 of `Ngi32`'s 145 exports
  are stubs** folded onto fourteen addresses — `vrtTextOut` is a bare `ret 0x18` — against the control
  that `rsOpenLib`, `rsLoadFast`, `rsLoad` and `rsGetInfo` are real and the Direct3D texture loader
  calls them. A negative worth its own line came with it: **nothing in the install opens `gamefont.rlb`,
  and nothing opens `sprites.lib`** — the byte strings occur in no file, against the control that every
  one of the other twenty archives' names does occur somewhere. The game's on-screen text is
  `ui/font.lib`'s nine ([12-rsli](docs/12-rsli.md)).
- [x] ~~What `iron3d.dll:0x1008a690` does with a part's derived number, and what else reads a part's
  `Type`.~~ — closed 2026-09-20: it is a **leaf of compares** turning the record into a **part category
  0–7, or −1**, and the number is a **jump-table index and nothing else**. Kind 9 sub 32 → 0, sub 33 →
  1; kind 8 by its *second* sub-kind (`BLD`→5, `TUR`/`UPG`/`DEF`→1, `RDR`→2); kind 12 → 2; kind 11
  `BRN` → 6, `ARM` → 7; kind 10 → 4; the tail at `0x1008a761`–`0x1008a770` gives 3 for any other device
  and −1 for the rest — so **a building is filed by its branch**, a bunker's turret fitting as a turret
  and a tower's radar as a gun. The only two askers are fitting (`0x100519e0`, bound `cmp eax, 6`) and
  unfitting (`0x10053a50`, `cmp eax, 7`), and two consequences fall out: category 0 and −1 both fall
  past the fit bound, so **clicking a chassis once a project exists does nothing**, and on the unfit
  side 0 and 5 both reach `0x10053b00`, which **throws the whole project away**. **Who reads the
  derived `Type`**: six call sites in three functions, *all three the designer's* — the unit box title,
  the takt's Accept, the unit writer — and **nothing else reaches it**, a negative whose control is that
  the same raw scan does find `Terrain.dll:0x1007ed40` and `0x10080550` in the vtables the docs say
  they sit in. *Measured*: all **29** trees carry the same 395 parts and the same split — 27 chassis,
  74 turrets, 67 guns, 104 devices, 58 clips, 34 buildings, 6 brains, 24 armours — with exactly **one**
  uncategorised (`R_H_01`, the hero target) and **293 of the 395** deriving `Type` 0. The engine was
  already right for the range it covers ([16-research](docs/16-research.md),
  [38-designs](docs/38-designs.md)).
- [x] ~~Which of the `bb`/`bl`/`bm`/`bp`/`br`/`ba`/`bf`/`bt` building prefixes is which~~ — closed
  2026-09-18, and **they are not buildings**. All 61 records under those prefixes are `BULL` records —
  rounds — with their meshes in `weapon.rlb`, and none is placed in any mission. `b` is the tag's own
  letter and the second letter is the weapon family: bullet, laser, missile, plasma rifle, rocket, animal,
  flamer, taser, each settled by what fires it, by the trail effect its controller plays and by its
  flight ([18-vocabulary](docs/18-vocabulary.md)).
- [x] ~~Briefings: what game mode 4 is, how a briefing is skipped (`WaitForClick`), and the spline's
  curve.~~ — closed 2026-09-20, **and it overturns a premise of [21-briefing](docs/21-briefing.md)**.
  The queue had conflated two things: the 8 × 8 table at `0x10104b18` is the interface's *view* mode
  stack, while **game mode is the first word of the shell's parameter block**, split into
  `+0xe4`/`+0xe5`/`+0xe6` at `0x1005c748`–`0x1005c766`. Its five modes are 0 no game, 1 a single
  mission, 2 multiplayer, 3 the attract-mode demo, and **4 a mission of campaign 0** — computed from
  the chosen campaign's index in five branchless instructions at `0x1000386e`–`0x1000387c` (`neg` ·
  `sbb` · `and 0xfffffffd` · `add 4`), giving **4 for index 0 and 1 otherwise**; *re-read by the
  coordinator* off a continuous decode from the handler's entry. Campaign 0 is `CAMPAIGN.00`, *Tara.
  The Home Base* — **the training campaign**. *Measured* from the other side: the two messages the game
  asks for itself in mode 4 (22, 100) are each defined in exactly one of the install's 16
  `messages.cfg`, both in `CAMPAIGN.00`. **The premise corrected**: the byte the briefing tests before
  running at all (`0x100a29e5`) is **mode 3, the demo — not a saved game being loaded**, which the doc
  said; with it set the set-up overwrites the mission directory with `missions\autodemo.00\`, so the
  demo plays with no briefing. **`WaitForClick`** closes as a negative with its control: the name
  occurs in exactly one of the 22 binaries and its only four references are inside the waypoint loader,
  which parses it *identically* to `WaitForTime` from the same function — and `WaitForTime`'s slot **is**
  read by the dwell, so this build loads the value and never reads it. The spline was already closed
  and is confirmed: a **cubic Hermite** over `EdgeTime` ([21-briefing](docs/21-briefing.md)).
- [x] ~~What `TRF1`'s directory flag does beyond the debug warning.~~ — this line was **stale**,
  caught by the coordinator's own check rather than spent on an agent: it is already **narrowed** where
  it stands in [16-research](docs/16-research.md) to marking a tree as carrying debugging information,
  whose one reader found is the warning `FULL_RESEARCH_TREE` silences, and **no shipped archive sets
  it** — near enough a settled negative that an agent's time went to the briefings instead.

## Mission 02, *The Constructor*

Added on 2026-09-15 against `4f3a16e`, after the Outpost island's landing was fixed in M14.
Audited on 2026-09-21 against `72c72aa` for lines already answered: of 27 open, 2 were stale
outright and 7 more stale in part; 19 closed, 8 narrowed or partly answered.
Worked on 2026-09-22 as the eighth round, against `0715e11`: of the 9 still open, 7 closed and
2 narrowed, with 4 new lines for what the answers left.
[34-progression](docs/34-progression.md#not-established), [36-factory](docs/36-factory.md#not-established),
[37-designer](docs/37-designer.md#not-established), [38-designs](docs/38-designs.md#not-established),
[39-boarding](docs/39-boarding.md#not-established), [24-motion](docs/24-motion.md#not-established),
[31-packages](docs/31-packages.md#not-established)

**Where the engine is weaker than the game**

- [x] ~~The Large Factory's front door: its hall way does not lead to the pod, so the engine's way in is the west side door, which the recording's hero takes.~~ — closed 2026-09-21, and **stale in part**: the west door is the game's own way in for a walker, not a workaround. What joins a hall way's groups was already read — each exit is linked at cost 1 to the walkable areal under it (`ArealMap.dll:0x1002363f`), and all nine of the Large Factory's exits stand over walkable areals on Tut_2. Counting all 85 links, gated ones included, the hall way is still three groups and three lone vertices (18, 19, 20), and the front group never reaches the pod. The four gated links — 69–60, 60–46, 68–64, 64–41 — are all flyer-only and all inside the interior group, so a walker reaches the pod from exit 67 alone, through `i21`. **The engine was wrong** in a way the old premise hid: its route search ignored the gates, so a walker from the east could be routed in over 69's flyer-only links; walkers now drop flyer-only links and everyone drops shut ones ([24-motion](docs/24-motion.md#a-building-is-drawn-cell-by-cell-through-its-portals--read)).
- [x] ~~The chimney smoke is orange where the recording's plumes are black~~ — closed in M14: an effect sprite takes its material entry's cell as a mesh batch does, and plays its track from its own start, so a puff leaves the chimney on `fire_smoke`'s first, orange cells and is on its later, black ones a quarter of a second on. Its plume's size went with it: a stream's particle walks and grows in metres, which the recording's 29 m over the chimney and 31 m across measure, where the control points' 2.6-long axes had made it 130 and 78. What a sprite's material starts from, and that the effect draw takes the entry's cell at all, are still stand-ins ([07-objects](docs/07-objects.md#how-a-material-reaches-the-device--read-and-measured)).
- [x] ~~[M14] When `AniMesh.dll` works an agent's sphere and its node sphere out again (`0x10009510`), at which pose, and who sends a part's removal in play~~ — closed 2026-09-22, a **negative with its control**: **no unit in play loses a part from its list**. The removal `0x10003c40` has one caller, the agent's `IGameObject` slot 13 (`0x10001370`) on message 6 with sub-code 20. A stack-aware read of all 376 calls through `+0x34` in the 18 shipped binaries finds five that pass 6/20, all `iron3d.dll`'s designer taking a part off its own project model (`0x10053bab`, `0x10053c2e`, `0x10053c94`, `0x100540ff`, `0x100543ba`); the same sweep finds the 7 part loads (6/`0x80000020`) — the designer's five, `Behavior.dll:0x1001ce5b` and `ArealMap.dll:0x10014cab` — and 21 sends of 6/7. Each forwarder was followed and passes other codes. A destroyed part is knocked off and hidden, and the agent keeps it. **The engine was right**, its spheres worked out once at rest; the M14 row is gone ([24-motion](docs/24-motion.md#finding-the-ground--read)).
- [x] ~~The minds: the recording's factory shows the hero holding one of the player's two, the same question as Mission 04's which unit holds none.~~ — closed 2026-09-21; it had been inferred from the recording's figure, and is now **read**. The mission loader places the kind-1 list with create flag 8 unless the game is the auto-demo (`0x100774d1`), and `ArealMap.dll`'s create takes a free mind for every non-building with that flag (`0x100152ba`) and writes the new object's id into it (`0x10015f24`); with none free the unit is not made at all. All 296 placed robots are kind 1, the 37 heroes among them, so **the hero holds a mind**. A unit the hero's Enter takes holds none — `Capture` writes no mind (`Behavior.dll:0x10009051`) — which **settles Mission 04's line** below. Control: the same search finds the factory's claim (`0x1001e0e0`) and reservation (`0x1002a348`). The engine was right ([23-economy](docs/23-economy.md#the-bot-limit-is-the-clans-mind-count--read-and-measured)).

**The factory screen and production**

- [x] ~~What puts the capture and the factory screen on one frame.~~ — closed 2026-09-21, **stale**: read in docs/27 on 2026-09-14 (`a30258c`), the day before the line was queued. The pod's callback (`0x10061050`) calls the ownership change (`0x100a48a0`), whose every branch shows System line 5039, makes the factory record (`0x100a4c02`) and joins at `0x100a4e1e` to end in the opening `0x10062630` (`0x100a4e2d`), so the line, the record and mode 5 come out of one building tick. The engine's `pod_fired` already did all of it in one call ([27-ownership](docs/27-ownership.md#capture--read), [36-factory](docs/36-factory.md#how-the-screen-opens--read)).
- [x] ~~[M11] How the cursor is shown in view mode 5 (the engine: the system's cursor), and the fill colour the resource rows hand their bar (the engine: the weapons list's).~~ — closed 2026-09-21. **The cursor is ARROW, drawn as command mode draws it**: mode 5's case of the screens' draw sets the cursor-shown byte `0x1010b5c8` (`0x1008d4f1`) and raises the system's cursor only when the display's slot 12 answers (`0x1008d508`); the frame then runs the chooser (`0x10058710` at `0x10060c95`), whose pick answers kind 0 in view state 1 and over the designer (`0x1008daa4`–`0x1008dae5`), so the state is 1. `0x100a4fc0`, the line's lead, is the mouse filter's zoom, not the cursor. **The engine was wrong** and now draws ARROW's software strip last. **The fill is the weapons list's**, picked by the bar primitive from the percentage alone (`0x1009a530`–`0x1009a54c`), so the engine was right; the same read found the factory's progress bar is handed its percentage and fills to it (`0x10097709`–`0x1009772f`), which the engine never drew and now does ([36-factory](docs/36-factory.md#the-cursor-in-mode-5--read), [the resource rows](docs/36-factory.md#the-resource-rows)).
- [x] ~~Whether the designer pauses the world (the level's flag bit 8), and whether the `Mission` handler runs while a building's screen or the designer is up (the pause byte `+0xe8`).~~ — closed 2026-09-21: **nothing pauses**. The designer's 9 goes to the level's `+0`, the view the world is drawn from (`0x100a4fb2`), which is `Terrain.dll`'s `CCamera` flag word `+0x164`; `CCamera::Render` tests the 8 twice and skips the world's passes (`0x100845e5`, `0x100846fb`). So bit 8 draws no world behind the designer. The pause byte `+0xe8` has two writers in `iron3d.dll`, the constructor (`0x1005c512`) and the setter `0x1005f620`, whose five calls are the Pause toggle, Esc, the help screen and a widget's show slot, none on mode 5's path or the designer's — control: the same scan finds the setter's own store (`0x1005f630`) — so the `Mission` handler runs. The engine was right; the 9 also replaces infrared's `0x20`, so the engine now puts the hero's night sight out when the designer opens ([36-factory](docs/36-factory.md#the-designer-does-not-pause-the-world--read)).
- [x] ~~What handing the hero back does to it while the screen is up, and whether the player's keys still move it.~~ — closed 2026-09-21: **nothing moves it**. The hand-back sends (6, 7, 0), the Wizard's AI mode (`Wizard.dll:0x10001ced`), in which the manual manager that runs the key rows is never updated (`0x10001ca2`); the 0→5 transition then sets the unit's own group word to 3 (`0x10064480`), so its AI does not follow its path either, and clears the keyboard (`0x10064452`). The engine was right ([36-factory](docs/36-factory.md#what-the-hero-does-while-the-screen-is-up--read)).
- [x] ~~The Wizard's heading curve (`Wizard.dll:0x10003d80`), which sets the heading a new bot ends its escape with, and the height a flyer's walk points are given.~~ — closed 2026-09-22. **The heights**: the trajectory builder (`Behavior.dll:0x1003a480`) cuts a flyer's leg into ⌊d⌋/20 points across the ground (`0x1003a5ea`; a walker's divisor is 7777, so never), and `0x10040f20` stands each 15 over the ground routine's answer (the float at `0x10059974`) — an animal's 30 and up to 50 more. The ground routine (`0x100146b0`) looks down for buildings, trees and stones first and answers their top face **plus 100** (`0x100148a5`), then the landscape. The walk to a point (`0x10001960`, 28 call sites, migrate's and the escape's among them) and the global path's waypoints ask for it (place word `+0x14` = 0). **The curve** is a second Hermite cubic from the machine to the point, leaving along the hull's forward axis and arriving along the point's heading, which the walker writes as the point's velocity (`0x1003a7f9`); the Wizard faces its tangent at t + dt, or a straight blend with flag bit 0 (`0x1000335d`). **The engine was wrong**: `FLY_NEAR_LAND` and the M8 height row are gone, the Wizard samples the read curve, and Mission 02's escape now climbs 115 m over the Large Factory's roof instead of being pushed onto it; a new install test fails on the old stand-in (18 m over the roof). One small stand-in stays: the sample is held at a segment's end ([24-motion](docs/24-motion.md#the-heading-curve--read), [A flyer's walk points](docs/24-motion.md#a-flyers-walk-points--read-and-measured)).
- [x] ~~What commander pages 1–4 and 6–8 show from first person.~~ — closed 2026-09-21, **stale in substance**: what those pages show is docs/41's ("The pages"). What was left is which are ever up from first person, and it is only 4 and 5: only the pods turn the page in mode 5 (`0x10062723`, `0x1006276a`), the column's buttons work in modes 3 and 4 alone (`0x1008439b`) and the page keys in mode 3 (`0x10071782`), and none of the other 40 calls of the page setter was found turning a page in mode 5. The engine was right ([41-commander](docs/41-commander.md#the-pages--read)).

**The warbot designer**

- [x] ~~[M11] That the factory record's `+0x30`, the grade the chassis page is taken over, is the building's size class.~~ — closed 2026-09-21: the writer is found. `iron3d.dll:0x1007e3c0`, slot 3 of the base record's vtable `0x100e6548`, is called first by the building record's bind (`0x10032d69`) and the unit record's (`0x10074d68`) and by nothing else; it asks the object's interface `0x10` slot 26 for `0x201` and stores the first dword at `+0x30` (`0x1007e538`–`0x1007e549`, checked). `MBehaviour` answers with `+0x960`, the size class, a building's from the fourth letter of its root name (`Behavior.dll:0x1000cee0`: l 2, m 3, b 4, e 5). *Measured*: the 28 placed factories are 16 `fr_b_plant` (4), 2 `fr_m_plant` (3) and 10 `fr_l_plant` (2). The engine was right; the same bind answers the unit record's `+0x30`, docs/31's old line ([38-designs](docs/38-designs.md#the-catalogue--read-and-measured)).
- [x] ~~[M11] The part box: `Epower`, the properties behind `regener`, `capacity`, `throughput`, `shotnum` and `blast`, and the formatter that prints one decimal whatever the template asks.~~ — closed 2026-09-21; `regener` was **stale**, component query `0x1100` in docs/26 (`577891e`) before the line was queued. The row draw `0x1006ea50` pushes the constant `"%6.1f"` (`0x10104d98`, its one reference at `0x1006eb5e`, checked) and never reads the template's width or decimals. `Epower` is query `0x1200`, a class-5 engine's power × its value 0 × its condition — LEng1's 4.5 × 0.7 prints the recording's 3.1; `capacity` the sum of every battery's value 0 (property 113); `throughput` device 0's power (property 164); `shotnum` the magazine (query `0x800`), printed as a float; `blast` the gun's `+0x178`, filled from its round's `0xa5` — node 0's `.exp` radius raised by how far each other node's blast reaches. *Measured* over 58 clips: 14 direct-hit rounds print 1.0, the winged SSMs 60.7 and 45.5; no tree names `Adfactor` or `product`. **The engine was wrong** on `Epower`, `shotnum`, `blast` and the columns' right edges (x+125 and x+128, not 120 and 123), and is fixed ([38-designs](docs/38-designs.md#a-parts-box--read)).
- [x] ~~[M11] What the turret fit does to guns on a turret it replaces, and the gun fit to a clip on a gun it replaces.~~ — closed 2026-09-21: **neither fit is ever given a filled socket**. The takt adds a turret or a gun only into an empty row (`0x100509ba`, `0x10050a0e`), and to change a part the player takes it off first: a turret's removal (`0x10053df0`) takes its radar, deflector and each gun with it through `0x10053a50`, and a gun's (`0x10054210`) its clip. The engine behaved so already, its interface fitting only empty slots; `fit_turret`, `fit_gun` and `openparkan.designs` now refuse a filled one ([38-designs](docs/38-designs.md#fitting--read-and-measured)).
- [x] ~~[M11] Which destination row a tab selects as it turns on, which tab the panels turn to after a fit (only *seen*), and when fitting a chassis enables Armour (`0x10052491`).~~ — closed 2026-09-21: **each tab keeps its own row** (panel `+0xcec8` + k × `0xcea4`), and turning one on changes none (`0x10049e50`). A chassis fit opens Turrets, Internal systems and Armour only when it gives them rows (`0x10051f3a`, `0x1005241a`, `0x10052491`) — Armour when the chassis has an `i_arm_` slot, which the engine already had — and turns the panels to Turrets; a turret fit turns them to Weapons only if the turret has a gun socket; every fit and every removal steps its tab to the next row, wrapping. *Measured*: 20 of 24 chassis give all three tabs rows, `r_b_07` and `r_l_06` having no turret socket, and 7 of 55 turrets have no gun socket. The engine's row-per-panel and its tab rules were partly wrong and are fixed ([37-designer](docs/37-designer.md#which-tab-and-row-a-fit-leaves--read)).
- [x] ~~[M11] The previews' camera, the scan band's specular, the scene colour a preview's materials take while the designer is up, and the order of the turn and the pitch~~ — closed 2026-09-21 (the camera and the band) and 2026-09-22. **One scene colour for the game**: `CID_SHADER` is `terrain.dll CreateShader`, a singleton at `0x100a60e8` whose state block starts at 0.2 grey (`Terrain.dll:0x1004bbcd`); `CShade` hands it to every prim buffer, and of the five mask sends only the sky's `0x50` carries the colour (`0x1007bbc5`), on the atmosphere's takt, not in the draw — so a preview takes the world's colour of the moment even while the designer hides the world. **The engine was right**; its fallback without a sky is now the shader's 0.2, not 0.15. **The placement** is P · T(c) · R · T(−c) (`0x1009efea`–`0x1009f060`): R is a quaternion about +z laid down transposed by `Ngi32.dll:0x10014540`, so the model turns **clockwise from above** about its own centre, and P carries −c (`0x1009ec66`), so the centre lands at (P − I)·c, off the origin; the recording at 108.95–109.95 s shows the clockwise sense. **The engine was wrong** and turned it the other way, centred; fixed and pinned. Besides: the colour filter's slot 5 (`0x1004f900`) is read — green = min(255, 7 × 0.33 × (R+G+B)) ([37-designer](docs/37-designer.md#the-previews--read-and-seen), [10-sky](docs/10-sky.md)).
- [x] ~~The game object's `+0x08` value 4; bit 1 of the camera's flag word; the destination panel's takt (`0x1004cd00`); which screens open the `fr_` and `a_` designers; the load list.~~ — closed 2026-09-22. **4 is the game's state word *playing*** (docs/34), and the designer's mouse and Esc handlers act only then (`0x10056004`, `0x10055ea6`, re-read by the coordinator): **the engine was wrong** and took both after the outcome. **Bit 1** means *the eye places this view*: its one reader is `Control.dll`'s first-person eye (`0x100234ec`), against a sweep of 14 modules finding the other bits' tests; nothing to change. **The destination's takt** scrolls as the source's; a picked row rebuilds the source's same tab for its socket (`0x10048220`) and the preview only if the row holds a part (`0x1004d0dd`). **No screen opens the `fr_` and `a_` designers**: the factory's `0x1009810a` is the open's only caller, against the holder's constructor found once, and the kind's three other writers only zero it. **Save and load are read and built** — a 16-character name field on (270, 410)–(370, 430), and a load list of `stdGetValidRobots`' designs less names *containing* `bld_unit_`, `view_unit_` or `temp_unit` (`strstr`: docs/37 had *beginning*), four 242 × 21 rows, a picked design fitted again in the file's order (`0x10055190`); the M11 save/load row is gone. The engine saves to the player's own folder, never the install (`03e9eb3`, `6f57c55`): a departure, `--save-to-game` for the game's ([37-designer](docs/37-designer.md#the-buttons--read-and-seen)).

**Boarding, flying and getting out**

- [x] ~~[M11] The heading the hero is given on leaving, read as (F.x, −F.y) under an assumed matrix layout; the engine turns the hero to face the bot, as the recording shows.~~ — closed 2026-09-21: the layout is read, and **(F.x, −F.y) stands**. `IControl` slot 12 copies all sixteen words unchanged into the hull's `+0x264`, `+0x2a4` and `+0x2e4` (`Control.dll:0x10004690`, checked), `IGameObject` slot 7 copies it on into the world matrix (`AniMesh.dll:0x1000212b`), and the own panel's camera reads a unit's facing from its elements 1, 5 and 9 (`iron3d.dll:0x1004186f`–`0x10041884`, checked on a continuous decode). So the hero faces the direction to the bot mirrored about x: toward it from the places due ±x, away from it due ±y, side-on from the diagonals. **The recording did not discriminate** — its exit is from place 0, due +x, where both readings agree — and **the engine was wrong**: it now takes the read heading ([39-boarding](docs/39-boarding.md#leaving--read)).
- [x] ~~[M11] Which of a turret's nodes the boarding test's property `0x52` reads the life of.~~ — closed 2026-09-21: **the body, node 1**. `0x52` goes through interface `0x202` slot 3 to `IDeviceManager` value `0x400` (`0x1002bc3c`), which asks `ILifeSystem` for the life over maximum of the node the device keeps at `+4`; the loader rebases that node for every part but the chassis so that node 0 is the socket the part hangs on and node k the part's first node − 1 + k (`0x1000906a`–`0x10009095`). *Measured* over 64 class-1 components: all 58 in `turrets.rlb` and `r_l_06`'s name node 1, the 5 animals' node 0, and all 60 turret parts have a 1-hit-point socket as node 0 and their body at node 1. **The engine was wrong**, reading the socket, and now reads the node the first class-1 component names ([39-boarding](docs/39-boarding.md#the-turrets-life-is-its-bodys-node-1--read-and-measured)).
- [x] ~~[M11] The name a bot's gun takes in the weapons list (`0x1008a470`, not followed).~~ — closed 2026-09-21: the device's part (`+8`), that part's `objects.rlb` name (`AniMesh.dll:0x100025e0`), the first `TRFB` entry in the player clan's tree matching it without regard to case (`MisLoad.dll:0x10002a40`), and that item's `TRF7` short code (`0x10002e70`), else NONAME; *measured*, no part name appears twice in any of the 29 trees. The recording's "LAL365" is **LRL36S**, the game font drawing R much like A. The engine was right for guns fitted as parts and wrong for the 7 turrets whose guns are built into their controller, which it named NONAME; fixed ([35-hud](docs/35-hud.md#the-weapons-list--read-and-measured)).
- [x] ~~Why the recording's bot came to the hero between 236 and 249 s: an order the player gave, or its own behaviour after production.~~ — closed 2026-09-21: **neither — the hero walked to the bot.** Frame by frame, once a second from 234 to 248 s, the Large Factory grows in view while the bot hovers off the factory's end, its shadow under it. That the bot had finished its escape and stood with no order was already in docs/36 ("[escaping]" at 232.5 s, "[no order]" by 234.5 s). The engine was right ([39-boarding](docs/39-boarding.md#against-the-recording--seen)).
- [x] ~~How a boarded hero's route follows the bot, and what a receiving machine does with `0x3f1` and `0x3f2`.~~ — closed 2026-09-22, both halves, and **a premise of docs/39 overturned**: the detached hero is **not frozen** and **is ticked**. The game frame (`iron3d.dll:0x1005e680`), after the calculation, reads the bot boarded from foot (`CState +0x28`); a bot the component test `0x10076d30` passes has its kind-2 world matrix read, element 7 (y) lowered by the bot record's `+0x94` — its node sphere's radius, 11.84 on the L-2f — and given to the hero's object (`0x1005ead6`–`0x1005eb39`, re-read by the coordinator). World3D's queue takt sends the game takt to every object in its registry, not down the tree (`0x10006c49`–`0x10006c85`), so the hero's behaviour still reports: route 4 reads (bot x, bot y − 11.84, bot z). A bot the test refuses — gone, or its turret's body at no life — rolls the stack back to mode 0 (`0x1005eacf`). **The engine was wrong** on both: it reported the hero from the bot's own place, and kept the hero aboard a bot whose turret was shot off; two install tests pin them. `0x3f1`/`0x3f2` travel as type-9 network packets, and the receiving queue's consumer (`World3D.dll:0x10006460`) dispatches codes above `0x3ee` **by subtraction** (`0x1000695d`), which is why no immediate sweep found a compare: `0x3f1` detaches the object with that id, `0x3f2` attaches a parentless one under the root; single play sends neither ([39-boarding](docs/39-boarding.md#boarding--read)). Still open: what else reads the boarded hero's object place, other clans' sensors among them — the engine keeps the hero's own body where it boarded.

**Walking into the Large Factory**

- [x] ~~[M11] What `CBuilding` does to a door's or a pod's switch word as it files the item, and the capsule a door is measured against (`Terrain.dll:0x1005a27f`).~~ — closed 2026-09-21. **Every door and a building's first computer are switched to 2**, shut, through `IItemManager` slot 6 (`Terrain.dll:0x10058532`, `0x100585a1` → `Control.dll:0x1002ed30`, property `0x600`); a **second** computer is never switched (index 0 only, `0x1005858e`, `0x100577ad`), so on the 18 of `fortif.rlb`'s 30 controllers that carry one it keeps the constructor's 5 and wraps for ever. The capsule is `IJointMesh` slot 5 (`AniMesh.dll:0x1000fd60`): the node's level-0 box stood on its longest world axis, from one end face's centre to the other's, as wide as half an end face's diagonal — narrower than the node's sphere on **56 of 56** door parts, 7.73 against 13.48 on the Large Factory's front door. The engine had the doors right and the capsule wrong, and dropped the second computers; both are fixed. Left as a stand-in: that `CBuilding` works a door's holds out as each child moves (event 1, `0x10059f40`), where the engine does it every tick ([24-motion](docs/24-motion.md#walking-into-a-building--read-and-measured)).
- [x] ~~[M11] Where a gathered face's batch word comes from (the engine lets movers **and rounds** through the `DEFAULT` and `PORTAL` materials' faces).~~ — closed 2026-09-21, the ray walker's half **stale** (`1f75a53`, the third round): it is the stream-13 batch record's first dword, handed back by `IMesh2` slot 3 (`AniMesh.dll:0x100134d0`). *Measured*, re-counted by the coordinator: 8 is on **633 of the 15153** batches of the ten mesh archives, every one a `fortif.rlb` `DEFAULT` (567), `PORTAL_001` (36) or `PORTAL_004` (30) batch, and `0x200` on **none**. So the engine's material names picked the right faces for the wrong reason; movers and rounds now pass faces by the word, and a hider whose word carries 2 hides both ways. Narrowed beside it: whether the draw's word at `Terrain.dll:0x1004552a` is the same dword, which would file 2953 batches see-through ([24-motion](docs/24-motion.md#the-ground-inside-a-building--read-in-part-and-measured)).
- [x] ~~[M14] What a basement builder's piece is, and where the band is triangulated, and so how far a cut face's own triangles reach toward the outer ring.~~ — closed 2026-09-22. **A piece is one triangle of a quad-edge constrained Delaunay triangulation**: the walk `0x10007f00` hands over each left face once (`0x10003510`), sites are swapped under the in-circle test except across constraints (`+0x60`, set at `0x100057e5`), and labels flood across unconstrained edges (`0x1000b710`). **A cut face keeps exactly its part outside the outer contour**: each is triangulated from its own corners (`0x1000f7b0`) with the contour in, labelled 1 left and 2 right; all 60 `.bas` rings wind anticlockwise, and the first builder takes the 2s (`0x1000bdd9`). **The band is one triangulation of the ring between the contours**, the outer cut at every landscape edge it crosses (`0x1000fac7`); `AddBasementFaceProc` takes the 1s (`0x1000cd69`), and `StartCheckMaxBasementAngle` (`0x100150f0`) builds the same band from the raw rings, closing M12's triangulation half. **The engine was wrong**: it stitched the rings corner to corner, covering ground outside a concave ring; `cdt.rs` now builds the read triangulation, and an install test finds the band tiling the ring exactly on all 167 placed buildings ([03-terrain](docs/03-terrain.md#what-a-basement-face-wears--read-and-measured), [32-builder](docs/32-builder.md#the-test-isplacementvalid--read)).
- [x] ~~[M11] Whether the slope brake reads a building's stair faces, and who sets a collision object's flags, so which movers keep the floors in their push-out.~~ — closed 2026-09-21 by read. The collision object's slot 5 (`Control.dll:0x1001f670`) sets its flags, called only by the control dispatchers at the end of message 4 (`0x10007bd0`, `0x100319c5`) with the current state's word: flag 8 unless the word carries bit 4, flag 4 from `0x4000000`. None of the 206 controllers with states mixes bit 4 — 99 carry it in every state, the hero's `r_h_02` in 105 of 105, and 107 in none, all 9 flying chassis among them — so **walkers lack 8 and flyers keep the floors**. The slope brake reads a building's floors too, driven by the lift's averaged ground normal (body `+0x194`, `98dcfbb`). **Neither read is in the engine**, and the new line below says why ([24-motion](docs/24-motion.md#the-ground-inside-a-building--read-in-part-and-measured)).
- [x] ~~[M14] Who calls `CBuilding::PortalDrawNotify` (`Terrain.dll:0x1005a5d0`) with a portal's face, and so which cells a building draws.~~ — closed 2026-09-22. **CShade's portal fade, `0x1002c4d0`**: `CBuilding` answers `IMesh2`'s interface `0x18` with its `+0x10` interface, whose slot 4 is the notify (`AniMesh`'s own is a bare `ret 4`, `0x10007e80`); CShade's mesh draw (`0x10044ea0`) sends every batch whose record's first dword carries 8 to the fade (`0x1004501e`–`0x1004502d`, re-read by the coordinator), which sets the primitive's alpha from the camera's distance to its first vertex and the field of view, and calls slot 4 or not (`0x1002c65e`). By the batch word: `0x10`, 66 green signs, fade in from √d = 2.6/f to 7.8/f, the room always drawn; `0x40`, 170 open quads, alpha 0, the room always drawn; neither, 397 doorways, fading to black between 1.3·`PortalNearDist`/f and 1.3·`PortalFarDist`/f (75 and 95 at 1.3 rad), the room not drawn beyond far. **A premise of docs/24 overturned**: `PortalNearDist` and `PortalFarDist` are not read by nothing — they are settings entries copied to CShade `+0x1660`/`+0x1664` (`0x10046d77`, `0x10046d8e`) and read by the fade. **The engine was wrong** and dropped every portal quad; it now draws signs and doorways translucent by the read fade, which shows the recording's green arrow signs in the corridor to the pod (98–99 s) ([24-motion](docs/24-motion.md#a-building-is-drawn-cell-by-cell-through-its-portals--read)).
- [x] ~~[M14] Where the node matrix an action-3 effect takes as its frame (`Effect.dll:0x1000625a`, property 2) comes from.~~ — closed 2026-09-21: `IAnimation` (interface `0xb`) slot 4 asked with 2 (`AniMesh.dll:0x10005320`) returns the node's `+0x20`, the world matrix the pose walk builds — the object's world matrix times the node's chain of keyed poses. So **the game plays a door's sound at the node's origin**, 30.8 m from the three factories' side doors and more than 10 m off on 68 of the 112, and all but inaudible in the doorway. The engine had moved it to the level-0 sphere's centre and now follows the read; its audio install test is inverted to pin that. The sibling row, the matrix `AniMesh` slot `0x10` hands the effect manager for a beam's muzzle, closes with it: node 0's world pose was right ([13-control](docs/13-control.md#a-buildings-load-group--read-and-measured)).
- [x] ~~[M14] The 16 of 167 placed buildings that do not stand on their cut contour's mean height, and
  who calls the flag's setter, slot 12~~ — closed 2026-09-30, and **it overturned a premise**: this
  queue had closed "what reads a building's start flag back" on 2026-09-18 as *nothing*. The insertion
  asks the building for `IBuilding` at `Terrain.dll:0x1000e5fc` and keeps it on the stack (`[ebp-0x7ec]`),
  and calls **slot 13** through it twice, at `0x10011147` and `0x100147cc`. The first lays the final
  outer contour at its mean and the second takes the drop off the matrix, each **only on 1**.
  `ArealMap.dll:0x10015a0d` calls slot 12 with 2 on the start flag, so **the flag keeps a building at
  its file height**. The earlier pass followed an answer only while it stayed in a register.
  *Measured*, with the engine's cut contour: **150 of the 154** buildings without the flag stand on
  their mean to a centimetre, the other 4 within 0.14; **12 of the 13** with it stand off it by 0.2 to
  5.2, the 13th 0.004. Those 12 are the 8 bridges and C02 Mission 03's two mines, factory and
  generator. The setter's caller was already read
  ([04-missions](docs/04-missions.md#the-start-flag-keeps-a-building-at-its-file-height--read-and-measured),
  [03-terrain](docs/03-terrain.md#a-building-is-set-down-on-the-mean-of-its-contour--read-and-measured)).
  The four unflagged ones are queued.

## Mission 03, *The Field Base*

Added on 2026-09-15 against `950af7e`, after M12 made the mission winnable end to end.
[40-command-mode](docs/40-command-mode.md#not-established), [41-commander](docs/41-commander.md#not-established),
[42-selection](docs/42-selection.md#not-established), [32-builder](docs/32-builder.md#not-established),
[23-economy](docs/23-economy.md#not-established), [31-packages](docs/31-packages.md#not-established),
[34-progression](docs/34-progression.md#not-established), [37-designer](docs/37-designer.md#not-established)

**Where the engine is weaker than the game**

- [x] ~~[M12] The construction sphere's look: its effects' time mode 0 and slot `0x1c`~~ — closed 2026-09-22: mode 0 is only an idle instance's resting mode, which action 10's start mode replaces (`Control.dll:0x10002f9d`); nothing in `Control.dll` calls slot `0x1c` (control: the same sweep finds 22 calls at 13 slots through the controller's manager, the start at `0x10002fad` among them), and all 133 blocks of the four sphere effects open above t = 0, so an idle one shows nothing. The dome showed partly because the engine laid it on its side, half under the ground, and looped it; it now stands up and runs in the mode its code starts, checked against the recording's frames at 203–237 s ([11-effects](docs/11-effects.md), [32-builder](docs/32-builder.md#the-construction-sphere--read-and-measured)).
- [x] ~~[M12] The commander's satellite map (`0x10073830`): its title bar and exit icon beyond their place, and the `+0x230` rectangle the column's click tests first.~~ — closed 2026-09-28: from (374, 43) the pen draws an `ending_text`, a 226-wide box holding 5074 in `#37ff37`, the `ending_text` mirrored back from 5 past the box, and the exit button in its normal variant, `exit_icon` at (+15, +3) tinted `0xfff0f0f0`. `+0x230` is that button, stored as the pen reaches it, (605, 43)–(640, 63); the column's click tests it second, after the lock, and closes the map (`0x10084362`–`0x1008437d`), its tooltip 6169 *Close*. The engine drew a page header — a white title and the pressed button — and took no click on the exit; it now draws the read pieces, the title centring at 490.7 against the recording's 491.3 at 84 s. The heading line's `+0xd8`/`+0xdc` stand-in closed with it: the x and y of the unit's forward column, copied at bind and each takt (`0x10075761`–`0x1007576d`), as the engine already drew ([35-hud](docs/35-hud.md#the-commanders-variant--read-and-seen)). Left: [below](#mission-03-the-field-base).
- [x] ~~The warbot designer's save name field and load list: not built, and how they work is not established.~~ — closed 2026-09-29, **stale**: already in the engine since 2026-09-22 and this file was never ticked. [37-designer](docs/37-designer.md#the-buttons--read-and-seen) read both — `stdGetValidRobots`'s list of the `units/*.dat` that read as designs, fit the factory and are researched whole, passing over names holding `bld_unit_`, `view_unit_` or `temp_unit`, four 242 × 21 rows from (200, 320); the 16-character field in (270, 410)–(370, 430), saved when the typing stops — and `6bed1b9` built them (`NameField`, `LoadList` in `cockpit/designer.rs`), `03e9eb3` writing a saved design to the player's own folder rather than into the install.
- [x] ~~A Mission 03 transport walks about 33 m/s where docs/23 gives 24~~ — closed in M13: the live limits now come from the unit's engine and load, and it walks 23.98. The recording's round being 13 s longer than two full-speed walks is still open.
- [x] ~~Why the patrol took about 85 s to reach the base in the recording, when a straight flight is 37–44 s; and how high a patrolling flyer flies, which decides whether its 3D attack limit ever holds.~~ — closed 2026-09-29. **The premise was stale**: the 37–44 s took the chassis's authored 44.4 m/s, which [31-packages](docs/31-packages.md) had already replaced with the live top speed; only docs/34 still carried it. The 85 s is **84**: the orders go out on the Mission run that completes objective 3 at 334 s (function-15 nodes 120, 124, 128) and the fight starts at 418 s. The three flyers fly at 0.8 of 17.8–20.0 m/s (14.27, 16.02, 14.90) along ways 1,407, 1,716 and 1,419 m long at 15 m over the ground, 6.2–10.8% longer than straight; off the satellite map (fitted to five building icons) the pair comes in at 12.7 m/s across the ground and the single flyer at 14.2, against 13.1–13.8 and 14.4 predicted, and at 418 s the pair is about 275 m short of its place and about 195 m from the bunker, inside the HFTB's 250. **The height is read**: of `MWalker::SetTarget`'s 15 calls, seven build their own place, and the patrol's loop (`Behavior.dll:0x1002dd74`) leaves the place word `+0x14` at 0 (`esi` cleared at `0x1002d916`, stored at `0x1002dcec`), as do attack (`0x100279c0`), go by place (`0x1002b3e8`) and transport (`0x10032fc7`), so a patrolling flyer flies 15 m over the ground (115 over a building); follow carries 1 (`0x1002affc`), at the leader's height and 5. So the 3-D limit holds near the place: 13,464 of the 14,641 1 m points of the loop's square about (1124, 783) lie inside the 120 sphere about z 0, 7,601 (51.9%) about (606, 993), and nowhere on the way in. The engine's patrol was right; a flying follower was wrong, flying the ground + 15, and now keeps its leader's height and 5, pinned on Mission 01's captured flyers ([24-motion](docs/24-motion.md#a-flyers-walk-points--read-and-measured), [34-progression](docs/34-progression.md#seen-in-a-recording-1)).
- [x] ~~Which fight-module bar a building's guns must clear. With the walker's 0.85 the Small Bunker's flamers only fire at a unit close to its own ground level, never at hovering flyers, so the warbots do the fighting.~~ — closed 2026-09-28, **stale**: [29-weapons](docs/29-weapons.md#how-the-ai-fires--read) read the bar on 2026-09-19 (`a63320a`) and this file was never ticked — 0.85 for a building (Type `0x80000000`) or a machine whose live forward top speed is 0.5 m/s or more, 0.45 for a flyer or anything slower (`0x10024e7a`–`0x10024ec5`), the score strictly above it (`0x10024f07`). The engine's stand-in had not followed and gave every non-flyer 0.85; it now gives 0.45 to the 28 placed tower units on seven maps, authored at 0.2 m/s, and to any machine damaged to a crawl — outside free fire only, the C01 M03 tower tests still passing; Mission 03 places no such unit. **The premise had gone stale too**: the flame `bf_f_01` carries frame flag 8, which the distance score (`0x1001ba00`, in the engine since before this round) answers with 1 before the height term, so the bunker passes 0.85 at any height its sight reaches, −15° to 75°. A new install test holds Mission 03's flyer 15 m over the ground at 80 and 40 m and 40 m up at 80 m, and the player's bunker fires 4, 5 and 4 flames in 6 s.

**Command mode and the panel**

- [x] ~~[M12] Whether the character handler sees Esc before its binding leaves command mode~~ — closed 2026-09-22: the window routine (`iron3d.dll:0x100a0e30`) hands a key-down to the listener chain before the bindings in every view state but 0, and the game's listener's slot 0, `0x10070db0`, is the **key-down** handler (`WM_CHAR` goes to slot 2). Esc peels one thing a press — objectives, placement, a message box, the wingman menu, the game menu, then in state 2 the map and the page — and only then does 735 leave. The engine peeled placement, map and page in command mode only; it now follows the whole order in every view ([40-command-mode](docs/40-command-mode.md#input--read-and-measured)).
- [x] ~~What view state 4 and the second camera at `+0x68` are for; cursors 7 and 8; the globals `0x1010bf7c`–`0x1010bf80`; the display's slot 12, which picks the system's cursor over the software one.~~ — closed 2026-09-29, all four. **View state 4 is the level's second camera**, set from six sites: the hero's loss (`0x100a4e50`, from the unit record's removal `0x10075612`), a level with no hero (`0x100a3da8`), a lost network session (`0x1005fb05`), the auto-demo's set-up and director (`0x100a2971`; `0x1002c673`–`0x1002caa7`), and Alt+D (`0x100719be`) under `DEBUG_KEYS_ON`, which the install's `Iron_3D.ini` does not carry — the one writer of `+0xaf5`. In single play it is the view after the hero dies: the hero's matrix, lifted to 16 over the highest surface under it (mask `0x41a`), looking along its x axis, and still, since its update needs a network game or `+0xaf5` (`0x10037a70`). **Cursors 7 and 8**: of the state setter's 27 calls one stores 7, the band (`0x100714bf`), one 8, the Build row's building (`0x10079e74`); under the band the camera keeps the edge flags it held (`0x10037a8f`, `0x10037c52`). **The globals** are the left button's press — held `0x1010bf7c`, an unread byte, moved `0x1010bf7e`, stamp `0x1010bf80` — cleared by all eleven transitions into a command view. **Slot 12** is `services.dll`'s display byte `+0x4fe`, set from `FORCE_SOFTWARE_CURSOR` read as 0 and held to 0 without `DDCAPS2_CANRENDERWINDOWED` (`Ngi32.dll:0x1000552d`); the install sets 1, so the game draws its own cursor, as the engine does. The engine was wrong on three — it kept the dead hero's eye, re-read the edges under the band and kept a press across a transition — and all three are fixed; Alt+W, C, B, R and P turn the commander's page in a command view (table `0x10071a08`), now too ([40-command-mode](docs/40-command-mode.md#view-state-4-the-second-camera--read)). The `.ani` system cursor stays a stand-in for a setting of 0.
- [x] ~~What interface `0x201` slot 9 with (`0x20`, 1) and message (6, 7, 0) do to a bunker left for another view or for telepresence.~~ — closed 2026-09-29: **nothing new — they say again what already holds**. `0x201` is the unit's Wizard; slot 9 with (`0x20`, 1) gives word `+0x20c` (turret, guns, arms, builder and the fight module) to the AI, and (6, 7, 0) sets the Wizard's mode to the AI's. Every exit from a tower's *Manual* sends the same pair, and the four entries into mode 6 send (`0x20`, 3) and (6, 7, 1) — the only four sites of the ten in the mode handlers that push 1, the other six pushing 0. Nothing on the way into mode 4 sends a bunker anything, and `World3D.dll`'s `CreateObject` sends every object (6, 7, 0) as it is made (`0x10007dae`), so a bunker's guns are its AI's while it is the command view's building, during telepresence and after it is left. The engine was right; an install test now pins the bunker firing in all three ([40-command-mode](docs/40-command-mode.md#what-a-bunkers-guns-do-in-command-mode--read)).
- [x] ~~[M12] What telepresence's auto-driver levels 1 and 2 hand to the AI~~ — closed 2026-09-22: the takeover (`0x10074ff0`) writes the unit's `Wizard.dll` words through slot 9. Level 1 hands the AI the walk alone (`+0x200`), and the turret, guns, shields, sensors and engines stay the player's; level 2 hands it `+0x200`, `+0x20c` and `+0x208`, sends mode 0 so the rest follow, and clears `+0xa2`. The engine had the player drive the unit whole at every level; it now runs the AI walk at 1 and 2 and the fight at 2, pinned on Mission 03 ([40-command-mode](docs/40-command-mode.md#telepresence-mode-2--read)). Left: [below](#mission-03-the-field-base).
- [x] ~~[M12] A unit record's `+0x30` and property `0x207`, and a building's `+0x30`: they pick and tint the panel's icons.~~ — closed 2026-09-28: `+0x30` is the size class, written for unit and building records alike by one bind from property `0x201` (`0x1007e538`–`0x1007e549`). A unit's icons (`0x10077120`) are tinted through the jump table at `0x100773ec` — 1 `0xffffe7ff`, 2 `0xffff8080`, 3 `0xff80ff80`, 4 `0xff8080ff` — and a building's (`0x100344e0`) through `0x100347dc`: 1 white, 2 red, 3 green, 4 and 5 blue. The first unit icon is picked by the Type compared whole; `0x207` is the chassis profile's `ChassisType`, and it picks the second through `0x100773fc`. All 296 placed units fall inside the tables (sizes 13/140/75/68, ChassisTypes 58/127/58/53), and the 167 buildings are 107/27/25/8 for sizes 2–5. The engine gave every unit the flyer's second icon and every building row red; it now picks and tints as read, which the Mission 04 recording's HQ and helicopter rows at 84 s match ([41-commander](docs/41-commander.md#the-box)). Left: [below](#mission-03-the-field-base).
- [x] ~~[M12] The chat overlay; the game menu's screen (mode 7); tooltips.~~ — closed 2026-09-29 as far as single play needs. **Chat** is enabled only in a network game (`+0xe4`) and stays disabled. **The game menu is built**: F3, Esc on foot or the column's button open it and F3, Esc or *Resume game* close it; showing it pauses the game (`0x1005f620` → `PauseGameTime`) and the screens' draw draws it alone (`0x1008d211`) — the screen darkened with `0x99000000`, a box (200, 150)–(440, 330) filled `0xcc328032`, 5085 *Game Menu* over *Resume game*, *Save game*, *Load game* and *Quit game* (3080–3083) at y 210, 235, 260 and 285. *Save game* and *Load game* are disabled in the training campaign (`+0xe6`), so Mission 03 offers Resume and Quit. **Tooltips are built** from docs/37's read: the manager is cleared each frame (`0x10060aac`), 23 of its 26 callers hand it a text, and the engine wires the column, the unit box, the building rows, the map's *Close*, the factory panel and the designer ([39-boarding](docs/39-boarding.md#the-game-menu--read), [37-designer](docs/37-designer.md#the-buttons--read-and-seen)). The save page and the shell that *Load game* and *Quit game* hand the mission to are a line [below](#mission-03-the-field-base).
- [x] ~~[M12] What slot 7 of a unit's object does 0.6 s after *Explode!*~~ — closed 2026-09-22: the record's `+0x48` is the unit's `ILifeSystem`, and its slot 7 is the kill (`Control.dll:0x1000eb70`): unless the invulnerability byte `+0x5ac` is set, node 0 loses the object's whole maximum through `0x10010f30`, past the armour. The takt (`iron3d.dll:0x10075680`) runs it once more than 0.6 s (`0x100e64ec`) have passed since the click's stamp. The engine drew the button and did nothing; it now blows the unit up, pinned on Mission 03 ([41-commander](docs/41-commander.md#explode--read)).
- [x] ~~[M12] What `0x10034230` accepts for an Upgrade row, and what Type `0x80000200` is~~ — closed 2026-09-22: it refuses five Types (generator, hangar `0x80000040`, main teleport, bridge, ruin; `0x10034282`–`0x100342b5`), then asks property `0x20c` at 0, an owner word other than `0xfffe`, a level `0x209` below the scheme count less one, and **every part of the next level researched**, by the count a Build row uses (`0x1008b130`) — read now, where it was derived from the recording. The click sends each selected unit to its nearest accepted building; the engine sent all to the first. `0x80000200` is `BUILDING_MAINTELEPORT` (`varset.var` line 169) ([41-commander](docs/41-commander.md)).
- [x] ~~The research panel's contents and controls (page 4)~~ — read and built in M13 ([41-commander](docs/41-commander.md)).
- [x] ~~[M12] The routine that names a building (strings 6031–6098)~~ — closed 2026-09-22: `iron3d.dll:0x100338d0`, from the building record's slot 1, by the behaviour's Type and its size class (variable `0x201`: the root record's fourth letter, l 2, m 3, b 4, e 5), with 6205 *Unknown* for the rest. 167 of 167 placed buildings across the 29 missions get a name, none *Unknown*; the engine's size-letter rule was wrong on 31 of them — the 4 enhanced institutes, and 19 bridges, 3 ruins and 5 teleports unnamed ([35-hud](docs/35-hud.md#name-and-status--read-and-seen)).

**Selecting and ordering**

- [x] ~~[M12] An areal's first flag word (`+0x20`), which decides where a walker may be sent~~ — read in M14: it marks a walkable areal, the only kind the areal map links, and the engine's walker now keeps to them ([24-motion](docs/24-motion.md#the-global-path--read-and-measured)).
- [x] ~~Why the recording shows `PLACE`, not `GUARD`, over the bunker's roof at 190.5 s. **Narrowed** 2026-09-22, *inferred*: the pick takes a building only within 0.7 of its radius (read, below), so over a roof's outer part it finds nothing and a valid place shows `PLACE`. Left: the recording's cursor measured against the bunker's sphere.~~ — closed 2026-09-29, **the ninth round's inference overturned**: the ray passes 11.6 from the bunker's centre, well inside 0.7 of its radius; the pick found nothing because **the eye stood inside the bunker's sphere** (step 3, `0x100362d8`). The sphere is the agent's own (interface `0x18` slot 9, `AniMesh.dll:0x10014580`), its six parts' header spheres joined as `0x10009510` joins them: 52.58 about (1260.92, 813.89, 84.76). The command camera, fitted to the eight corners of the roof's two `B_LBL_01` signs in the 190.5 s frame, stands at (1259.89, 816.21, 136.09), yaw 2.623, tilt 0.140, 1.4 pixels' rms — 51.39 from the centre, inside in 400 of 400 fits with each corner read 2 pixels astray (50.99–51.73), and 1.7 over the camera's read floor over the turret. The engine was wrong: it took the box over the node spheres, 87.23 about the placement, which held the eye inside up to z 167 where the game lets it out at 137.3; it now takes the agent's sphere, pinned by `PLACE` at the fitted pose and `GUARD` 10 m higher ([42-selection](docs/42-selection.md#low-over-a-roof-the-eye-is-inside-the-buildings-sphere--measured-and-seen)).
- [x] ~~[M12] Which objects the world's classes 3 and 4 are, and the object pick's order and nearest-hit rule (`0x100361a0`)~~ — closed 2026-09-22: class 3 is the buildings and class 4 the units (`World3D.dll` files each object under the class it answers, and builds it into its id, `0x1000562a`, `0x10005636`). `0x100360f0` walks class 3 at **0.7** of the radius and then class 4 at **1.0** (`0x1003614b`, `0x10036171`) — the engine and docs/42 had the shares the other way round — sharing one best distance; per object it passes one whose sphere holds the eye and keeps the centre strictly nearest **the eye** (`0x100363e3`), not nearest along the ray, a tie keeping the building. Engine fixed ([42-selection](docs/42-selection.md)). Left: [below](#mission-03-the-field-base).
- [x] ~~Whether the band is filled or only outlined; double clicks; whether a group sent to one place spreads out.~~ — closed 2026-09-29, all three. **Outlined**: the band goes to the GUI server's slot 3 (`services.dll:0x10001a60`, through `getGUIServer`), not `IDisplay`'s, which hands the renderer five vertices as `D3DPT_LINESTRIP`, one window pixel wide; the control is the server's slot 4, which fills with a four-vertex strip. **No double click reaches the game**, a negative with its control: the one window class (`RegisterClassA`, `iron3d.dll:0x100a0818`, the only importer of 22 modules) has style `0x23`, without `CS_DBLCLKS`; nothing imports `GetDoubleClickTime`; and the window handler's table (`0x100a1014`) and World3D's two converters send `0x203` and `0x206` to their defaults, where the same dispatchers route `0x201`, `0x202`, `0x204` and `0x205`. The designer's double click rests on its own 0.2 s timer (`0x10047f10`). **A group does spread**: each unit gets the same place (`0x1007931e`), but a go that ends at a place as the unit's only order (`IBehaviour` slot 4 answers 1, `Behavior.dll:0x1002b83b`) appends `PATROL` of radius 150 about it (`0x1002b8a8`–`0x1002b8d7`), so each circles a loop of its own. The engine was right on the band and the clicks and wrong on the spread, now fixed — for every lone go to a place, the AI's and the scripts' too ([42-selection](docs/42-selection.md#spreading-a-group--read), [31-packages](docs/31-packages.md#what-each-package-does--read)).
- [x] ~~The pending picks not traced: kind 2 (attack-target mode) and the orders kinds 2–5 give.~~ — closed 2026-09-28: **kind 2 is never opened**, so the attack-target mode, pick mode 1, is never entered: the executor stores 1, 5, 4 and 3 to `+0xac` and never 2 (a sweep of all 10 stores to `+0xa8`/`+0xac`), and pick mode 1's one writer is kind 2's own first stage (`0x10079c0f`). Kind 5, Capture building, is never opened either: the executor's one caller is the order row's click (`0x1007b65e`), and the HQ table's 22 rows carry no command 4 or 5 — Attack and Capture building are the wingman menu's. Opened, kind 2 would give `ATTACK` only with both its flags set, which no click does, and kind 5 `ORDER_ROBOT_CAPTURE`. **Guard, kind 4, gives `PATROL` with parameter 300** (`0x1007a08c`) on a unit or a place; a building under the cursor answers kind 9, which acts on a unit only (`0x10090337`), and a building guarded from the free mode gets 100. The engine already could not reach 2 or 5, but gave both guards 0, took a building in the Guard pick, and turned an open page to 0 on a unit with none; all three are fixed and pinned through the bunker's pod on Mission 03 ([42-selection](docs/42-selection.md#what-opens-them-and-what-never-does--read)). One quirk is a line [below](#mission-03-the-field-base).
- [x] ~~[M12] A unit marker's gap figure (the record's slot 5), its bar frame on page9, the clan's sign, and where its name, icon and bars stand~~ — closed 2026-09-22: slot 5 (`0x10075650`) answers 60 over the camera's distance to the unit, never below 0.3, and the gap is 44 times it; the clan sign (`icons` y 224, cell by clan) and the class icon stand right of the right bracket, the page9 frame (0, 0, 27 × 16) under the left one holds a green life bar over an orange battery bar of the unit's real fill, and the name, own clan only, stands over the left bracket (`0x10077d80`); signs and name are drawn for no nature or neutral clan (`0x10077f46`). The engine's projected-radius gap, blue box and full battery bar are gone ([25-sensors](docs/25-sensors.md)).

**Placing and building**

- [x] ~~[M12] The site test's path search and its hall-way areal test (interface `0x303`, vertex bit 1)~~ — closed 2026-09-22: `0x303` is the hall way (`AniMesh.dll:0x100034fe`, `0x100018fc`), and the path is the walker's own search (`0x10020910`) from the builder to the sphere's centre, not flying and gated by the builder's size class; step 6 asks every exit to stand on an areal whose `+0x20` word is set (`0x1000bf65`). All 12 first buildings carry a hall way, 55 exits. On Mission 03 a storage passes at 189 of the 205 grid sites that passed without the two steps, all 16 refused for an exit off walkable ground (3 with no path besides; the path alone refuses none); control: the posing puts Tut_2's exit 67 where docs/24 has it. The engine passed every site; it runs both steps now ([32-builder](docs/32-builder.md#the-test-isplacementvalid--read)). One consequence is a line [below](#mission-03-the-field-base).
- [x] ~~What the pick's query record (first word `0xa`) asks the world for, so which objects stop the cursor's ray.~~ — closed 2026-09-28: the record is `[0xa, 0 × 7]` (`iron3d.dll:0x10035e82`), classes 1 and 3 — the landscape and the buildings — with nothing excluded, to `IWorld` slot 7 (`0x10035ec2`); only the hit's point is kept. So a unit, a tree or a stone never stops the cursor's ray, and a building's shell and a lake's surface do. Control: the one other site that sends a record to slot 7 after `GetWorld`, the outer camera's `0x41a` (`0x10038699`), does admit units. All 167 placed buildings carry the Type's top bit and all 296 units clear it, so they load as classes 3 and 4. The engine used a round's query, so units and scenery stopped its ray and water did not; the pick and the building ghost now share one cursor point, with docs/32's 0.001 map margin ([42-selection](docs/42-selection.md#what-stops-the-cursors-ray--read-and-measured)).
- [x] ~~What the game's `+0xe4` byte is, under which a site within 400 of one of the level's records turns red; whether holding `,` or `.` keeps turning the ghost.~~ — closed 2026-09-28. **The repeat was stale**, answered by `de98f00` on 2026-09-22 and never ticked: a repeated key-down is taken as a new one, and 741 and 742 add or take 0.05 each time (`0x100725a7`, `0x100725dc`). **`+0xe4` is the network game's byte**: its only writers are the constructor's 0 (`0x1005c500`) and `Run`'s mode == 2 (`0x1005c74e`), and the one menu write of mode 2 (`0x10029075`) leads to `multi_login`. The `+0x728` records are the clans' 0x68-byte records, `+4`/`+8` the clan's base point from `data.tma` (`MisLoad.dll:0x10001320`); the walk (`0x10033d80`) skips the builder's clan and any clan marked joined. Single play never applies the rule — Mission 03's lode lies 65.1 from the neutral clan's base, and the recording shows the mine green there at 184 s — and the engine does not ([32-builder](docs/32-builder.md#the-model-under-the-cursor)). docs/34's *derived* "a network game" is now read.
- [x] ~~What an unfinished building looks like before the dome.~~ — closed 2026-09-28, **stale**: `85c6f00` (2026-09-22) answered it and this file was never ticked — *seen*, nothing of it: at 230–234 s the camera stands close on the site and shows the builder, the sign's rings, the ray and the dome over bare ground, and the mine stands only from 237 s. This round read how: the controller's action 1 hides the building's whole node tree and action 2 shows it (`IAnimation` slot 8, node flag 1, `AniMesh.dll:0x10005500`; the mesh draw skips a flagged node, `0x10014e57`) ([32-builder](docs/32-builder.md#actions-1-and-2-hide-and-show-the-building--read)).
- [x] ~~[M12] Which state each sphere code opens, where an action-5 effect is placed, and which classes the sphere's kill takes~~ — closed 2026-09-22: played through the read planner on all 30 `fortif` controllers, code 1 starts the sign (mode 2), 2 the dome and the ray (mode 1) and stops the sign, 0 stops the ray and starts the dome in mode 3, 8 and 10 the dome in modes 1 and 3; 2, 8 and 10 kill every 250 ms from 0, 250 and 500 ms. A phase's code −1 is not sent, so a new building holds code 1 and its sign for 35 s, and an upgrade's first phase clears the area. Action 5's frame (`0x10002e0e`) stands on world z, the sphere's radius long, so the ray and dome stand up. The kill's mask is `0x414` — classes 2, 4 and 10 by the `1 << k` table at `Control.dll:0x1003b1a0` — through `ILifeSystem` slot 7, which spares the invulnerable: units, trees and stones fall, an upgrading builder stands. Engine wrong on all of it, fixed ([32-builder](docs/32-builder.md#the-construction-sphere--read-and-measured)). Left: [below](#mission-03-the-field-base).
- [x] ~~Which call sends SuperAI event 2 for a building a builder puts up (the recording counts the mine at 196 s, and it stands at 236 s).~~ — closed 2026-09-28: event 2 is sent from four sites in `iron3d.dll` — the capture (`0x10032ffa`, the control the sweep finds), the loader (`0x10033d01`), game message 1's building case (`0x10060418`) and a building record's first step in the frame (`0x100333f6`) — and only `iron3d.dll` reads the clan table `+0x774`, against 12 other modules that do not. A builder's building goes `CreateObjectFromScheme` → `AddObjectToGame`, which sends message 1 (`World3D.dll:0x100082ea`); on a fresh single-player start that case does not file it, and the record's first step does (`0x1005eaa0` → `0x10033020`, behind byte `+0xa4`). So the mine is on its clan's list **the frame after it is made**, 40 s before it stands: made at 194.5–195 s, the objective at 196 s. The engine already counted it from its making ([34-progression](docs/34-progression.md#what-the-scripts-ask--read-and-measured-1)).

**Economy**

- [x] ~~The clan's minds in the recording: 4 again from 221.5 s, and five warbots built on four free minds~~ — closed 2026-09-22: a build's reservation lasts only until the clan's next takt. The takt (`ai.dll:0x10001780`, every 7–8 s) ends by writing −1 into every mind entry naming no object (`0x100059e3` → `0x10006580`), and a reservation is 0, which no object's logical id is (`ArealMap.dll` bumps its counters before each use, `0x1002b305`; 0 of the 864 placed objects carry id 0); the sweep runs for the player's clan too (`iron3d.dll:0x100a2750`). A completion takes the clan's first free entry (`Behavior.dll:0x1002a7e5`), and one with none free makes no bot. Read every 0.5 s the panel gives 3 from 218 to 221 s and 4 from 221.5 s, and 5 then 3 at 260–261.5 s: five warbots, five minds, the builder gone at 237.1 s. The engine held a build's mind for the whole build; it sweeps at each takt now ([23-economy](docs/23-economy.md)). ~~The 4 before any build~~ — **read** 2026-09-21: every placed robot takes a mind as it is made, the hero, the builder and the transport among them, so 4 of the 7 are free.
- [x] ~~The direction of the ore a mine's and a storage's loading places move by themselves (`0x10019482`, `0x100195b8`), and unit property `0x208`~~ — closed 2026-09-22: every move goes through `0x100155f0` (amount, giver, taker). A mine's loading place gives to a transport or builder (`0x10019580`); a storage's unloading place takes from a transport and gives to a builder (`0x100196cd`, `0x1001973e`), at the smaller of the giver's efficiency × `Transfer_Ore_OffBoard` and the taker's `Transfer_Ore_OnBoard` — 1 a second at a small mine, 5 at a large, and 0 from a transport, which has no class-26 part — shared `dt / n` among the place's occupants (`0x10019493`). `0x208` is `MBehaviour+0xa64`, written only by the constructor and message 7 (`0x10006266`): docs/13's word not simulated in single play, and not among the 15 property names the 864 objects carry. Now modelled; on Mission 03 a transport fills in 19.8 s rather than 20 ([23-economy](docs/23-economy.md)).
- [x] ~~[M12] Batteries and efficiency read from a building's root controller only; the economy timers' random source~~ — closed 2026-09-22: the fill and capacity functions (`Control.dll:0x1002b42b`, `0x1002b4e9`) cover every class-19 slot, and a fitted part takes over its slot, so on 53 of 76 building assemblies the `i_pws_f_*` parts stand in the root's figures: 20/50 becomes 16 held and 1,000 a second, 19.5/51 24/1,500, 20/52 32/2,000; efficiency is unchanged on all 76 (control: the same walk without the parts gives back docs/23's 19.5–20 and 50–52). That a building attaches its parts as a robot does is *inferred* — a stand-in line below. The distribution step draws on `Behavior.dll`'s own unseeded `rand()` (`0x1004ce3c`, ×214013 + 2531011, starting at 1), the power tick's jitter on `Control.dll`'s shared 16-bit shift register, 250 ± 31.25 ms. Engine fixed for both ([23-economy](docs/23-economy.md)).

**Mission 03's script and recording**

- [x] ~~Why `T03_H03` and `T03_H02` never show in the recording.~~ — closed 2026-09-29: **the hero never stands in route 4 or 5**. The script gates each message only on the route test on id 1 and the enter-from-no-route latch — no objective, flag or earlier message is asked, and a voice asked for behind another is queued, never dropped. The radar's two still marks, the transport and the builder (`tut3_p.dat`'s radar is 250 m), each fix the hero: over 143.5–158.5 s, 41 frames whose two fixes agree within 6 m lie on one line with 1.1 m of scatter, heading 339.5° at 14.1 m/s, which passes 8.2–8.3 m north of route 4's corners and 21.3 m south of route 5's; after 164 s the hero stays at the bunker's pod, the command panel up in every frame sampled from 180 to 420 s. The engine's route reports and latch already matched the read ([34-progression](docs/34-progression.md#seen-in-a-recording-1)).
- [x] ~~Which of the generator's exits the recording's hero used (the south one is inferred), and whether a shot door opens sooner than an approach.~~ — closed 2026-09-29. **The south way**, *measured*: the generator's grey mark and the warehouse's blue mark fix the hero, agreeing with the target panel, crossing the south exits' row (model y −64.24) at about x −8, between exit 54 (−13.07) and 51. **A shot beats an approach whenever it is fired from outside the reach**: the hit's opener has no range, where proximity needs a child within its radius and the capsule's. In the recording **the hero shot both doors open before reaching them**: the generator's beam at 110.7 s with the hero about 10 m short of its leaves' reach, the corridor showing by 111.4 s; the bunker's at 166.93 s with the hero about 21 m up the ramp, the door lowering at 167.23 s with the hero still about 16 m back, where proximity would have started it at about 168.1–168.3 s (the target panel's transport distance read every 0.2 s). The generator's south doorway is two class-12 leaves, components 4 (`i32`, node 13) and 5 (`i31`, node 14); their faces' flags exclude them from no round's query, so a shot opens one leaf and not the other, now pinned. docs/24 had the hero at the bunker ramp's top at 164.0 s; it was at the exit then, and at the top about 166.1 s. The engine was right ([24-motion](docs/24-motion.md#a-shot-opens-a-door--read-and-seen)).
- [x] ~~What component property `0x200` is (taken to be the door's node: all 56 door components' nodes match their channels').~~ — closed 2026-09-29: **the component's node, for every class**. The opener walks the control system's own `IDeviceManager` (interface `0x204`, kept at `+0x38`, `Control.dll:0x10007972`): slot 3 the count (`0x10008830`), slot 8 the class word (`0x1002c390`), and slot 6 (`0x1002bb40`) answers `0x200` at `0x1002bc1f` with the component's `+4` and no class test — the `.ctl` record's node. The control: the weighing asks the same slot for `0x200` and then `0x100` and uses the first answer as a node index (`0x1000fbf5`–`0x1000fc31`). It is neither `IAnimation` slot 8's modes `0x200`/`0x201` nor contact flag `0x200`. The engine's `Building::shot` already matched the component's node ([13-control](docs/13-control.md#the-component-record)).

**Left by the ninth round** (2026-09-22)

- [x] ~~[M12] Whether the hero has a parent object in command mode: the object pick passes over any object whose `IGameObject` slot 3 answers nothing, and the engine never picks the hero.~~ — closed 2026-09-28: slot 3 is `CGameObject`'s parent (`AniMesh.dll:0x10017570`), which the pick asks at `0x100361f6`. Of the 36 handlers in the mode table `0x10104b18`, only boarding's detach (`0x100637f3`) and leaving's re-attach (`0x1006392b`) call slots 3–5 — the known pair, the control — and entering a bunker's view from foot (`0x10063ca0`) calls none. So **on foot the pick takes the hero** like any unit (kind 7, `PICK`), though a click on it selects nothing (`0x1007d0d4` refuses Type `0x1020000`); aboard a bot it is passed over; the satellite map's unit walk (`0x10072b70`) has no hero test. The engine never picked it; it now does on foot and on the map, the hero's place aboard *inferred* ([42-selection](docs/42-selection.md#the-hero-keeps-its-parent-until-it-boards--read)).
- [x] ~~[M12] The construction sphere's remainder: a code's groups run at the controller's next 250 ms step, where the engine runs them as the code is sent; its actions 1 and 2; what object class 2, in the kill's mask, is; and how the world query decides what lies inside the sphere.~~ — closed 2026-09-28, all four. **Timing**: `IControl` slot 19 (`Control.dll:0x10004800`) only stores the code, and the machine runs states only while its clock is not ahead of game time (`0x1000c2a5`), so a code waits for the next step and the states on its path play a step apart — all 250 ms on the 30 `fortif` controllers but the code-0 anchor's 5,000, jittered on 29 to 4.375–5 s; the dome turns back 40.25 s in. **Actions 1 and 2 are not properties**: they call `IAnimation` slot 8 with node 0 and modes `0x200`/`0x201` (`0x10002936`, `0x10002954`), hiding and showing the building's node tree. **Class 2** is stored, not returned: an agent's slot 11 answers `+0x6d8` (`AniMesh.dll:0x10002fd0`), which the load sets to 2 from a `WPNS` tag (`0x100031b2`), and no shipped object is one — the five `WPNS` records name `weapon.rlb` members that are not there, and the 864 placed objects are `BTLU`, `FORT` and `STAT`; control, the same search finds the `BULL` record `bld_l_01`. **Inside** is sphere against sphere: `IWorld` slot 3 (`Terrain.dll:0x10025f40`) takes an object whose own bounding sphere meets the kill's in 3-D (`0x10025d10`). The engine was wrong on all four; a building's real controller now plays its codes, and a tree whose sphere reaches in falls on Mission 01 ([32-builder](docs/32-builder.md#a-code-takes-effect-at-the-controllers-next-step--read-and-measured)). Left: [below](#mission-03-the-field-base).
- [x] ~~[M12] Telepresence's remainder: where a bot's auto-driver level is kept between takes; that a level 2 reached by the key leaves the camera the player's; the AI's repair decision and hit reaction at levels 1 and 2.~~ — closed 2026-09-28: the level is the unit record's `+0x9c` and stays there between takes — the take reads it (`0x10075027`), letting go writes nothing, and of 64 stores through `0x9c` in `iron3d.dll` eight reach a unit record, the panel's and the key's among them (the control). Boarding writes no level, so a bot left at 2 by its page is boarded at 2; Y works only in mode 1 or 2 (`0x10072645`). A take at 2 writes neither `+0x210` nor `+0x214`, so a 2 reached by Y leaves the camera, radar, seeker and repair the player's, and a 2 after a letting-go the AI's. At levels 1 and 2 the unit's takt runs: its repair decision switches repair through no Wizard word (`Behavior.dll:0x10019a80`, `0x100067c1`), and a hit asks for its attack at every level (`0x100064b0`). The engine kept one level for the play and dropped every hit on a driven bot; the level lives on each bot now, and repair and hits reach it ([40-command-mode](docs/40-command-mode.md#where-the-level-is-kept--read)). Left: [below](#mission-03-the-field-base).
- [x] ~~[M12] Whether a building's parts are attached as a robot's are: the part loop (`Behavior.dll:0x1001cd40`) is not followed for a building, and the engine's building batteries stand on it.~~ — closed 2026-09-28, **the ninth round's inference confirmed**: a placed building is made by `ArealMap.dll`'s copy of `CreateObjectFromScheme` (`iron3d.dll:0x100a411d` → worker `0x10015290`), a built one by `Behavior.dll:0x1001d440` from `CreateBuilding` (`0x10029286`), and the two workers are one routine twice: a building differs only in a sphere test against the buildings already standing, and otherwise falls through to the robot's loop over the root's children and the part loop (`ArealMap.dll:0x100158d6`, `Behavior.dll:0x1001da86`). `CBuilding` passes every message to its agent (`Terrain.dll:0x10057b55`), whose `0x80000020` case is the robot's attach (`AniMesh.dll:0x10001468`). The engine was right; its stand-in is gone, and a built Small Mine is pinned at 16 held and 1,000 a second ([23-economy](docs/23-economy.md#a-buildings-batteries-are-the-parts-fitted-into-its-slots--read-and-measured)).
- [x] ~~[M12] The power tick's shift register's seed, taken from an `Ngi32.dll` import by ordinal (`Control.dll:0x1000dc34`), and the 65 other callers of `Behavior.dll`'s `rand()` between two distribution steps.~~ — closed 2026-09-28: ordinal 52 is `ngiGetClocks` (`Ngi32.dll:0x100045b0`), the low dword of `rdtsc`, or of `QueryPerformanceCounter` without feature bit `0x20000`; `0x1000dc20` is a static initialiser run as the module loads, so **no two runs share a seed**. **"65 other callers" undercounted**: there are 66 direct calls, two of them inside the randomised timer that 49 places call, so **111 other sites in 41 functions** draw on the stream — 15 on timers every takt runs, 61 in tasks, 7 in the walker, 28 unnamed. A building's takt timer draws every 64–127 ms, so how many draws fall between two steps follows frame timing, and no run of the game repeats the sequence either. The engine's fixed seed and single stream stay as a fair model, the stand-in now a note ([23-economy](docs/23-economy.md#what-else-draws-on-the-steps-stream--read-and-counted)).
- [x] ~~With the site test's two steps, Mission 04 has **no** Small Research Center site: 0 of 7,225 20 m grid points at any of eight turns has all three exits, 80 m out, on walkable areals and a level basement besides. The mission captures its centre rather than builds one, so no play depends on it; whether the game refuses as widely, or the posing or areal test is off on that map, is not established ([32-builder](docs/32-builder.md#the-test-isplacementvalid--read)).~~ — closed 2026-09-29, **the ninth round's "no site" corrected: there are twelve**, and neither the posing nor the areal test is off. The three exits stand on the root node at (0, −80) and (±77.8, 54.7), 8–16 m past the `.bas` outer ring as the storage's and the mine's sit past theirs, but they make a triangle 155.6 m a side on a map only 9.7% walkable (78 of 737 areals). The 20 m grid at eight turns was too coarse: a 10 m grid at 32 turns finds 1 of 924,800, a 5 m grid at 64 turns **12 of 7,398,400**, in two patches about (670–705, 800–815) and (1025–1050, 1285–1330); the Enhanced Research Center's own site passes once it is taken away. Controls: the same posing puts 1,109 of the 1,137 exits of the 166 placed hall-way buildings on walkable areals, the Mission 04 centre's 3 of 3; on Mission 04 the test finds sites for the Light Tower (837 of 57,800), the Outpost (392), the Small Bunker (257) and the storage (12); and Small Research Center sites on Mission 03 (1,439 of 80,000) and Mission 02 (22 of 57,800). The engine was right. Whether the game shows the same few is not seen: Mission 04 gives the player no builder ([32-builder](docs/32-builder.md#the-test-isplacementvalid--read)).

**Left by the tenth round** (2026-09-28)

- [x] ~~[M12] The line under the commander's map, naming what the cursor points at on it: the clan sprite table it draws from (the game's `+0x2c`, then `+0x20`) and the relations `0x10039440` and `0x10039460` that colour it. Not drawn ([35-hud](docs/35-hud.md#the-commanders-variant--read-and-seen)).~~ — closed 2026-09-29: the table is the screens object's eight clan signs, the unit markers' own 32 × 32 cells of `icons` along y 224, taken at the owner's clan record `+0x14` × 0x8c (`0x10073a80`). The two relations ask the player's clan's SuperAI (slot 8) for its word toward the object's clan and answer whether it is 0 and whether it is 1, so the sign is red `0xffff0000` for a hostile, grey `0xff808080` for a neutral and `0xff8080ff` for the rest, the player's own clan holding 2 toward itself. An icon piece (`0x1009a7a0`) is 19 wide with its sprite 2 in: a building gets sign, icon and an empty 19, a unit sign and two icons, nothing a 57 box; all put the emitter at x 436 and a 183-wide bar from 446. *Seen* at 421 s: a red sign at 382.7–394.7, two pale red icons, *"SFW-2 Warrior [patrolling]"*, the box ending at 435.3 and the green at 446.7. The engine drew nothing; it draws the line now ([35-hud](docs/35-hud.md#the-commanders-variant--read-and-seen)).
- [x] ~~[M12] Telepresence's last part: which units the bind's own let-go reaches, by their object id's third byte against the level's `+0xad4`; and a hit on a bot driven at level 0, which the game gates at once, leaving the attack on the stack for the letting-go, and the engine at the behaviour's first takt after it ([40-command-mode](docs/40-command-mode.md#what-the-ai-does-at-levels-1-and-2--read)).~~ — closed 2026-09-29: an object id's third byte is the player number `World3D.dll` files the object under (built at `0x100055f0`, read back by `GetIGObject`, `0x10007860`); the game stores `GetNetPlayerNum`'s word in both `+0xad4` and `+0xad0`, the player's clan (`0x1005cbd4`, `0x1005cbe1`), and `CreateObjectFromScheme` files units under that same number (`ArealMap.dll:0x10015dc9`). In all 22 single-play missions the hero is clan 0, so **the let-go reaches every unit**: all 249 placed units — 55 of the player's clan, the 22 heroes among them, 144 enemy, 28 nature and 22 neutral — and whatever a factory makes; only a network game's mirrors come under another number. The engine's default already matched. The level-0 hit waited for the first takt after the letting-go; it is now gated as it lands (`Behaviour::hurt_now`), the attack waiting on the stack, and the stand-in is gone ([40-command-mode](docs/40-command-mode.md#which-units-the-bind-lets-go--read-and-measured)).
- [x] ~~[M12] Which robots answer query 2 (slot 4, `edx` 2, the one that names a *Tiny Tower*) with 0 or less: a walking warrior that does gets the towers' building cell for its first panel icon and a blank second (`0x10077342`–`0x100773d1`), and the engine draws the towers' cell alone ([41-commander](docs/41-commander.md#the-box)).~~ — closed 2026-09-29: the record's `+0x64` is the unit's `IDeviceManager`, and query 2 its id 2, **the batteries' capacity** (`Control.dll:0x1002b4e9`): over the class-`0x13` components it answers the first capacity below 0 at once (`0x1002b519`), else the sum when that is above 0, else nothing. **17 of the 374 robot designs** under `UNITS` are *Tiny Tower*s, every one a walking warrior on the *Small Tower* chassis `R_B_06`, whose own unslotted battery is −1; the control: 355 answer above 0, and the 2 target dummies, with no battery, answer nothing — the recording names one *SSW-1 Warrior*. 28 of the 267 placed robots are, in 7 missions. A building row's icon stands in the same 19-wide piece (`0x10096339`), its buttons from x 85, not 86. Along the way: every unit the game names raises its clan's count — the hero, animals and *Tiny Tower*s too (`0x10075eb2`) — and the engine skipped the hero, naming Mission 03's builder SWB-1 where the recording shows SWB-2. The engine was wrong on the icons, the piece and the count; all fixed, units counting in the clan they were placed in ([41-commander](docs/41-commander.md#the-box)). A *Tiny Tower*'s −1 battery is a line [below](#mission-03-the-field-base).
- [x] ~~What the seven other `AniMesh.dll` routines that test node flag 1 leave a hidden building out of, and so whether a building going up still stops a ray or is struck; the engine keeps it out of everything until code 0 ([32-builder](docs/32-builder.md#not-established)).~~ — closed 2026-09-29: the sweep finds 9 sites (the draw's two the control), and of the seven others six read node records — the subtree draw (`0x100101d0`), the segment query's node visitor (`0x10010dc0`, `IJointMesh` slot 10 under `IMesh2` slot 6), a point-inside test (`0x100106d0`), the two walk-face queries (`0x1000ce90`, `0x10015b60`) and the push-out (`0x1000dfe0`) — each passing a hidden node over; the seventh (`0x1001db51`) is a C runtime routine's argument. The sphere (`IMesh2` slot 9) reads no node. So **a building going up stops no cursor ray, sight ray or round, is not stood on and pushes no walker**, but the object pick still takes it by its sphere, answering kind 2, and its own kill never does (mask `0x414` has no class 3). Behavior's hall-way searches pass it over by the same flag: the refit's dock pick, a factory's creation vertex, the transport's mine and storage. **The engine was already right** — the queue's "out of everything" was loose, its pick answering kind 2 — and a test pins it ([26-damage](docs/26-damage.md#what-a-hidden-node-is-left-out-of--read)). Two remainders are a line [below](#mission-03-the-field-base).
- [x] ~~The Guard row's place is the first point on the pending unit's list, and nothing empties the list as the pick opens, so a unit still holding another pick's points would patrol about the first of them (*derived*). The engine keeps points only on the pending pick, as for Route ([42-selection](docs/42-selection.md#the-guard-rows-pick--read)).~~ — closed 2026-09-29, **the tenth round's derivation overturned**: every order row with a target passes through the executor's tail `0x1007bab8`, which **empties the list** (`0x1007bac4`–`0x1007bad0`) before it sets the pending byte (`0x1007bad5`). The list is an (x, y) vector at `+0xc0`–`+0xc8`, appended by kind 8, kind 12 and Route's first stage, and erased by that tail, the Go (`0x100792f6`), the upgrade (`0x10079106`) and the pending routines after their orders, so the Guard's place is always the one clicked, as the engine had it. One edge the engine had wrong: a band started on a building leaves the Guard pick open with no pick mode and can leave the pending unit out of the selection, and the game then drops the pick with no order (`0x1007a17a`) where the engine ordered it; fixed and pinned ([42-selection](docs/42-selection.md#the-guard-rows-pick--read)).

**Left by the eleventh round**, closed 2026-10-01

- [x] ~~What the quick save and the quick load do~~ — closed 2026-10-01, **read**, **measured** and
  **seen**.
  - `CMD_QUICK_SAVE` and `CMD_QUICK_LOAD` are 753 and 754, the last two rows of `iron3d.dll`'s
    command table (`0x100726f8`). **The quick save** (`0x100a5030`) takes the save index's slot 6,
    the seventh, which the save page does not list: it names it string 6245, *"Quick Save"*, clears
    its empty flag, hands its filename to the one save writer (`0x100a1590`), and posts string 6246,
    *"Game saved..."*, from the system. It is refused in the training campaign, a network game and
    unless the state word is 4; a briefing passes no key that far.
  - **The quick load** (`0x100a51e0`) asks whether slot 6's file exists; if not, nothing happens; if
    so the game **exits with code 4**, which nothing else hands the exit, and `iron_3d.exe` runs the
    same game object again with no shell between. It does not ask the state word, so it works on the
    outcome's panel.
  - **The failed panel's L** opens the shell's load-game screen, which lists all seven slots; it
    loads nothing itself.
  - *Measured*: `saveslots.cfg` holds seven slots; six are filled with files on disk, and the seventh
    reads `empty` with no `slot7.sav`, so the quick slot was never written on this install.
  - *Seen*: Part 6's load after the loss at 13:55.8 is ten frames of *"Exiting..."* and the loading
    screen, no shell — F8, where the first pass had it as L. Part 6 26:14.5 and Part 6.5 13:39.0 are
    quick loads too, and Part 6.5 has two more, at 7:22.85 and 7:50.07.
  - **A save does not pause**, and **the clan's takt is on the wall clock** (`ai.dll:0x100017ae`,
    `timeGetTime`); a load restores the clan's seconds clock. The 50 s by which Part 6.5's first raid
    warning trails Part 6's are the two loads, 52 s of play discarded.
  - The engine bound neither key. It now keeps the play itself on F7 and puts it back on F8, a
    stand-in in two rows ([14-controls](docs/14-controls.md#quick-save-and-quick-load--read-and-seen),
    [17-saves](docs/17-saves.md)).
- [x] ~~[M12] The game menu's save page and the shell that *Load game* and *Quit game* hand the
  mission to~~ — closed 2026-10-01, **read**. The page (`iron3d.dll:0x100669a0`, draw `0x10066d50`)
  is the title 5086, six slot widgets, *Save* and *Cancel*; typing appends letters and digits to the
  name already there up to 16, which is why the six installed names are "empty" and three to eight
  letters. The index object (`0x1008c300`) rewrites `saveslots.cfg` whole from its constructor,
  destructor and each setter. The exit codes are enumerated, seven callers of `0x10061a30`: 3, 1, 1,
  1, 2, 3 and 4. What is left is queued: a quick save's own bytes, what a save keeps of the
  interface, and what `Run` does when a slot's file is gone
  ([39-boarding](docs/39-boarding.md#the-game-menu--read), [17-saves](docs/17-saves.md)).

## Mission 04, *Teleport*

Added on 2026-09-15 against `f26ffd8`, after M13 made the mission winnable end to end along the
briefing's route. [40-command-mode](docs/40-command-mode.md#not-established),
[16-research](docs/16-research.md#not-established), [27-ownership](docs/27-ownership.md#not-established),
[31-packages](docs/31-packages.md#not-established), [34-progression](docs/34-progression.md#not-established),
[24-motion](docs/24-motion.md#not-established), [30-turrets](docs/30-turrets.md#not-established),
[35-hud](docs/35-hud.md), [41-commander](docs/41-commander.md#not-established)

**Where the engine is weaker than the game**

- [x] ~~[M13] Which of Mission 04's hero, helicopter and HQ holds no mind. The recording's factory shows one free of three once all three are the player's, and Mission 02's shows the hero holding one; the engine lets the HQ taken by Enter hold none, or the build could never start. Mission 03's "4 free, five built" may be the same question.~~ — closed 2026-09-21, from Mission 02's round: **the HQ**. Every unit a mission places takes a free mind as it is made (`ArealMap.dll:0x100152ba`), and the `Capture` that Enter runs on a unit writes none (`Behavior.dll:0x10009051`), so of the three the HQ taken by Enter holds none and 3 minds leave the recording's one free. The engine was right; its stand-in comment and M13 row are gone ([23-economy](docs/23-economy.md#the-bot-limit-is-the-clans-mind-count--read-and-measured)).
- [x] ~~Mission 02's leaving places moved with the joined sphere~~ — closed in M14: the ground contact holds a machine about its node sphere's centre with the joined sphere's radius (read), so a landed L-2f stands 9.67 over flat ground and the whole island lets the hero out. The recording's 11 on landing and the hero's 3 on the same ground fit it ([24-motion](docs/24-motion.md#finding-the-ground--read), [39-boarding](docs/39-boarding.md#against-the-recording--seen)). Open still: see [Mission 02](#mission-02-the-constructor).

**Capture and the maps**

- [x] ~~[M13] A contour vertex's areal flag word, which the engine has no areals for~~ — closed in M14: the engine reads the areal map, and a vertex counts on a walkable areal.
