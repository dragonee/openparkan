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
Nothing read enforces it; the data never breaks it.

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

**What the label is not** — *read*, as a search. The component parser keeps a
copy of the label on the component object (`+0x30`, `Control.dll:0x10021f00`),
but nothing found compares it: `Control.dll`'s one string compare
(`0x1003a670`) is called only on `(archive, member)` pairs in its resource
caches, and a loaded part is appended, not matched to a slot (next section).
*Guess*: the label is the unit editor's fitting rule, and the data obeys it.

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

## The parts join one control system — *read*

Assembling a unit does not give each part its own machine. `AniMesh.dll`
attaches every part to the one agent (`0x10003760`, a list of 76-byte entries at
`+0x6e4`): it merges the part's mesh into the agent's model and sends the
agent's control system the load message `0x80000020` with the part's three
`(archive, member)` pairs, the part's id and its first node (`0x10003b6f`).
`Control.dll`'s loader (`0x10008b10`) then **appends the part's components to
the same device list**, rebasing their node indices into the merged model
(`0x10009081`), skips the part's parameter block and animation states — those
come from the first controller, the chassis's — and rebinds every device
(`0x1000789c`).

What that means for a slot and its part, which are then both in the list:

- **Classes the control system keeps one of are the last loaded.** The factory
  overwrites its single pointer for a turret (`+0x5c8`), fight shield (`+0x5cc`,
  `0x1002d56e`), deflector (`+0x5d0`), radar (`+0x5d4`) and class 17 (`+0x5d8`)
  with every record of that class, and the armour struct's three numbers with
  every class-27 record (`0x1002d7fc`). A fitted shield generator, deflector,
  radar or armour loaded after the chassis is the unit's. That parts load in
  assembly order, chassis first, is a *guess*.
- **Classes that are summed add up.** The drive sums every engine's value
  (`0x1000fca0`, [24-motion.md](24-motion.md)), and every store is drawn on. So
  a unit's engine figure is the chassis slot's 1 plus the fitted engine's, and
  its charge is the slot's 10,000 plus the fitted battery's — *derived*.

This settles the open question in [26-damage.md](26-damage.md) of which parts
join a unit's control system: all of them.

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

A whole unit's weight follows the same sum over every part — *derived*. The
tracked `w_b_trk1` comes to about 82.6 t (the chassis 20 t, the turret, guns and
mounts 20.5 t, the internal parts 21.6 t, armour Mk3 at 22.5 × 915 of area 20.6
t) against 80 t of payload plus the chassis's own 20; a tiny helicopter
`11tin1` to 2.5 t against 2.775 + 0.275. That the payload's "own node range"
([24-motion.md](24-motion.md#load--read-and-measured)) is the chassis's nodes
is still a *guess*, and so these spare-payload figures are too.

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

## Not established

- What enforces a turret's size and a part's slot, if anything does at run
  time — the label and the size letter are obeyed by the data and read by
  nothing found.
- The order parts load in, and so which of two same-class single-pointer
  records wins when a chassis and a part both carry one.
- Whether a chassis slot's default (a 10,000 battery, a value-1 engine) is meant
  to stay alongside the fitted part, or whether the data relies on it.
- Which node range the payload sum treats as the chassis's own.
- The class-3 records on wheeled, tracked and flying chassis, and `r_l_06`'s
  purpose.
- Research-tree category 0 ("special"): what it does to an item.
