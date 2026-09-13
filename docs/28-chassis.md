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

So the size letter and the slot label are kept by what the editor offers,
not by what the engine accepts. Where each page item's name comes from is not
traced, so that the internal-part pages are keyed on the slot labels is still
a *guess*.

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
vtable `0x1003c448`). Each tick (slot 11, `0x10020900`) it moves the section-2
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
  and `+0x28` and added to the device's value each tick. A byte below 2 picks
  nothing: byte 1 then stands for 1 and byte 2 for 0, so a record with both at
  0 advances at a constant rate.
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
| `0x8000000C`, `0x8100000C`, `0x8200000C` | — | engines and wings of four flyers | set to forward speed ÷ top: they tilt |
| `0` | 1 | the T-2's two rotors, and the building's | a constant: they spin all the time |

So the unlabelled records are animation, not slots.

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
