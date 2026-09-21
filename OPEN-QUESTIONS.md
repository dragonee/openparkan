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
- [ ] What lies below the dome's rim, and which views carry mode 1. *Narrowed 2026-09-20* out of the
  line above: the sky's fourth draw is **read** — four pre-transformed screen vertices from the viewport
  rectangle (`0x1007a1a6`), flags `0xc`, depth test and write both off, and a material block (sky
  `+0x484`) whose only written fields in the whole module are the ambient alpha at `+0x20`, two zero
  texture fields and a format id, so the only colour it can carry is the **scene colour**. It is
  **skipped whenever the view's mode (slot 24, `0x10083010`) is 1** (`0x1007a325`), and which views
  carry which mode is not read; that the world's view is mode 1 is *inferred*, not established. What
  lies below the rim was again not reached ([10-sky](docs/10-sky.md#the-dome)).
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
- [ ] What the colour filter's slot 5 computes. *Narrowed 2026-09-20*: it is **not the shader**, as this
  page said. The object reaches the bodies from `CAtmosphere`'s `+0x16c` (`0x1006fde6`) and the item
  renderer from the shader component's **interface 4** (`0x10032a75`); its slot 7 returns a block whose
  `+8` bit 0 is the flag, tested at `0x10079786`, `0x1007d923` and `0x1002fe79`, and its slot 5 maps one
  `D3DCOLOR`. The `0xff00ff00` mask names it — the camera's **infrared** (`CMD_CAMERA_INFRARED`,
  `CIS_INFRARED_ON/OFF/INV`, `NightVisionOn`), which is also what the outer view's flag `0x20` turned out
  to be. What slot 5 computes is still unread: the object's vtable is installed outside `Terrain.dll` and
  its class was not found ([10-sky](docs/10-sky.md)).
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
- [ ] Who sets an object's material track (`ILifeSystem` slot 16), and who calls IAnimation slot 27.
- [ ] Whether any caller besides the round's hit test and the collision pass hands a face query a
  triangle mask carrying 2 or 16 — the round builds its filter inline, so enumerating the filter
  constructor's call sites is not a complete enumeration. (~~What reads object face flags 2 and 16~~ —
  narrowed 2026-09-18: a face flag can be read in only three places, all `AniMesh.dll`, whose visitor is
  not exported and has two callers; and the walk-face query reads no triangle flag at all, so the flagged
  floors are the push-out's and not the ground search's. What the two flags **are** is measured through:
  2 is the walkable surface, all 6166 in a level-0 slot and 6100 above the engine's own cos-80° threshold,
  a chosen subset; 16 is the broad face of a door leaf, all 384 vertical on 52 interior nodes
  ([07-objects](docs/07-objects.md)).)
- [x] ~~What IAnimation node mask bit `0x10` does~~ — closed 2026-09-18: it lays the node along the
  ground under it. Slot 8 mirrors the bit into the node record's byte `+0x113` — nothing tests the mask
  word against `0x10`, so the byte is the whole path — and the pose walk then turns the node's world
  matrix by the rotation slot 31 left on it and writes the translation back, so the node tilts where it
  stands. Set by the ground contact on a `CONTACT_PLACE` contact's carrier. ~~Set on twelve nodes in the
  game: the belts of the three tracked chassis.~~ **Corrected later the same day**: `CONTACT_PLACE` is
  worked out from the state's end pose as well as read from the file, so it reaches 2229 of the 2634
  contacts and every walker's feet, not twelve belts (see *Contact record flag `0x20`* below).
  Implemented ([28-chassis](docs/28-chassis.md), [24-motion](docs/24-motion.md)).

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
- [ ] [M4] When a stream emits its first particle, and where burst and stream particles go.
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
- [ ] How Direct3D Sound places a sound between the speakers. (~~[M5] How a sound falls off between
  its near and far distances~~ — this half was **stale**: the game takes the Direct3D Sound path
  rather than its own mixer, so the law is DirectSound's — whole within the near distance, then
  *min* ÷ (*min* + *R* × (*d* − *min*)) for the listener's rolloff *R* of 1, and no further past the
  far distance. Re-checked by the coordinator 2026-09-18: `services.dll:0x10011914` pushes flags
  `0x120` into `niCreate3DSound`, so bit 16 is clear and each buffer goes to DirectSound whole
  through `SetAllParameters`
  ([11-effects](docs/11-effects.md#how-a-sound-is-heard--read-and-measured)). What is left is only
  how DirectSound itself pans.)

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
- [ ] [M14] The walker's local path and its obstacle contours: how it goes round a tree's or a stone's hole, whether it widens it by the unit's size, how a walker in one walks out, and what it does with a goal in one; the sub-areals' shapes and whether the search measures one from its centre; whether every scenery object reaches the areal map, and the box of a mesh of several parts; how it drops the points a unit has passed (`MWalker::ClearMoverReachedPoint`); how a unit's place comes onto a building's map object and which vertex the search starts from; who calls `MHallWay` slot 11; a hall-way vertex's size gate (the unit's `+0x960`, the record's `+0x28`); the link flags `0x10000` and `0x20000`; how a walker goes to the point it finds off a non-walkable areal, and what it does when its search fails; and how a walk to a door gets past the building's own walls, which cut no areal (the engine: a door more than 20 over the ground under it is passed over, and a straight line into a wall goes round the building's ground contour) ([24-motion](docs/24-motion.md#not-established)).
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
  is the one bound the pass *is* read to use, and it is the smaller on 122 of the 148 unit models
  the campaign places. *Measured* over 18 AI captures on C02 M03, every small chassis against
  every enemy building: 15 taken before, 14 after, the Large Factory 5 s faster for a wheeled bot
  and 15 s for a walker, and one lost — below ([24-motion](docs/24-motion.md#holding-the-body-on-the-ground--read-and-measured)).
- [ ] **C02 M03's Small Bunker (`l_bunk1`) cannot be captured by anything on wheels, and since
  the contact bound changed not by a walker either.** Raised 2026-09-20. A flyer takes it in 18 s;
  a wheeled bot holds outside at the top of the ramp to its door in either configuration, and a
  walker took it in 20 s with the up-pass bound at r and stalls 34 m short of the pod with it at
  r2. The walker's stall is **not** the same mechanism — no lift over 0.5 and no segment stop at
  the spot, so it is the AI's own legs rather than the contact — and the wheeled one is: there the
  push-out keeps the ramp face the bot stands on, because the filter drops only the floors flagged
  2 and docs/07 measures **1802 faces** of the first variant's level-0 slots pointing within 10°
  of up and carrying no flag at all. Two questions in one building, and both want the whole
  inside-a-building pass looked at rather than another bound guessed
  ([24-motion](docs/24-motion.md#collision-between-objects--read),
  [07-objects](docs/07-objects.md#the-flags-word)).
- [ ] **What lets a mover past a face flagged `0x20`.** Raised 2026-09-20 from play on C02 M04,
  *The Last Bastion*. The collision's own two filters take a triangle mask of **4**
  (`Control.dll:0x1001dbad` for the segment, `0x1001dbce` for the push-out) where a round's takes
  `0x24`; nothing read drops `0x20`. The shipped data needs it dropped. *Measured*: all **9** bridge
  pairs the 29 missions place stand π apart with their decks abutting, so each half's join cap — six
  level-0 faces at the deck's far end, all facing the span — is square in the way; three of the four
  `fortif.rlb` bridge meshes flag that cap **4** and `fr_e_brige` flags it **`0x20`**, the bit its
  additive `B_A_BRIGE` material carries throughout. Taken as read the hero walks 183.8 m of the 185.5
  to the join and stops. The **batch word** is the obvious other candidate — the query excludes
  batches flagged 8, and the energy batches carry `0x100` and no 8 — so either a third filter or a
  flag set on the loaded batch does it. The engine passes `0x20` as a stand-in
  ([24-motion](docs/24-motion.md#the-cap-where-two-halves-meet--measured-and-a-stand-in)).
- [ ] **What stops a tower's gun mast at the top.** Raised 2026-09-21 from play on C02 M04,
  *The Last Bastion*. Both `fortif.rlb` tower controllers carry a **class-29** component whose
  channels raise the mast out of the ground — `fr_m_tower`'s node 13 by 10.4 m over mesh frames
  1 to 4, `fr_b_tower`'s node 12 by 19.4 m over 0 to 3 — and each sets its switch word at
  `+0x18` to **9**, open and *bouncing*. Bit 8 is read to hold the progress at the end and swap
  the low bits (`Control.dll:0x10020ae4`), so as the file stands a tower raises its mast over
  five seconds and stows it over the next five, for ever. *Measured*: those two are the only
  records of the install's 1066 whose word bounces; the other 28 that set the word at all set 0
  (class 25) or 33 (the hero's arms). Nothing found switches it off — the component factory
  files every class in the controller's timed list (`0x1002d70a`), `CBuilding` looks for classes
  12 and 13 and no other (`Terrain.dll:0x100583a2`), and neither record names a section-5 group
  at `+0x10` or `+0x14`, which is what an item runs on its first step and its first skip. In the
  game the mast comes up as the tower is built and stays up, so the engine starts a bouncing
  item as one that opens and stops. What is left to find is the switch: the likely places are
  the fire control that owns a tower's turret and whatever handles a building finishing
  construction ([28-chassis](docs/28-chassis.md#every-component-is-stepped-not-only-a-device--read-and-measured)).
- [ ] **Whether a building's collision mesh follows the nodes its own items turn.** Raised
  2026-09-21 alongside the above. Nine class-26 records turn nodes that never stop — the three
  mines' rotors, the Main Teleport's twenty-eight rings, `fr_e_brige`'s hub — and the engine
  poses them every tick but rebuilds the collision solid only for a door's channels, since
  rebuilding it for a hub turning for ever would rebuild it every tick on every one of them.
  Whether the game's own `IGeometry` follows an animated node, and at what cost, is not read
  ([28-chassis](docs/28-chassis.md#every-component-is-stepped-not-only-a-device--read-and-measured)).
- [ ] **What the eye the sprite draw is handed has been transformed by.** Raised 2026-09-21.
  The emitter loop's context carries the instance's own matrix at `+8` and a point at `+0x48`
  (`Effect.dll:0x10008050`, `0x100080aa`), and the draw takes that point for the eye
  (`0x100093fc`). The sprite's position channel is in the frame's space, so the eye must be
  too, and `[0x1001e0bc]` slot `0x68` — called just before with the matrix in `ecx` and the
  point in `edx` (`0x100080da`) — is where it would be put there. That slot lives in the maths
  interface, outside `Effect.dll`, and is not read; the engine assumes the transform
  ([11-effects](docs/11-effects.md#a-sprite-is-drawn-through-its-frame--read)).

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
- [ ] [M5] How the HUD draws the aim point, the guns and the player's target; what plays `TARGET_READY` and `TARGET_ZOOM`; and the unit record's `+0x94` and `+0x98`.
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
- [ ] Which objects answer world class 2, the one class the outer camera's line drops. *Narrowed
  2026-09-20* with three controls, all negative. A class is slot 11 of the object interface and the
  query's first word is anded with `[class*4 + 0x1009a5f0]`, sixteen dwords whose entry *k* is `1 << k`,
  so the camera's `0x41a` does drop class 2 from a round's `0x41e`. **Class 1 is the landscape**, named
  by `CLightning::Init`'s own panic. (a) Every `call [reg+0x2c]` in all 21 modules followed within 8
  instructions by a comparison with a constant < 16 gives **68 sites** naming 1, 3, 4, 5, 6, 7, 9, 10
  and 11 — **never 2**. (b) Of the **71** `return N < 16` stubs in the install, the **7** that sit at a
  vtable's slot 11 answer 1 (three times), 3, 5, 7 and 11; four stubs do return 2, none at a slot 11.
  (c) The game-message switch on the id-class nibble gives 1, 3, 4, 7 and 11 a case and drops 2 to the
  default — weaker, since it drops 10 too. A class computed at runtime would still escape all three
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
- [ ] What the design state's `+0x370` is — the string all three readers of a design's `Type` test
  before anything else, answering `0x20000000`, an animal, when it begins with lower-case `a`
  (`iron3d.dll:0x1004f318`, `0x100514fb`, `0x100544f9`, each `cmp byte ptr [ecx], 0x61`; the boundary
  confirmed by five start offsets converging on it). *Raised 2026-09-20* out of the formats agent's
  part-category work, **and it corrects a claim in [30-turrets](docs/30-turrets.md)**, which read that
  string as the chassis's name ("a chassis whose name starts with `a`"). It is not: *measured over all
  **29** trees, which carry an identical set of **27** chassis part ids, **all 27 upper case** —
  `A_L_01`..`A_L_05` being the five animal ones — so **0 of the 783** examined begin with lower-case
  `a`*, and the test could never fire on a chassis id. The likely candidate is the designer's own kind
  prefix, spelled `r_`, `fr_` and `a_` and handed to the layout at `0x1004dff0`, but **no store into
  `+0x370` was found in `iron3d.dll`**, so that is *inferred*, not read — and that missing store is the
  next place to look. It makes no difference to a robot design, where the base part is a chassis and the
  `Type` comes from the turret either way ([38-designs](docs/38-designs.md#not-established),
  [30-turrets](docs/30-turrets.md)).
- [ ] Where the unit constructor's page item names (`+0xc4`) come from.

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
- [ ] **What the engine below a SuperAI does with a problem's action record** (`+0x34`, function 27).
  Raised 2026-09-21 by M18. `varset.var` names the five *"what to do with expression"*, *"When
  building capture"*, *"When all units in group killed"* and *"When all units in group nothing to
  do"*, and nothing else ends a problem a handler left `ST_SOLVING`: `PBM_ROBOT_NEEDED_Start` files
  `ACTION_NOTHING_DOING` on the factory it ordered and its `_Continue` has no nodes, so without them a
  clan builds one warbot and never another. The engine retires a problem whose action has come true,
  tested at the head of each clan takt. What the game actually does with the record — retire, re-raise,
  or something else — and where it is tested, is the question
  ([15-behaviour](docs/15-behaviour.md#what-the-functions-do)).
- [ ] **What `fn8(ST_SOLVED)` leaves behind, and what the repeated `_Start` pass excludes.** Raised
  2026-09-21 by M18. The state setter is read only as far as releasing the problem's units, but a
  solved record that keeps its slot would be matched by the raise's duplicate test and block the next
  want for as long as it stood — which [23-economy](docs/23-economy.md) says does not happen. And the
  pass is read to run again whenever a handler leaves its problem neither solved nor solving, which
  `PBM_ROBOT_NEEDED_Start` does when the clan has no factory: as read, that is an endless loop. The
  engine retires a solved problem and does not offer a problem twice in one takt
  ([15-behaviour](docs/15-behaviour.md#the-planner-when-a-_start-runs-and-when-a-_continue--read-and-measured)).
- [ ] **A `PBM_PLACE_PROTECT` place that is not on walkable ground strands the clan's warbots.**
  Raised 2026-09-21 from play on C02 M03. `c2m3e`'s `Problems0` names three places; (265, 1540) has
  **0 of 400** points sampled within its 150 m radius usable, its centre included, so the patrol's 350
  tries all fail and it takes the last point tried ([24-motion](docs/24-motion.md#not-established)'s
  stand-in). The clan's nearest capturer was standing off the areal map at (1341, 997) by the time the
  timed `PBM_BUILDING_CAPTURE` fired; from there the route is one escape leg, walked straight into the
  gorge wall, and it stands at 0 m/s for the rest of the mission — while from where the mission places
  it there is a 31-leg route over the bridge to the player's factory. What the game does with a patrol
  place off its own areal map, and what its walker does once stranded, are both unread.

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
- [ ] Whether the hero reports its route from inside a boarded bot.
- [ ] What the behaviour does with the message 6 it sends itself.
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

- [ ] Saves:
  - [ ] most chunks' contents (the control chunk past `+32`, the wizard, behaviour, building and tree chunks)
  - [ ] the clan word before each mind list
  - [ ] the 24-byte records and the `1, id, id` triple
  - [ ] the AI state's layout
  - [ ] whether a mind list's ids are logical ids
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
- [ ] What the words behind `objects.dlb`'s `A` and `N` are, and why twelve clip-less guns carry `A4`
  and `A5`. (~~what the `A` and `N` size letters stand for, and the 37 exceptions~~ — partly answered
  2026-09-18: the **referents** are pinned. `N` is the five `ANM` animals, which the research tree marks
  role 7 and no others; `A` is 27 fortification fittings, 21 of them fitted only under an `fr_*` root, and
  every line in the file that says "fortification" belongs to one. The words are **not recoverable**: the
  classification vocabulary reaches no shipped binary, with the `objects.dlb` token itself as the control.
  Of the 37 exceptions, 25 fall out of a rule — a weapon's `A<n>` is the size of the **round it fires** —
  7 more fire `f`-lettered rounds the rule cannot grade, one is a mobile builder, and 12 stay unexplained
  ([19-descriptions](docs/19-descriptions.md)).)
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
- [ ] What the word after `data.tma`'s map path was for — it marks the free-play maps, but `Single.02`
  disagrees with every reading. (~~what reads a building's start flag back, and the word after the map
  path~~ — closed 2026-09-18, both negatively and with controls. **Nothing reads either.** `IMission`
  slot 11 returns the map word and no module calls it; the mission pointer is never stored outside the
  five frames that hold it. `IBuilding`'s start-flag getter has no caller either, and the field it reads —
  `CBuilding +0xb8`, the building's object state, 1 by construction and 2 when the flag is set — is only
  ever tested against zero, which a placed building never is. So the flag changes nothing
  ([04-missions](docs/04-missions.md)).)
- [x] ~~What `TRF1`'s directory flag does beyond the debug warning.~~ — this line was **stale**,
  caught by the coordinator's own check rather than spent on an agent: it is already **narrowed** where
  it stands in [16-research](docs/16-research.md) to marking a tree as carrying debugging information,
  whose one reader found is the warning `FULL_RESEARCH_TREE` silences, and **no shipped archive sets
  it** — near enough a settled negative that an agent's time went to the briefings instead.
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
