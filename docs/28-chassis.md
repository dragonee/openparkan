# Chassis

A robot is a chassis with a turret on it, internal parts inside it, and guns on
the turret ([07-objects.md](07-objects.md#unitsdat--unit-and-building-assemblies)).
This page is the chassis: the families, the slots each one offers, what they
weigh and cost, where the research tree puts them, and what a chassis
contributes to the unit it carries. Speed and load are in
[24-motion.md](24-motion.md), hit points, shields and armour in
[26-damage.md](26-damage.md), battery and engine draw in
[23-economy.md](23-economy.md).

**Every claim is tagged**, as in [23-economy.md](23-economy.md): *measured*
is re-derived by `openparkan verify`, *read* comes from the disassembly at the
address given, *guess* fits the evidence and is not established.

## The families — *measured*

A chassis is a `BTLU` record in `objects.rlb` (`r_t_*`, `r_l_*`, `r_m_*`,
`r_b_*`, `r_h_*`) whose mesh, damage table and controller live in `bases.rlb`.
Its **sixth slot names a behaviour profile** in `behpsp.res`, and the profile's
`ChassisType` is its locomotion:

| profile | `ChassisType` | chassis | also |
|---|---:|---|---|
| `chas_fly.var` | 1 | every flyer and the Tiny Helicopter | `CanFly`, `CanStrafe` |
| `chas_wlk.var` | 2 | walkers, the Tiny Spider, the Transformer, the Small Tower, the hero | `CanJump`, `CanStrafe`, `WalkChassis` |
| `chas_wel.var` | 3 | wheeled | — |
| `chas_trk.var` | 4 | tracked | — |
| `chas_worm.var` | 5 | none shipped | `Worm` |

All 24 robot chassis records carry one, and on the 23 that `objects.dlb` names
the profile agrees with the name (*measured*). The animals name `chas_wlk` or
`chas_fly` the same way.

The size letter is the third character, and it is also the size of the turret
the chassis carries: **318 of 318** turrets on shipped robots share their
chassis's letter (`t`, `l`, `m`, `b`, and `h` for the hero) — shuffled, 86 do.
Nothing in the load path enforces it; the data never breaks it. What keeps it
so is, by the look of it, the robot constructor screen, which offers parts by
name ([below](#what-the-label-is-not--read-as-a-search)).

## A chassis declares its slots — *measured*, and *read*

A chassis's controller carries **six labelled component records**, all on node
0, one per internal-part family, each on the class that family is
([13-control.md](13-control.md#the-component-record)):

| label family | class | the part that fills it |
|---|---:|---|
| `i_eng_<s>` | 5 engine | an engine |
| `i_pws_<s>` | 19 battery | a power store |
| `i_fsh_<s>` | 9 fight shield | a shield generator |
| `i_dsh_<s>` | 10 detection shield | a detection shield |
| `i_rps_<s>` | 15 repair | a repair unit |
| `i_arm_<s>` | 27 armour | armour |

The hero declares the last five (its engine is a built-in, unlabelled one of
power 0.1); the two hero targets `r_h_01` and `r_h_03` declare only an engine.

**The label is the slot.** On all 320 robots with a chassis root, **every one of
the 1,889 internal parts on the chassis starts with one of its chassis's labels,
and no label is filled twice** — `i_eng_b_03` into `i_eng_b`. Given another
size's chassis instead, 1,878 would not fit. So the label's size letter says
which size of part a slot takes:

| chassis | engine | battery | shield | det. shield | repair | armour |
|---|---|---|---|---|---|---|
| tiny `r_t_*` | l | l | **t** | l | l | **t** |
| small `r_l_*` | l | l | l | l | l | l |
| `r_l_07` (S-7f) | **b** | b | b | b | b | b |
| medium `r_m_*` | m | m | m | m | m | m |
| large `r_b_*` | b | b | b | b | b | b |
| hero `r_h_02` | — | l | l | l | l | l |

Radars and deflectors do not go on a chassis: they fit on the turret, which
declares their slots the same way ([30-turrets.md](30-turrets.md)).

### What the label is not — *read*, as a search

The component parser keeps a copy of the label on the component object
(`+0x30`, `Control.dll:0x10021f00`), but nothing found compares it:
`Control.dll`'s one string compare (`0x1003a670`) is called only on
`(archive, member)` pairs in its resource caches, and a loaded part is
appended, not matched to a slot (next section). Neither `AniMesh.dll` nor
`Behavior.dll`, which send the parts, looks at a name's letters.

**The robot constructor offers parts by name prefix** — *read*, in outline.
Its page builder (`iron3d.dll:0x10048220`, called from `0x1004cd00` with a
page item's name at `+0xc4`) works like this:

- **A turret page.** A name starting `e_tur_` lists the catalogue entries
  starting with the name it was given (`0x10049126` into `0x1008a780`, which
  walks the item list and keeps the matches).
- **The chassis page.** It offers the prefixes `r_t`, then `r_l`, `r_m` and
  `r_b` as a four-way grade rises (`0x10048b38`).
- **Guns.** `e_gun_` is built the same way (`0x10048311`).

The screen those pages fill, with its two panels of six tabs, the project in
the middle and its buttons, is [37-designer.md](37-designer.md).

So the size letter and the slot label are kept by what the editor offers,
not by what the engine accepts. ~~Where each page item's name comes from is not
traced.~~ Answered in [38-designs.md](38-designs.md): the turret and gun pages
are keyed on the sockets' mesh labels (`e_tur_bb`, `universal_bl`), the
internal-part pages on the slot labels, and a chosen chassis or turret fills
each slot with its `<label>_df` part.

**The slot records carry defaults** — *measured*. Every chassis's engine slot
is value 1 at power 20, its battery 10,000 at 250 a second, its repair slot 1
point a second at 1 a point, its armour `(0, 1, 0)` (no reduction), its
detection shield all zeros; its fight shield grows with size — tiny 100 a
sector recharging 10, small 200/20, medium 300/30, large 500/50, the hero
1,000/100, all at 2 a point.

A few chassis carry more than the six: the Transformer, the L-7f, L-8f and S-7f
a second store of 1,000,000 at 1,000 a second, the Small Tower a generator;
every wheeled and tracked chassis and most flyers unlabelled class-3
records naming wheel, track or rotor nodes. `r_l_06` (S-6f) is a whole unit in one controller —
turret, camera, radar, deflector, detection shield and two guns — and no
assembly uses it.

### The class-3 records turn the wheels — *read*, and *measured*

Class 3 is `CICLS_SIMPLE`. The factory builds it, like every class without a
case of its own, as the plain 0xa4-byte device (`Control.dll:0x10020800`,
vtable `0x1003c448`). Each step (slot 11, `0x10020900`) it moves the section-2
channels its entries name by a mix of the machine's motion:

- **What the flags word picks.** Its bytes 1 and 2 each pick a source
  (`0x10020d90` into `0x10020ea0`):

  | byte | source |
  |---:|---|
  | 2–4 | spin about x, y, z |
  | 5–7 | the same, negated |
  | 8–10 | the lean on x, y, z ÷ triple 6 ([24-motion.md](24-motion.md#the-hull-leans-and-rights-itself--read-and-measured)) |
  | 11–13 | velocity x, y, z ÷ the authored top speed |
  | 14 | the speed ÷ the top speed, as lengths |

- **How they combine.** The two are weighted by the floats at record `+0x24`
  and `+0x28` into the device's **rate**, which scales how far each step moves
  it ([below](#what-a-devices-value-turns--read-and-measured)). A byte below 2
  picks nothing: byte 1 then stands for 1 and byte 2 for 0, so a record with
  both at 0 advances at a constant rate.
- **Byte 0.** When it is set, byte 0's source is written straight into the
  value instead, and a byte 0 of 1 holds the value where it is.

*Measured*: **72 class-3 records**, 71 on 11 chassis and one on the medium
MTP building. Every one has zero power, and zero values but for eight of the
flyers' records, whose first value is 0.5.

| flags | gains | on | reads as |
|---|---|---|---|
| `0x01070C00` / `0x02040C00` | 1 and 0.5 | the drive wheels and tracks of all six wheeled and tracked chassis, left and right | forward speed ∓ half the turn: skid steering |
| `0x00000004` | — | two wheels of the Small Wheel Chs and four of the Medium | set to the turn rate: steering |
| `0x00000001` | — | the other wheels, and the upper track rollers | held |
| `0x8000000C`, `0x8100000C`, `0x8200000C` | — | engines and wings of four flyers | a switch on forward speed ÷ top: they tilt |
| `0` | 1 | the T-2's two rotors, and the building's | a constant: they spin all the time |

So the unlabelled records are animation, not slots.

### What a device's value turns — *read*, and *measured*

**The device is an item, stepped.** Class 3 is built as the base item, the
same object a door, a control pod or the hero's arm is
([29-weapons.md](29-weapons.md#the-button-reaches-the-selected-guns),
[27-ownership.md](27-ownership.md#capture--read)), and the radar's constructor
(`0x10024310`) calls it too and keeps its update: the radar's vtable
`0x1003c800` differs from the item's `0x1003c448` only in slot 0. The update
does not run every tick. Each machine tick (`0x1000bcf0`) hands the game
time to the **time driver** (`0x1002d260`, called at `0x1000c745`), and the
driver, for each device whose last step has ended, plays its channels to that
time, starts the next step at the old step's end, and runs the **update**
(slot 11, `0x10020900`). Several steps run in one tick if the tick is long.

**The update** keeps a progress (`+0x94`, 0 at construction) and a switch word
(`+0x50`):

- **When it does nothing.** The word is 0, or the device's node is destroyed
  (slot 2, `0x10021820`, 1 at a life of 0). The first step it runs, and the
  first it skips after running, each run a section-5 group from the record's
  `+0x10` and `+0x14` (`0x1002095a`, `0x10020931`, `0x100221e0`); every one
  of the 72 class-3 and 76 radar records has −1 in both. A rate below 1e-9
  (`0x1003b380`) also does nothing.
- **The word.** The constructor sets it to **5** (`0x10020832`) and the parser
  replaces it with the record's `+0x18` unless that is −1 (`0x10021d86`).
  *Measured*: all 72 class-3 records and all 76 radar records hold −1, so
  every one runs in 5. Across the install's 1066 components only 30 set the
  word at all: the 24 class-25 records set 0, the hero turret's four class-24
  arms set 33, and **the two towers' class-29 masts set 9**, open and
  bouncing ([below](#every-component-is-stepped-not-only-a-device--read-and-measured)).
- **Byte 0 clear.** The progress moves by **0.45 × rate** (`0x1003c488`) while
  the word's low bits are 1, back by it while they are 2 (`0x10020a28`). Then
  the word's bits `0xc` pick what an end does: **4 wraps** the progress once
  into 0–1; **8 bounces**, holding it at the end and swapping the low bits; any
  other value holds it and clears the word, which is how an arm or a door
  stops. So a word of 5 is open and wrapping: the progress goes round for
  ever.
- **Byte 0 set.** The progress is the source byte 0 picks, outright
  (`0x100209ea`).
- **What each channel is sent** (`0x10020bc1`), by byte 0:
  - **0** — the progress;
  - **2 and up** — the progress ÷ the channel's span, plus its initial value;
    or, with the flags word's top bit set, **a switch: 0 below the channel's
    initial value and 1 from it** (`0x10020bf0`);
  - **1** — the channel's initial value plus a float found in the machine's
    list at `+0xc4` by the channel's `+8` (`0x10020c25`), over its span; that
    list is not read.

  The value is held to 0–1 (the link's wrap byte, `+0x10`, is left 0 by the
  parser, `0x10021e49`), and the channel runs from where it is now to it.
- **How long the step lasts** (`0x10022120`): the longest over the channels of
  1000 × |to − from| ÷ (|rate| × the channel's rate) ms, the short way round
  on a channel flagged 1 (a gap over a half counts as one minus the gap); **100 ms**
  if that is under a millisecond (`0x10020d72`). The driver keeps
  1 ÷ the length (`0x100221b0`).

**Playing the step** (`0x10021a30`): at time t the phase is s = (t − start) ÷
length, or 1 once t reaches the end; each channel's value is from + s × (to −
from), the short way round and wrapped once into 0–1 on a wrapping channel,
held to 0–1 otherwise. It is then offered to the node as every channel's is: a
wrap or a hold by the channel's flag 1, 1 − v by flag 2, and the node plays
first + v × (last − first) on its own segment
([07-objects.md](07-objects.md#how-the-engine-plays-it--read),
[30-turrets.md](30-turrets.md#aiming-and-the-camera--read-and-measured)). The
item's slot 12, which a turret uses for its strafe offset, is 0.0 here
(`0x10021810`).

**So the speed is the rate times the channel's rate** (*derived*). With byte 0
clear, a step of 0.45 × rate lasts 450 ÷ the channel's rate ms, whatever the
rate: the value
moves at |rate| × the channel's rate a second, linearly within each step. A
wrapping value a step moves by more than a half would play backwards, since a
step goes the short way round; that needs a rate above 1.11, which no constant
device has.

### Every component is stepped, not only a device — *read*, and *measured*

The class a component carries decides what it *is*, not whether it steps. The
component factory (`Control.dll:0x1002d4b0`) switches on the class through a byte
table at `0x1002d864` into fourteen cases, and **every case ends at `0x1002d70a`,
which appends the component to the controller's timed list** at `+0x5bc`. The time
driver walks that list whole. Four of the fourteen cases build the plain base item
and keep its update: class 3 and class 12, 13, 28, 29 and the rest through the
default case (`0x1002d6ec`), class 8 the radar (`0x1002d5b8`), class 5 the engine
(`0x1002d6ae`) and class 26 (`0x1002d63f`); their four vtables — `0x1003c448`,
`0x1003c800`, `0x1003cdf8` and `0x1003ce38` — are the only ones in the module that
carry `0x10020900` in slot 11.

So a class its owner never looks at still turns what it names. *Measured*: twelve
classes name section-2 channels across the install, and two of them are steered by
nobody — **class 26**, a building's efficiency, on nine records, and **class 29** on
two. Class 26's nine are the three mines' rotors, the Main Teleport's twenty-eight
rings and `fr_e_brige`'s hub; class 29's two are the Small and Large Towers' gun
masts. `CBuilding` files classes 12 and 13 and no other
(`Terrain.dll:0x100583a2`), and a robot's own device list is classes 3 and 8, so a
reader who files components the way their owners do steps neither.

**When the picture moves between steps** — *read*, and not established.
Channels are played at the start of each step (`0x1002d29e`), where the phase
is 1 and the value is the finished step's end. They are also played every
machine tick, through `0x1002dc50` at `0x1000c275`, but only while one of two
countdowns on the control system, `+0xd0` and `+0xd4`, is above zero; each
tick takes one off (`0x1000c843`, `0x1000c854`). `IControl` slot 8
(`0x10004710`) sets `+0xd4` to 100 for an argument of 0 and `+0xd0` for 1, and
the driver plays a device again after a step while `+0xd4` is set
(`0x1002d2f4`). Which caller sets them was not found: a search of every
module for a slot-8 call shaped `push 0 or 1; push object` turned up none that
could be shown to be on an `IControl`. Slot 8's place in `Terrain.dll`'s stub
table would make it `SetControlCalculationMode` (a *guess*, counting from
slot 5 = `SetTangSpeed`, `0x100044a0`). Without a countdown a T-2's rotor would
be drawn only at each step's end, 162° apart; with one, it turns smoothly.

**What turns, and how fast** — *measured*, then *derived* from the read
update:

| device | record | channel: node, frames | channel rate, span, flags | turns |
|---|---|---|---|---|
| T-2 rotor, top | `r_t_02` class 3, flags 0, gains 1 and 1 | `Tup_m1o1`, 2–6, −90° about z a frame | 7.3, 2π, wrap | **7.3 turns a second** |
| T-2 rotor, bottom | the same | `Tdn_m1o1`, 2–6, +90° a frame | 7.3, 2π, wrap | **7.3 turns a second**, the other way |
| MTP building | `fr_m_mtp` class 3, flags 0 | node 11, 0–4, −90° a frame | 0.4, 2π, wrap | 0.4 turns a second |
| a robot turret's radar dish | the turret's class-8 slot, flags 0, gains 1 and 1 | `TTrad`, `TMrad`, `LTrad`, `BTrad`, …, four frames of −90° or +90° | 0.5, 2π, wrap | **0.5 turns a second** ([25-sensors.md](25-sensors.md#the-dish-turns-while-the-radar-runs--read-and-measured)) |
| M-2f engines, left and right | `r_m_02` class 3, `0x8100000C` / `0x8200000C` | `engnL`, `engnR`, 1–2: x out by 0.3 at 1, in by 0.2 at 2 | 0.2, span 1, initial 0.5, invert | a switch at half the top forward speed, 5 s to swing |
| M-2f tail engine | `0x8000000C` | `engnT`, 0–0 | 0.2, initial 0 | nothing to play |
| M-2f wings | `0x8000000C`, four channels | `LTwng`, `RTwng` 30° and `LBwng`, `RBwng` 50° about y at frame 1, flat at 2 | 1, span 1, initial 0.5, invert | a switch at half the top forward speed, 1 s to swing |

Byte 0 of 12 is the velocity's y (`+0x1c8`, the machine's own frame) ÷ the
frame's third triple's y, the authored top forward speed (`0x1002104c`). With
the invert flag, a switch at 0 plays the last frame and at 1 the first. So an
M-2f hovering or slower than half its top speed holds its wings flat and its
side engines in; from half its top speed they sweep and swing out over one and
five seconds (*derived*). A switch re-reads the speed every 100 ms while it
holds, and not until a swing has finished once it moves.

`openparkan.control.Item` is this update, driver and playback, and `openparkan
verify` steps it over the shipped records.

### The belt is a material a channel plays — *measured*, and *read*

A wheeled chassis turns its wheels: its devices' channels span frames and the
node plays them. A **tracked** chassis has nothing to turn — a belt is one
piece — and its four skid-steering channels say so. They are the only channels
in the game that carry **flag `0x10`**, and they are the only ones a device
drives that span no frames.

*Measured*, over every `.ctl` in the install — 991 channels:

- **12 carry `0x10`**, and they are the four belt channels of each of the three
  tracked chassis, `r_l_04`, `r_m_04` and `r_b_04`, and nothing else.
- Every one has **`first` = `last` = −1**: no segment to play, so no node moves
  with it. Every other channel a device drives has frames.
- Their flags are `0x15`: wrapping, not driven by the component update, and
  `0x10`. Their span is 1 and their initial value 0.
- Their devices are the **skid-steering** records of the table above,
  `0x01070C00` and `0x02040C00` at gains 1 and 0.5: forward speed ∓ half the
  turn.
- **The nodes they name are exactly the nodes that wear a played material** —
  `TFL/TFR/TBL/TBR` on the S-42t, `TMFL/TMFR/TMDL/TMDR` on the M-42t,
  `BRLFD/BRRFD/BRLBD/BRRBD` on the L-42t. Those wear `R_RL_25`, `R_RL_26` and
  `R_RL_27` and their `REV` twins: four-entry tracks ending at 50, 100, 150 and
  200 ms, which step four tread images (or four cells of `NP03`), the `REV`
  twins the other way round so the far side of the hull scrolls to match. The
  upper rollers, which are held at `0x00000001`, wear `R_NP03`, which has one
  key and does not play.

So the value has one thing left to drive: the belt's material. The manager has
a fetch for that — **slot 5** (`World3D.dll:0x10003680`,
[07-objects.md](07-objects.md#how-a-material-reaches-the-device--read-and-measured)),
which takes a **fraction** rather than the world clock and shows the key at
last-key-time × it. A wrapping 0..1 value is exactly what that fetch wants.

**What it comes to** (*derived* from the read update): the value moves at
|rate| × the channel's rate a second, so the belt goes round once per

| chassis | channel rate | top speed | metres a loop |
|---|---:|---:|---:|
| S-42t (`r_l_04`) | 100.0 | 29.17 m/s | **0.29** |
| M-42t (`r_m_04`) | 51.4 | 26.39 m/s | **0.51** |
| L-42t (`r_b_04`) | 19.23 | 25.0 m/s | **1.30** |

— a track-link pitch that grows with the chassis, which is the check on the
reading: the rates are not arbitrary, they are that chassis's belt measured
against its own top speed. And because the rate is the skid-steering one, the
belt **holds where it stopped while the bot stands** (a rate under 1e-9 moves
the progress not at all), runs backwards in reverse, and the two sides run
opposite ways when the bot turns on the spot.

The engine plays it this way: `CHANNEL_MATERIAL` in `parkan-formats`,
`Robot::material_phase` for the value and `Animation::by_fraction` for slot 5,
handed to the node's model as it is placed.

### The belt lies along the ground — *read*, and *measured*

The other thing only a tracked chassis does is a contact flag. A contact's
`0x2` ([13-control.md](13-control.md#section-1s-conditions-are-contacts--read-and-measured))
lays the node the contact **carries** — the control point's third slot,
[24-motion.md](24-motion.md#a-contact-point-sits-on-one-node-and-dies-with-another--measured) —
along the ground beneath that contact, and leaves it where it stood.

**How it reaches the picture** (*read*), in three steps:

1. The ground contact, for every contact whose flags carry `0x2`, marks the
   carrier node with **`IAnimation` node mask `0x10`** — slot 8
   (`AniMesh.dll:0x10005500`), called at `Control.dll:0x1001a3af` with the
   carrier the pass has just fetched (`0x1001a3aa`). Slot 8 sets the bit in the
   node record's flags word and mirrors three of that word's bits into bytes of
   the same record, `0x4`, `0x8` and `0x10` into `+0x111`, `+0x112` and `+0x113`;
   a change to the last two clears the node's pose cache (`0x1000a2a0`).
   **Nothing tests the flags word against `0x10` anywhere** — a scan of every
   test of that word against an immediate finds 1, 2, 4, 8, `0xc` and
   `0x1000000` and no `0x10` — so the byte is the whole consumption path.
   Nothing clears it either. (Enumerating every four-argument call of slot 8
   across all sixteen modules returns eleven, all in `Control.dll`; three of
   them are the damage flags [26-damage.md](26-damage.md) already reads, which
   is the check on the scan. `0x8` is both set and cleared; `0x10` is only set.)
2. The same pass, for a contact whose `0x2` is set and whose node still stands,
   hands **`IAnimation` slot 31** (`0x10005c90`, from `0x1001affd`) the
   contact's own axis and the ground normal it has just found under that
   contact (`0x1001bfc0`). The slot shifts the node's current turn `+0xf0` into
   its previous one `+0xe0`, writes identity in its place, and — given two
   vectors — builds the turn that takes the first onto the second by the
   half-angle trick: from the dot and the cross of the pair it makes twice that
   turn, and slerps identity halfway toward it. **Handed a null vector it
   leaves identity** (`0x1001aff5`, `0x1001affa`), so a node whose contact stops
   being placed relaxes back to level over the next blend.
3. The **pose walk** reads `+0x113` in three places — `0x100090a5`,
   `0x100091a6` and `0x100092b6`, the root's pass, a parented node's and a third
   (`0x10008b70` / `0x10009018`) — each just after the node's world matrix has
   been composed from its parent's and its own. Where the byte is set the walk
   slerps the node's previous and current turns by the mesh's pose-blend weight
   `+0x1f4`, the weight it uses everywhere else; saves the world matrix's fourth
   column; turns the slerped quaternion into a matrix and multiplies it onto the
   world matrix; and writes the column back. **So the node tilts where it
   stands** and the hull above it does not move (*derived*, from the save and
   the restore).

**Who asks for it** (*measured*), over every `.ctl` in the install — 2634
contacts on the 961 states of the 99 controllers that declare any:

- **12 contacts carry `0x2`**, four each on `r_l_04`, `r_m_04` and `r_b_04` —
  the S-42t, the M-42t and the L-42t — and nowhere else. Their flags are `0x7`:
  support, place and fallback together.
- Their points are `weel_fl`, `weel_fr`, `weel_bl` and `weel_br`, **every one
  placed on node 0**, the hull, and each **carried by a node of its own**. The
  twelve carriers are exactly the belt nodes: `TFL/TFR/TBL/TBR` on the S-42t,
  `TMFL/TMFR/TMDL/TMDR` on the M-42t, `BRLFD/BRRFD/BRLBD/BRRBD` on the L-42t.
  All twelve are leaves, so the tilt reaches no child node.
- Their axes come out along the model's up once posed: the point's vector is
  `(0, −1, 0)` on the S-42t, whose root turns −90° about x, and `(0, 0, 1)` on
  the other two, and all three land on world up.
- Every contact of the wheeled `_03` chassis carries `0x5` — support and
  fallback — and never `0x2`.

**So a tracked warbot's four belts each lie on the patch of ground under them
while its hull holds its own attitude.** Twelve nodes in the whole game carry
the flag as authored.

**A walker's feet reach the same code by another road.** Flag `0x20` asks for
`0x2` to be worked out from the state's own last pose, and 2410 contacts carry
it — every foot of every walking chassis and of the three animals — of which
2217 stand up and are placed
([24-motion.md](24-motion.md#a-walkers-feet-lie-flat-where-the-animation-lays-them--read-and-measured)).
So the twelve belts are what is *authored* as placed, not what is placed: 2229
contacts in the game lay their node along the ground. A tracked chassis is
still the only one that does it without asking the pose, which is the point —
a belt is flat in every state, so there is nothing to work out.

**Where the point stands when the turn is worked out** (*read*). The pass that
lays the node runs once a frame, on message `0x1c`, and it does not pose the
object: it asks the point's position and axis through the carrier's node matrix
as the mesh currently holds it, and the machine tick has just played the mesh
at that frame's own time (`Control.dll:0x1000c737`). So the axis a belt or a
foot is turned from is the **frame's interpolated pose**, not the state's last
frames
([24-motion.md](24-motion.md#holding-the-body-on-the-ground--read-and-measured)).
The state's end pose is used only once per state, to decide the flag.

**It is not the belt's material.** These three chassis also carry the
**channel** flag `0x10`,
[`CHANNEL_MATERIAL`](#the-belt-is-a-material-a-channel-plays--measured-and-read),
on channels naming the same four nodes. The two are unrelated: one is the
belt's texture phase, driven by the skid-steering value; the other is the belt
node's tilt, driven by the ground contact. They meet on these twelve nodes
because both are things only a tracked chassis needs.

The engine does it this way: `CONTACT_PLACE` in `parkan-formats`,
`place_by_pose` for the flag `0x20` decides, `Walker::lay_belts` for the turn —
kept per carrier node, in the machine's own frame — and `Robot::chassis_pose`
turning that node's world pose by it with its translation untouched. The
previous/current pair is left out: all twelve belt contacts belong to a
velocity-driven state, whose blend weight never leaves 1, so there the slerp is
the current turn. A walker's foot is placed in states that do blend, so its
tilt arrives a step sooner than the game's.

### What moves by itself on Mission 01 — *measured*

Over every unit and building `data.tma` places (the hero `tut1_p`, the enemy
`tut1_e1`, the neutral `helic` and `tut1_mf1`, three `l_targ`, two `M_targ` and
two `m_bridge`), the parts that move without a controller state are these
devices and nothing else:

| unit | chassis | turret | what moves |
|---|---|---|---|
| `helic`, neutral | T-2 (`r_t_02`): its two rotors, 7.3 turns a second each, opposite ways | `e_tur_tb_01`: its dish `TTrad_m1o1`, 0.5 turns a second | three |
| `tut1_mf1`, neutral | M-2f (`r_m_02`): side engines and four wings, switched at half its 125 km/h | `e_tur_mb_01`: its dish `TMrad_m1o1`, 0.5 turns a second | seven channels |
| `tut1_e1`, enemy | Tiny Spider (`r_t_01`): none | `e_tur_tt_01`: its dish `TTrad_m1o1`, 0.5 turns a second | one |
| `tut1_p`, the hero | `r_h_02`: none | `e_tur_ht_02`: a radar slot with no channel, so no dish | none |
| `l_targ`, `M_targ` | `r_h_01`, `r_h_03`: one state, one frame, no channels | — | none |
| `m_bridge` | a `FORT` with no controller; its parts are internal | — | none |

The rest of the map is still as well: the flyers' one state plays frames 0 and
1 with no node moving between them, and each tree and stone's controller has
one state whose frames move no node either. The turret's aim, the guns'
barrels and the hero's arms move too, but only when something drives them
([30-turrets.md](30-turrets.md#aiming-and-the-camera--read-and-measured),
[29-weapons.md](29-weapons.md#a-gun-is-ready-once-its-arm-is-out--read-and-measured)).
A walker's legs are its states ([24-motion.md](24-motion.md#playing-a-state--read-and-measured)).

## A fitted part takes over its slot — *read*, and *measured*

Assembling a unit does not give each part its own machine: every part goes into the
one control system, and **how depends on the part's `objects.rlb` tag**.

- **An external part (`EXTO`: a turret, a gun) is appended.** `AniMesh.dll` attaches
  the parts in list order (`0x100036cb`, 76-byte entries at `+0x6e4`); for an `EXTO`
  (`0x100038db`) it merges the part's mesh into the agent's model and sends the
  control system the load message `0x80000020` with the part's names, its id, the
  node its mesh starts at and a component index of −1 (`0x10003b6f`).
  `Control.dll`'s loader (`0x10008b10`) then builds every section-4 record into a new
  device, rebasing its node (`0x10009045`, `0x10009081`), and loads the part's node
  and `.ndp` tables.
- **An internal part or a clip (`INTO`) replaces the component it is fitted to.** Its
  mesh is not merged, and the message carries no node (−1) but a **component index**:
  the `.dat`'s attach field plus the first device index of the parent part
  (`0x100039d9`; interface `0x202`, slot 17, `0x1002ecc0`, finds a part's first
  device by the id each device keeps at `+8`). With an index, the loader skips the
  node and `.ndp` tables (`0x10008b75`) and hands the part's first component record
  to `0x1002d890`, which **re-parses that existing device through its own class's
  parser** (slot 7), keeping the device's node — and, for armour, refreshes the
  armour numbers.

So for an internal part the `.dat`'s "attachment node" is not a node: it is **the
index of the slot in its parent's controller**. *Measured*: all 3,828 internal parts
and clips in `UNITS` give the index of a parent component of their own class, whose
label, where it has one, prefixes their name (the animals' slots are unlabelled);
shifted one index on, 687 would. The 1,417 external parts attach to mesh nodes.

**What a unit ends up with** is therefore the fitted parts' figures, not the slots':

| slot | on the chassis or turret | on a shipped robot |
|---|---|---|
| engine | drive 1, draw 20 a second | the fitted engine: drive 0.7–1.0, draw 0.75–11.2 |
| battery | 10,000 at 250 a second | the fitted battery: 3,000–4,080 at 5–6.8 (small), 9,600–12,000 at 12–15 (medium), 22,000–31,000 at 25.5–34.5 (large) |
| shield generator | 100–1,000 a sector | the fitted generator's 350–3,800 |
| detection shield | all zero | the fitted shield's cuts and camouflage |
| repair | 1 HP a second at 1 a point | the fitted repair unit's 11–80 at 0.04–0.06 |
| armour | (0, 1, 0), no reduction | the fitted armour |
| radar (turret) | 0.5/0.5/0.5, range 500 or 800 | the fitted radar: 0.05/0.7/25, range 250–700 |
| deflector (turret) | 0.5 | the fitted deflector's 0.7–1.0 |
| a gun's magazine | the gun's value 0 | the fitted clip's rounds |

*Measured*: on the 374 robots every labelled slot is filled but three — the Small
Tower's armour (`tower_s.dat`) and the two target dummies' engines. What is *added*
rather than replaced is what has no slot: the hero's built-in engine, and the second
1,000,000 store on five chassis and the Small Tower's generator, which are unlabelled
records appended with the chassis.

The single pointers the factory keeps (turret, fight shield, deflector, radar,
class 17: `0x1002d56e`) point at the slot's device, which the fitted part re-parses
in place, so they need no second look.

### The order parts load in, and what a slot keeps — *read*, and *measured*

**Parts load in the assembly's own order, depth first** (*read*).

- **The chassis first.** `Behavior.dll` builds the unit from the root record
  ("Chassis created", `0x1001d8eb`). The agent enters that object as part 0
  (`AniMesh.dll:0x1000311f`).
- **Then each child.** Each child of the root goes through `0x1001cd40`, which:
  - sends the part (message `0x80000020`, `0x1001ce5b`);
  - reads back the id the agent gave it, the smallest not yet taken
    (`AniMesh.dll:0x10003775`);
  - recurses into the part's own children with that id as their parent.
- **So the order is the file's.** It is the `.dat` file's pre-order, the order
  `objects.load_unit` lists the components in. Each part loads after
  everything listed before it, so a turret's guns and clips load after the
  turret.
- **What a part brings.** An `EXTO` part's mesh is merged without its own node
  0, whose place is the node it attaches to: `AniMesh.dll` skips it
  (`0x1000a79d`), and `Control.dll` skips the first `.ndp` row to match
  (`0x10008c6a`). On all 111 external parts the mesh and its `.ndp` have the
  same number of nodes, so the two stay in step (*measured*).

**The last record of a single-pointer class wins, and no assembly has two.**

- **The rule.** The factory overwrites the class's pointer on every record it
  builds (`0x1002d53c`, `0x1002d56e`, `0x1002d5a0`, `0x1002d5d2`,
  `0x1002d604`). The armour numbers are refreshed from every class-27 record
  (`0x1002d7b6`). So the later part in load order would win.
- **The data.** It never comes to that (*measured*). Over all 458 assemblies,
  counting the root's records, every external part's, and the one each
  internal part re-parses, no assembly has two records of class 1, 8, 9, 17,
  21 or 27. The chassis brings the fight shield and armour; the turret brings
  itself, the radar and the deflector.

**A fitted part keeps nothing of the slot's figures** (*read*).

- **What the re-parse replaces.** It runs the class's parser with the node
  kept (`0x1002d8ad`). The shared parser (`0x10021d50`) then:
  - points the device at the part's record (`+0x48`), from which its mass,
    power and flags are read;
  - copies the part's sixteen values over the slot's (`0x10021d93`);
  - takes its initial state unless that is −1;
  - and renames it.
- **What survives.** Only the node and the slot's entries: entries are
  appended (`0x10021df3`), and a fitted part brings none.
- **The data.** All 3,832 internal parts and clips carry exactly one record,
  with no entries. The 958 slots that have entries keep them: 588 guns'
  magazine slots (their barrels), 353 radar slots and 17 deflector slots
  (*measured*).
- **So the defaults stand only in an empty slot.** The three are the Small
  Tower's armour, which weighs nothing and cuts nothing, and the two targets'
  engines of drive 1.

## What a chassis weighs — *read*, and *measured*

`Control.dll:0x1000fac0` weighs the merged model node by node. A node's mass is

    .ndp density  ×  volume  (+  armour rating × area,  when the unit has armour)

and each part adds the mass on its controller record
([24-motion.md](24-motion.md#load--read-and-measured)).

- **Volume and area are the node's level-0 geometry slot**, `+0x34` and `+0x30`
  of the 68-byte slot record in the current damage variant
  (`AniMesh.dll:0x100051f0`, `0x100124d0`), scaled by the object's scales.
  *Measured*: `+0x34` is exactly the slot's bounding-box volume on 705 of 705
  slots of the player chassis. A node with no level-0 slot weighs nothing.
- **Armour's first value is a weight, not a rating.** The factory stores it in
  the armour struct at `+0xc` (`0x1002d7fc`), and the mass sum multiplies it by
  every node's area (`0x1000fbac`). Heavier armour marks weigh more:
  0.125–30 kg per unit of area. [26-damage.md](26-damage.md) found it plays no
  part in a hit; this is what it does.

*Measured*: a player chassis's own body — density × level-0 volume over its
nodes — is **a round figure on 13 of 15**, although 13 of them carry fractional
densities. The densities were computed to hit a design weight:

| chassis | code | body kg | payload t |
|---|---|---:|---:|
| Tiny Spider | T-12w | 600 | 3.25 |
| Tiny Helicopter | T-2 | 275 | 2.775 |
| Small Walking | S-12w | 1,375 | 6 |
| Small Flying | S-2f | 1,125 | 4.25 |
| Small Wheel | S-31 | 1,875 | 6.5 |
| Small Track | S-42t | 2,050 | 7.5 |
| Small Flying | S-4f | 1,272 | 5.5 |
| Medium Walking | M-12 | 4,500 | 28 |
| Medium Flying | M-2f | 4,006 | 24 |
| Medium Wheel | M-32 | 5,000 | 32 |
| Medium Track | M-42t | 6,000 | 36 |
| Large Walking | L-12w | 12,500 | 65 |
| Large Flying | L-2f | 10,000 | 55 |
| Large Wheel | L-32 | 17,500 | 70 |
| Large Track | L-42t | 20,000 | 80 |

A whole unit's weight follows the same sum over the parts that bring nodes —
the chassis, turret and guns — plus each internal part's and clip's own mass: an
internal part's mesh is not loaded, so it adds no volume and no area — *derived*.
A turret's or gun's node 0 is not merged
([above](#the-order-parts-load-in-and-what-a-slot-keeps--read-and-measured)),
so it weighs nothing either.

**The chassis's body does not count against its payload** — *read*. The nodes
the payload sum returns are part 0's, the root object's
([24-motion.md](24-motion.md#load--read-and-measured)). That settles the
earlier *guess*. So:

- The tracked `w_b_trk1` comes to about 80.6 t: the chassis 20 t, the turret
  and guns 20.5 t, the internal parts and clips 21.6 t, and armour Mk3 at
  22.5 × 824 of area 18.5 t. That is against 80 t of payload plus the
  chassis's own 20, leaving 19.4 t spare.
- A tiny helicopter `11tin1` comes to 2.5 t against 2.775 + 0.275.

*Measured*, weighing every robot that way (`units.Workshop.weigh`): 364 of the
374 carry no more than their payload. If the chassis's body counted, 187 would
not. The ten over are nine large assemblies and one medium walker
(`23mwalk1e`, 33 t on 28). The worst is `AI_LW_31`, at 121 t on a 70 t Large
Wheel Chs. On those ten the spare payload is 0, which halves the top speed.

## What a chassis costs, and who builds it — *measured*

| chassis | code | kind | node 0 HP | research E/O | build E/O | tech level | placed by |
|---|---|---|---:|---|---|---:|---|
| R_T_01 | T-12w | walking | 120 | free | 6/15 | 0 | enemy |
| R_T_02 | T-2 | flying | 90 | free | 5/12 | 0 | enemy, player, neutral |
| R_L_01 | S-12w | walking | 450 | 7/20 | 7/20 | 1 | enemy, player |
| R_L_02 | S-2f | flying | 190 | 10/15 | 10/15 | 1 | enemy, player |
| R_L_03 | S-31 | wheeled | 320 | free | 5/22 | 0 | all three |
| R_L_04 | S-42t | tracked | 320 | free | 6/26 | 0 | player, enemy |
| R_L_05 | S-4f | flying | 190 | 15/15 | 15/15 | 2 | — |
| R_M_01 | M-12 | walking | 1,100 | 20/45 | 15/45 | 7 | enemy, neutral |
| R_M_02 | M-2f | flying | 480 | 25/50 | 25/45 | 14 | player, neutral |
| R_M_03 | M-32 | wheeled | 950 | 15/60 | 15/55 | 5 | all three |
| R_M_04 | M-42t | tracked | 950 | 15/75 | 15/70 | 7 | all three |
| R_B_01 | L-12w | walking | 4,000 | 25/95 | 25/95 | 14 | enemy, neutral |
| R_B_02 | L-2f | flying | 1,500 | 35/85 | 35/85 | 14 | all three |
| R_B_03 | L-32 | wheeled | 3,000 | 15/110 | 15/100 | 14 | all three |
| R_B_04 | L-42t | tracked | 4,500 | 20/150 | 20/120 | 14 | player, enemy |
| R_B_05 | L-22w | Transformer | 4,000 | — | — | 0 | enemy, neutral |
| R_B_06 | L-22w | Small Tower | 4,000 | — | — | 0 | enemy (28) |
| R_B_07 | L-7f | flying | 10,000 | — | — | 0 | enemy |
| R_B_08 | L-8f | flying | 80,000 | — | — | 0 | enemy |
| R_L_06 | S-6f | flying | 200 | — | — | 0 | — |
| R_L_07 | S-7f | flying | 83,000 | — | — | 0 | enemy |
| R_H_02 | HERO | hero | 380 | — | — | 0 | player (37) |

Costs are `objects.dlb`'s `Research*`/`Build*` pairs (energy/ore); *free* is a
zero research cost, the stock a player starts with; *placed by* counts mission
objects by their clan's type ([27-ownership.md](27-ownership.md)). Every chassis
shows the same three stat rows — weight in t, payload in t, max speed in km/h —
all computed at run time from the control system
([24-motion.md](24-motion.md#the-chassis-in-the-games-own-units--measured)).

**The research ladder climbs one kind at a time.** In each chassis's commonest
wiring across the 29 trees, a chassis needs the one below it of its own kind
plus its grade's factory and research centre ([16-research.md](16-research.md)):
the Small Walking the Tiny Spider, the Small Flying the Tiny Helicopter, each
Medium its Small, each Large its Medium, and the S-4f the S-2f. 11 of 11
prerequisite chassis are the same kind one size down or level; the tiny ones,
the Small Wheel and the Small Track need nothing.

**The costless chassis are not the player's.** The Small Tower, the L-7f, L-8f
and S-7f are placed only by enemy clans, and the hero only by the player
(*measured*). Those four, the Transformer and the S-6f have no cost and no
prerequisite, and all six are category 0 in 25–28 of the 29 trees. *Guess*: they are mission set pieces the
player cannot build.

**Category 0 means out of the tree** — *read*, and *measured*. The `TRF1` byte
is not a category but three state bits, which the loaded tree reads and writes
(`MisLoad.dll`):

| bit | meaning | read at |
|---:|---|---|
| 1 | available to research | slot 26's second answer (`0x10002aa0`) |
| 2 | researched | its first; slots 48 and 49 save and restore exactly this bit (`0x100034b0`, `0x100035f0`) |
| 4 | in this mission's tree | its third |

- **Finishing a research.** Slot 30 (`0x10002c10`) finishes research `i` only
  if bit 4 is set, and then sets bits 1 and 2.
- **What becomes available.** It then sets bit 1 on every item with bit 4 whose
  prerequisites all have bits 4 and 2.
- **What the values are.** Read that way, 7 is a researched item, 5 one open to
  research, 4 one still locked, and 2 one researched but out of the tree (the
  animals and the hero).
- **What 0 is.** None of the three: the item is out of the tree, cannot be
  opened and can never be researched there.
- **The data agrees** (*measured*, all 10,672 items in the 29 trees). All 177
  items at 5 have every prerequisite at 2 or 7, and all 2,044 at 4 have at
  least one that is not.
- **Where it goes next.** That a builder then refuses an item without bit 2
  is *derived* from this; the consumer of slot 26 was not read.
  [16-research.md](16-research.md)'s names for the values (special, creature,
  main, starting, basic) describe the shipped trees, not the bits.

## Not established

- What enforces a turret's size and a part's slot — narrowed: nothing in the
  load path. The robot constructor lists parts by name prefix
  ([What the label is not](#what-the-label-is-not--read-as-a-search)). Open:
  where its page items' names (`+0xc4`) come from, and so whether the internal
  pages key on the slot labels.
- ~~The order parts load in, and which same-class record wins~~ — answered:
  the `.dat` pre-order, root first; the last would win and no assembly has
  two ([The order parts load in](#the-order-parts-load-in-and-what-a-slot-keeps--read-and-measured)).
- ~~Whether a chassis slot's default stays alongside the fitted part~~ —
  answered: it does not; only the node and the slot's entries survive the
  re-parse.
- ~~Which node range the payload sum treats as the chassis's own~~ —
  answered: part 0's, the root object's
  ([What a chassis weighs](#what-a-chassis-weighs--read-and-measured)).
- ~~The class-3 records~~ — answered: simple devices that turn wheels, steer,
  tilt and spin rotors from the motion
  ([The class-3 records](#the-class-3-records-turn-the-wheels--read-and-measured)).
  `r_l_06`'s purpose stays open. Only `objects.rlb`, `objects.dlb`,
  `bases.rlb` and the 29 `.trf` name it; no `.dat`, mission or binary does
  (*measured*).
- ~~Research-tree category 0~~ — answered: none of the three state bits, out
  of the tree ([above](#what-a-chassis-costs-and-who-builds-it--measured)).
  Open: what reads slot 26 to allow a build.
- ~~How a device's value becomes a pose, and what turns a radar~~ — answered:
  a stepped item whose channels play their nodes' frames, and the radar is one
  ([What a device's value turns](#what-a-devices-value-turns--read-and-measured)).
  Open:
  - **who sets the two countdowns** (`+0xd0`, `+0xd4`, through `IControl`
    slot 8) under which a device's channels are played every tick rather than
    only as each step starts — so whether a rotor seen in play turns smoothly
    or in 162° jumps is not read;
  - **what a byte 0 of 1 adds**: the float `0x10020c25` finds in the machine's
    list at `+0xc4` (count `+0x38c`, 0x5c-byte entries keyed at `+0x24`), over
    the channel's span, which the held wheels and rollers play;
  - what a control system of agent kind 10 is: its tick skips the states and
    the devices both (`0x1000c294`).
