# The economy — ore and power

How a clan's ore and power are made, moved, shared and spent. Most of it lives
in `Behavior.dll`, which logs its own arithmetic by name; the per-building
numbers live in `behpsp.res`; and the missions carry three of the engine's
constants as properties, which is how the binary reading is cross-checked
against data.

**Every claim is tagged.** *Measured* means `openparkan verify` re-derives it
from the shipped data. *Read* means it comes from the disassembly, with the
address given, and has no data-side check. *Guess* fits the evidence and is
not established. *Unknown* means exactly that.

## The pieces — *measured*

`behpsp.res` is an NRes archive of 32 binary `.var` profiles, one per role; see
`openparkan/profiles.py` for the format. The building profiles carry the
economy:

| profile | power out | power used | ore used | ore held | ore on | ore off |
|---|---|---|---|---|---|---|
| `prof_generator` | **10** | 0 | 0 | 0 | 0 | 0 |
| `prof_mine` | 0 | 1 | 0 | 1000 | **0** | 1 |
| `prof_storage` | 0 | 1 | 0 | 4000 | **20** | 1 |
| `prof_plant` — the factory | 0 | 4 | **5** | 0 | 100 | 0 |
| `prof_institute` — research | 0 | 3 | 1 | 0 | 100 | 0 |
| `prof_trn` — transport | 0 | 0 | 0 | 1000 | 100 | 100 |
| `prof_tower`, `prof_mast` | 0 | 10 | 0 | 0 | 0 | 0 |
| `prof_angar` | 0 | 3 | 0 | 0 | 0 | 0 |
| `prof_teleport` | 0 | 2 | 0 | 0 | 0 | 0 |
| `prof_bunker` | 0.07 | 1 | 0 | 0 | 0 | 0 |

The columns are `Transfer_Power_Out`, `Use_Power`, `Use_Ore`,
`Store_Ore_Maximum`, `Transfer_Ore_OnBoard` and `Transfer_Ore_OffBoard`.

**`prof_plant` is the factory, not a power plant.** The engine's own error text
calls a factory's build node a plant — "Plant creation node destroyed" — and it
is the one building profile that constructs. The power source is **`prof_generator`**,
and apart from a debug `prof_universal` (100) and a bunker's 0.07 it is the only
profile that puts power out at all.

## The engine's constants — *measured*

`Behavior.dll` compiles a table of 70 behaviour constants into a constructor at
`0x10016250` and binds them by name at `0x10016480`. No file in the
installation names any of them, so the compiled values are the ones the game
runs with. The economy's share:

| constant | value |
|---|---|
| `Mine_OrePerSecond` | 50 |
| `Mine_MaxOre` | 500 |
| `Storage_MaxOre` | 4000 |
| `Transport_MaxOre` | 2000 |
| `Transport_OreOnBoardPerSecond` | 100 |
| `Transport_OreOffBoardPerSecond` | 100 |
| `Transport_BuildingDist` | 80 |
| `Building_Cost` | 100 |

The check that these are live is in the missions. The `MaximumOre` property
takes exactly four values across every placed object, and they sort by what the
object is: **500 on every mine**, **2000 on every transport and every builder**
— the object paths put them in `UNITS\UNITS\TRANSPRT` and `…\BUILDER` —
**4000 on every storage**, and 0 on everything else. Those are the engine's
three capacities, written in by the editor. A mine's profile says it holds 1000
and a transport's 1000; the missions and the engine agree with each other and
not with the profiles. A builder carrying ore is what `Building_Cost` is paid
from — a *guess*, since the construction task is unread.

## How ore reaches a consumer — *read*, after two corrections

Every ore holder, **a mine as much as a storage, feeds consumers directly**.
The distribution step (below) draws from each at **its efficiency times its
`Transfer_Ore_OffBoard`**, per second, capped by what it holds — *read*, at
`Behavior.dll:0x1001a8c6`. That off-board rate is **1 on a mine and 1 on a
storage** (*measured*). So a mine on its own does supply the factory, slowly.

The game says so in its tutorial, `TextRes.dll` string 101 — "the Mine works
kinda slow, and if your base doesn't have a Warehouse, you'll always have to
wait to accumulate enough raw materials" — and string 103: without a
warehouse "your Factory's production … will be limited by the Mine's
parameters". What a warehouse adds is a second outlet and a 4000 buffer that a
transport fills from the mine at 100 a second, so the mine never sits full at
500 and idle. A transport picks the nearest mine with a free loading place and
the nearest storage, leaves a mine part-loaded once it runs dry, and waits
beside a full storage ([32-builder.md](32-builder.md#transporting-ore--read-and-measured)).

**The id table**, read off `MBehaviour`'s variable getter
(`Behavior.dll:0x1000a490`, vtable slot 26, a switch):

| id | what it returns |
|---|---|
| `0x1001` | `Transfer_Ore_OnBoard` |
| `0x1002` | efficiency × `Transfer_Ore_OffBoard`, computed on each call |
| `0x1004` | `Transfer_Power_In` |
| `0x1005` | `Transfer_Power_Out` |

`0x1003` is not handled at all, and a holder's ore contents are not one of
these ids but a separate property, `0x2000100`, read and written through two
other slots.

**Two earlier versions of this section were wrong, in sequence.** The first
said a mine feeds nobody until a transport carries its ore. The second
retracted that on the tutorial's word but still read the rate as
`Transfer_Ore_OnBoard` — 0 on a mine — because it took the ids to be
`0x1000 +` a variable's position among the floats. That numbering was an
inference, and only `0x1005` happened to agree with the real table. With the
table read, the rate is the off-board one, a mine's is non-zero, and the
tutorial and the code agree.

## The sharing formula has two tiers — *read*

`Behavior.dll:0x10019e80` runs once per clan per distribution step and logs
every quantity it uses. Generators are the building types `0x80000002`,
`0x80010000`, `0x80020000` and `0x80040000`, and each **gives
`Transfer_Power_Out × dt`**. Mines and storages are `0x80000004` and
`0x80000008`. Every other building keeps a charge and **asks for what it
lacks**: its need times one minus its current fill.

The step then totals supply (`fPowerAvailable`) against demand
(`fPowerUsage`), with part of each building's demand, set by its priority
(`fPriority`), counted as a first tier, and fills the tiers in order:

```
if Available ≤ FirstTier:       A = Available / FirstTier;  B = 0;  Take = 1
elif Available < Total:         A = 1;  B = (Available − FirstTier) / (Total − FirstTier);  Take = 1
else:                           A = 1;  B = 1;  Take = Total / Available
```

**`fPowerA`** is the share of first-tier demand met, **`fPowerB`** the share of
the rest, and **`fPowerTake`** the share of the generated power actually drawn —
below 1 only when there is a surplus, which is simply not taken. A generator is
set straight to full. Ore goes through the identical formula as `fOreA`, `fOreB`
and `fOreTake`, with a holder's ore and a consumer's free ore space in place of
power and charge.

**The tiers are not a set of buildings.** A priority `p` is a number, and it
splits *each* building's demand: the first tier is `Σ want × p`, and a consumer
is topped up by

```
received = want × (p × A + (1 − p) × B)
```

*read*, at `0x1001a7cb` for power and `0x1001a881` for ore.

## Every building has the same priority — *read*

The distributor keeps one 0x1c-byte record per building — id, priority, a lock
flag, and the power and ore wanted and received — and its vtable
(`Behavior.dll:0x10059794`) manages the priorities so that **they always sum to
1**:

| slot | address | what it does |
|---|---|---|
| 3 | `0x1001a9a0` | set one building's priority and lock flag, clamped to what the locked ones leave |
| 4 | `0x1001aac0` | add a building at `1 / (n + 1)` |
| 8 | `0x1001abf0` | remove a building |
| 10 | `0x1001ac50` | reset every priority to `1 / n` and unlock all |

After each change `0x1001ac90` rescales the unlocked priorities to fill whatever
the changed and locked ones leave.

**Only add and remove are ever called.** A building's behaviour registers
itself when it joins a clan and unregisters when it leaves
(`Behavior.dll:0x100060dc` and three more; `0x10008f71`, `0x1000ce6b`),
reaching the distributor through `ai.dll`'s `GetSuperAI(clan)` — imported by
ordinal — and SuperAI slot 11. Adding at `1 / (n + 1)` and rescaling the rest
keeps every share equal, and so does removing. **No call to the setter or the
reset exists anywhere in the install:** across every module, 87 indirect calls
at the setter's offset push four values since the last call or branch; 68 of
those push an address built by `lea`, which none of the setter's arguments is,
and the other 19 turn out to be a function's own register saves plus one
argument, or a different interface's `(self, id, flags)`. The lock flag, which
only the setter writes, is therefore never set. That negative is a search, not
a proof: a call that computed an argument by calling something else between its
pushes would be missed.

**With every `p` equal, the two tiers collapse.** Put `p` into the formula. If
`Available ≤ p × Total`, a building receives `want × p × Available / (p ×
Total)`; otherwise `want × (p + (1 − p)(Available − p × Total) / ((1 − p) ×
Total))`. Both come to the same thing:

```
received = want × min(1, Available / Total)
```

So **no building is served first**. When power or ore is short, every consumer
gets the same fraction of what it lacks, and that fraction is what the clan
produces over what all of its consumers want.

## Research — *read*

`M_Task_Research::OnBehaviourTakt` (`Behavior.dll:0x1002f730`) runs each tick.

- **A research has three budgets** — ore and power, from the item's research
  costs in the `.trf`, and time — and the task computes **the smallest of its
  three completion fractions**.
- **The time budget is the research centre's `FreeResearchTime`**, the same for
  every technology it researches, and time accrues as plain `dt` in seconds —
  see below.
- **Free technologies cost nothing.** While the research centre's own
  `FreeTechnoNum` is above zero, starting a research decrements it and sets the
  ore and power budgets to zero.
- **It draws `KPD × Use_Ore × dt` ore and `KPD × Use_Power × dt` power** each
  tick, the ore capped by the institute's own buffer, and then requests the ore
  still missing — half of it while more than 5 remains. `KPD` is logged by that
  name: Russian *КПД*, efficiency.
- **`KPD` is the building's efficiency**, below. `0x100198e0` totals property
  `0xe00` over one list of the building's components; `0x10019880`, the
  `SetPowerUsage` call, writes property `0x500` on each of them.

### The four grants a mission gives a building — *read*, and *measured*

Every placed object carries `FreeBotNum`, `FreeTechnoNum`,
`FreeConstructionTime` and `FreeResearchTime` ([04-missions.md](04-missions.md)).
`MBehaviour` registers them as property kinds 7 to 10 and its setter
(`Behavior.dll:0x1000b470`, slot 2 of the interface at `+4`) writes kind 7 to
`[self + 0x9e8]`, 8 to `+0x9f0`, 9 to `+0x9ec` and 10 to `+0x9f4`. **`self`
there is the `+4` sub-object** — the setter reaches the object's own vtable
through `[self − 4]` — so each field is four bytes further into the
`MBehaviour` than the switch makes it look:

| property | field | read by | before a mission sets it |
|---|---|---|---|
| `FreeBotNum` | `+0x9ec` | construction: "Begin to constructing free bot" while above zero, and decremented when a build completes | 0 |
| `FreeConstructionTime` | `+0x9f0` | nothing found | 5 |
| `FreeTechnoNum` | `+0x9f4` | research: free while above zero, and decremented | 0 |
| `FreeResearchTime` | `+0x9f8` | research: `fTimeCost` of every research (`0x1002f942`) | 2 |

The defaults are the constructor's (`0x10003a3c`). Read against them without
the offset, the table would put the research time into the free-technology
counter and the construction time into the free-bot counter.

The missions agree with the corrected table (*measured*). `FreeBotNum` is
non-zero on 27 objects, **all factories**; `FreeTechnoNum` on one, **a
research centre** — the enhanced one in `CAMPAIGN.00/Mission.04`, with 5.
`FreeConstructionTime` leaves 5 on 10 objects, every one a factory that grants
bots, and `FreeResearchTime` leaves 2 only on that same research centre. So a
research takes at least two seconds on every map but one, and what sets its
pace is ore and power: the ore fraction grows at `KPD × Use_Ore × dt` over the
ore cost, the power fraction likewise.

`FreeConstructionTime`, by its values, is meant for the free bots, but no
instruction reads `+0x9f0` — a search by displacement, not a proof.

## Efficiency is a building's size

**Which components count** — *read*. When a building's behaviour binds to its
controller, `Behavior.dll:0x100186b0` asks `IControl` for its components class
by class and files classes 26, 25, 29, 10 and 15 in five lists, with a sixth
filled through another interface. The list `KPD` totals is **class 26**
(`0x1001874c`).

**What a class-26 component reports** — *read*. `Control.dll:0x1002bb40`
answers property `0xe00` according to the component's class: class 26 returns
its value `0x300`; class 10 the mean of `0x300`–`0x302`; class 21 the mean of
`0x300`–`0x305`, six of them; class 5 its value `0x100`. Only class 26 reaches
`KPD`.

**What a value id is** — *read*. Every component's value getter
(`Control.dll:0x10021d00`, vtable slot 4) takes an id's **low byte as an index
into the sixteen floats** its `.ctl` record carries at `+0x2c`
([13-control.md](13-control.md)), then multiplies by up to two run-time
factors:

| id bit | multiplies by |
|---|---|
| `0x100` | **the life left in the part**: its model node's hit points over their maximum, 1 intact and 0 destroyed |
| `0x200` | a level held at `+0x4c`, which starts at 1 and can be set only while the component's state at `+0x50` is non-zero and it is not destroyed |

So `0x300`–`0x305` are **the first six floats of the record, scaled by
condition and level**, and an intact building's `KPD` is its class-26
component's first float. The level is its share of the building's own power,
below.

**Condition is a node's life** — *read*, and *measured*. Each component names
a node of its model at record `+4`, and asks the control system about it
(`Control.dll:0x1000dc40`, id 1). The control system builds one node record per
entry of the object's `.ndp` damage table
([07-objects.md](07-objects.md#ndp-is-a-damage-table-one-record-per-node)): life
starts at the `.ndp` durability times the object's volume scale and the
difficulty's level ratio (`0x1000f940`, [26-damage.md](26-damage.md)), damage
lowers it clamped at 0 (`0x10010f30`), and the node's fraction `life / max` is
what id 1 returns. On all 781 components that sit beside a same-named `.ndp`,
the node index falls inside its table; shuffle the pairing and 308 fall
outside. So **shooting the part that carries a building's efficiency lowers
`KPD` in proportion** — and 22 of the 31 efficiency parts can be shot down,
with durability 35,000 to 500,000; the other 9 carry the 1,000,000 that means
a node cannot be destroyed.

**The data** — *measured*. Class 26 occurs only in `fortif.rlb`, once per
building controller (five times on the small main teleport), always with
fifteen zeros after the first value. Its first value follows the model number
in the building's name:

| | small, -17 | medium, -30 | large, -47 | enhanced, -67 |
|---|---|---|---|---|
| research centre | 1 | 3 | 5 | 7 |
| factory | 1 | 3 | 5 | — |
| core mine | 1 | 3 | 5 | — |

and every other building — storage, generator, tower, bunker, Outpost, bridge,
ruin — is 1.

**What it multiplies** — *read*:

- **A mine digs `Mine_OrePerSecond × KPD × dt`** into its own store, up to its
  capacity (`Behavior.dll:0x1002d06c`, logged as `fMinedOre`): 50, 150 and 250
  ore a second by size. A building with no `Use_Power` digs at a `KPD` of 1, and
  one whose `KPD` is below 0.1 stops and logs `BAD KPD`.
- **It gives ore at `KPD × Transfer_Ore_OffBoard`**, the `0x1002` id above.
- **Research and construction draw ore and power `KPD` times faster.**
- **A factory's ore cost is divided by `KPD`** when a build starts
  (`0x1002a2a7`, logged as `NewfOreCost`), so a large factory pays a fifth of
  what a small one does. Research's one `KPD` call multiplies; nothing there
  divides a cost.

## A power shortage lowers efficiency, once the batteries run down — *read*, and *measured*

A building does not run on the clan's power directly. It runs on its own
**batteries** — its class-19 components, `CICLS_POWERSTOR`, the `i_pws` parts —
and the distribution step above is what refills them.

**The distribution step's "charge" is the batteries** (*read*). The building's
control system answers the step's two questions over its class-19 components:
the fill is `Σ capacity × charge / Σ capacity` (`Control.dll:0x1002b42b`) and
the capacity `Σ capacity` (`0x1002b4e9`), capacity being a battery's first
value. Topping a building up writes the new fill into every battery's charge
(`0x1002bae6`). A battery with a negative capacity makes the whole building
read full: that is a generator's (`fr_l_gener`, −1).

**Every controller tick spends the batteries** (*read*,
`Control.dll:0x1002d340`, `dt` in seconds). Each component reports a flow
through its vtable slot 5. A battery gives `min(output × charge × condition ×
dt, capacity × charge × condition)`; the efficiency component takes `(power +
usage) × dt`, where *power* is a float in its record at `+0x20` and *usage* is
what `SetPowerUsage` gave it. Other classes price their draw their own way —
see the next section. What the batteries give is then handed out **by
channel, in a fixed order**, each class having one channel
(`Control.dll:0x1003ccc8`):

| served | channel | classes in it |
|---|---|---|
| 1st | 3 | engines (5), **the efficiency component (26)**, 16, 20 |
| 2nd | 0 | doors (12), computers (13), repair (15), elevators (11), 3, 25, … |
| 3rd | 2 and 5 | radar (8), camera (4); shields (9, 10), deflectors (21), armour (27) |
| 4th | 4 | turrets (1), guns (2), 22, 24, 30 |
| last | 1 | the batteries (19), and 14 |

Each channel's consumers all get the same **level**, `min(1, what is left /
what the channel wants)` (`0x1002dca0`), written through slot 6 into the `+0x4c`
factor above; the batteries, served last, drain by the share of their output
that was used. The class numbers are the engine's `CICLS_` ids (*measured*:
every `.ctl` label family sits on one class, and the named ones agree —
`i_pws` on 19, `i_fsh` 9, `i_dsh` 10, `i_eng` 5, `i_rdr` 8, `i_rps` 15).

**So a building's efficiency is served first and alone** (*measured*). On all
27 building controllers that have an efficiency component, it is the only
thing on channel 3. Its own power figure is 0.01; the batteries put out 50 to
52 a second at full charge, holding 19.5 to 20. Its level — and `KPD` — stays
at 1 until the batteries hold less than `(0.01 + Use_Power) / output`: **8.0%
for a small factory**, about 6% for a research centre, 2% for a mine.
Below that it falls in proportion to the charge.

That is how a short clan slows its work. The distribution step refills every
building's batteries by `Available / Total` of what they lack; a working
building drains its own; and once its charge sinks under that threshold its
`KPD` drops with it — and with `KPD` its ore and power collection, a mine's
digging, and so its progress. Until then a shortage costs nothing but charge.

### How often, and where it settles — *read*, with a derived settle point

The two sides run on separate timers, both measuring `dt` in real seconds from
the game's millisecond clock:

- **The distribution step runs every 192 to 255 ms.** The main loop calls each
  clan's SuperAI every frame (`iron3d.dll:0x1005edc9`), which calls the
  distributor's tick first and unconditionally (`ai.dll:0x1000178f`); the tick
  itself waits for a timer of `3 × 64` ms plus a random `0..63`
  (`Behavior.dll:0x10019e1b`, `0x1004c569`). The SuperAI's own thinking after it
  runs only every 7 to 8 seconds, and does not gate the distributor.
- **A building's power tick runs every 250 ± 31 ms** (`Control.dll:0x1000c756`):
  250 is a compiled constant, the jitter a sixteenth of a turn of a shift
  register, and a countdown of 100 set by a message forces it on every
  controller tick for a while.

Neither is a multiple of the other, and the jitter keeps them from locking.
Neither timer is in any data file.

**The top-up never overshoots**: a building receives at most `capacity × (1 −
fill)`, and a clan's surplus is simply not drawn. Full buildings ask for
nothing, so **only the draining buildings share a shortage**. For `N` equally
busy buildings on a clan making `G` power a second, each with draw `d = 0.01 +
Use_Power` and battery output `R`, the averages settle at

```
level (and KPD)   r = min(1, G / (N × d))
charge            f = r × d / R
```

— a derivation from the read formulas over the discrete steps, so a *guess* in
its exactness. One generator (`G` = 10) and three small factories building at
once (`d` = 4.01, `R` = 50) settle at a level of 0.83 and a charge of 6.7%:
research and construction at 83% speed. The building's capacity drops out,
which is measurable to matter little anyway — every `fortif.rlb` building with
batteries holds 19.5 or 20 and puts out 50 to 52 a second.

## Bots spend power through the same code, priced by part — *read*, and *measured*

**The machinery is shared.** `LoadControlSystem` (`Control.dll:0x10032280`)
builds the same 0x670-byte control system for every agent except a projectile
(kind 9, a `BULL` record, which gets a smaller class with a different tick), so
a robot and a building run the same power tick, the same channels and the same
battery code.

**What differs is what draws, and what refills.**

| | a building | a bot |
|---|---|---|
| batteries | `i_pws` parts holding 19.5–20, 50–52 a second | the fitted battery, which replaces the chassis's 10,000 at 250: 3,000–4,080 at 5–6.8 a second small, 9,600–12,000 at 12–15 medium, 22,000–31,000 at 25.5–34.5 large ([28-chassis.md](28-chassis.md#a-fitted-part-takes-over-its-slot--read-and-measured)) |
| refilled by | the clan's generators, through the distribution step | never by the distributor (*measured*: its registration needs bit 31 of `Type`, set on all 167 placed buildings and none of the 296 units); a docked unit gains 10% of a full charge a second (`Behavior.dll:0x10019372`, "Reloaded") |
| main draw | the efficiency component, `(0.01 + Use_Power) × dt` | engines, first on the same channel |

Each class prices its own draw in its vtable slot 5, read per class:

| class | draw a second |
|---|---|
| default — doors, computers, radar, deflectors, armour | `power` while switched on and intact (`0x10021860`) |
| efficiency (26) | `power + usage` (`0x1002e540`) |
| engine (5) | `power × speed ÷ top speed × the state's factor` (`0x100265c0`), speed and top speed each the largest of three axes: the machine's velocity (`+0x1c8`) against the controller frame's third triple; the factor at `+0x154` is the current animation state's, 0 at rest and on the hero ([24-motion.md](24-motion.md)) |
| gun (2, 30) | `power` plus whatever its capacitor lacks of value 1; its level is the capacitor's fill (`0x10029a40`, `0x10029a90`). A shot takes value 2 out of the capacitor, and a gun whose value 1 is 0 fires without power ([29-weapons.md](29-weapons.md)) |
| fight shield (9) | `power` plus `value 2 ×` the recharge it does: `min(value 1 × condition, value 0 × 6 − the six sectors' strength)` (`0x10025700`) |
| detect shield (10) | `power`, plus value 4 while it is in mode `0x1000` (`0x100264b0`) |
| repair (15) | `power` plus `value 1 ×` the hit points it restores, at most `value 0 × condition` a second (`0x10022b20`) |
| battery (19) | a source: `min(output × charge × condition, capacity × charge × condition)` (`0x100229a0`) |

The numbers make the split plain (*measured*). A building's fight shield
recharges 80 a second of 8,000 a sector at 0.00015 a point; a bot's large
`o_fsh_b` shields recharge 50–80 of 3,350–3,800 at 0.06 a point, and a chassis
shield 10–100 at 2. A building's repair system restores 100 a second at 0.0002
a point; a bot's large one, 50–80 at 0.06. (This page once said the building
figures were free: an earlier survey rounded them to 0.) A robot's engine is
its fitted one, 0.75–11.2 a second at full speed; the chassis slot's 20 is
replaced. So on a bot, moving, shield recharge and repair all cost charge, and
weapons, served fourth, get what is left.

*Derived*: a store gives at most its power figure × its charge a second
(`0x100229a0`), so a robot's whole supply is its battery's 5–34.5 a second, not
the chassis's 250. A small battery's 5–6.8 does not cover a small red laser at
full rate (7.1 a second) on its own.

## The bot limit is the clan's mind count — *read*, and *measured*

What caps a clan's army is not a factory setting but its **minds** — "Available
CPUs" in the game's own interface (`iron3d.dll` string 3067).

- **Where the number comes from.** The word after a clan's behaviour-tree path
  in `data.tma`, 2 to 17 ([04-missions.md](04-missions.md)). `iron3d.dll`
  fills the clan SuperAI's mind list with that many free entries
  (`0x10039266`); nothing found adds more during a mission.
- **What holds one.** Starting a build takes a free entry and marks it
  reserved (`Behavior.dll:0x1002a348`, and `0x1002a0bb` for a free bot); with
  none free the factory logs "No Free mind... cannot start constructing" and
  does not start. When the build completes the reservation is dropped and the
  new bot claims an entry by its id (`0x1001e0e0`, "Attached to Brain"). A bot
  captured or placed by the mission takes one too, and with none free that is
  a "Behaviour panic".
- **What frees one.** A bot destroyed or deleted, or captured by another clan
  (`ai.dll:0x10003e40`, `0x10006530`), or a build that is aborted
  (`0x10029910`).
- **What the player hears.** When the player clan has none free, the
  constructor plays `VOICE_NO_CPU` (`iron3d.dll:0x1005ee43`).
- **A refused build is dropped, not retried** — *read*. A factory's orders are
  tasks on a stack (`Behavior.dll:0x10034930`). When `Task_Construct` cannot
  find a free mind its start returns failure (`0x10029c30`); the stack logs
  "Cannot StartTask", reports the order back the same way as a finished one —
  "Reported that Order Done" (`0x10005560`) — removes it, and starts the next
  order in the queue, skipping any that fail in turn (`0x10034a30`). Nothing in
  the factory waits for a mind to come free: a new order has to come from the
  player or the clan's AI.

So a clan with 5 minds can have at most 5 bots alive or under construction,
and its factories stop until one is lost. Buildings take no mind.

*Measured:* across all 101 shipped clans no clan is placed with more robots
than its minds, two sit exactly at the limit, and counting every owned object
instead, 18 would exceed it.

## Construction — *read*

`M_Task_Construct::OnBehaviourTakt` (`Behavior.dll:0x1002a4f0`) is the factory
building a bot, and it has the same three budgets as research — ore, power and
time — with one difference in how ore is pulled.

- **Time** accrues as plain `dt`; **power** as `KPD × Use_Power × dt`.
- **A paid bot's time budget is 5 seconds**, whatever the bot: the start
  routine (`0x10029ba0`) sets it before anything else and only a free bot
  changes it. Ore and power, from the bot's technology, set the pace.
- **Ore** is requested each tick as

  ```
  f        = ore collected / ore cost
  k        = 0.2 below 20% collected;  f from 20% to 80%;  3 above 80%
  request  = (ore cost − ore collected) × k × KPD × 0.2 + 0.07
  ```

  so a build pulls ore slowly at first, in proportion through the middle, and
  three times harder at the end, always scaled by efficiency. The request is
  not multiplied by the tick's length.
- Once power is fully collected it calls `SetPowerUsage(0)` and stops drawing;
  once ore is, it withdraws its request.
- It **completes when all three are collected**, then decrements the
  factory's own `FreeBotNum` if it is above zero (`0x1002a7f9`).
- Until then its **progress is `min(time, ore, power)` as fractions, capped at
  1**, logged as "Construction in progress".

**What a free bot is spared** — *read*, at `0x10029f13`. While the factory's
`FreeBotNum` is above zero a build logs "Begin to constructing free bot" and
costs **no ore and 1 power**, in place of its technology's price. It is not
quicker: its time budget comes from a table of factory size against chassis
size (`0x1002a000`), in seconds —

| factory | tiny | small | medium | large |
|---|---|---|---|---|
| small | 30 | 60 | — | — |
| medium | 20 | 35 | 60 | — |
| large | 10 | 20 | 40 | 60 |

— and 20 for anything else. The mission's `FreeConstructionTime` plays no part.

**Sizes are letters in names** — *read*, and *measured*. A chassis's third
character is its size (`t` 1, `l` and `h` 2, `m` 3, `b` 4) and a building's
fourth (`l` 2, `m` 3, `b` 4, `e` 5), read at `0x10029e10` and `0x1000cee0`. A
factory refuses a chassis bigger than itself — "Robot SizedType not match". The
letters agree with the model codes in the labels: `R_T_` chassis are `T-`,
`R_L_` `S-`, `R_M_` `M-`, `R_B_` `L-`, on all 299 that carry one, and `fr_l_`,
`fr_m_`, `fr_b_`, `fr_e_` buildings are -17, -30, -47 and -67. So a small
factory builds tiny and small chassis, a medium one adds medium, and a large
one every size.

## What the HUD shows — *read*

In strategic control the top right carries two bars, **Ore** and **Energy**,
each with a percentage beside it. The labels are string ids 5092 and 5093 in
`iron3d.dll`'s own string table, the panel is built at `0x1006d290`, drawn at
`0x1006d510`, and updated at `0x1006d8f0`, which asks the player clan's
distributor for its totals — `0x1001ac40`, `return self + 0x30` — and computes:

```
Ore     = ore held in the clan's mines and storages / 4500
Energy  = (power available − power demanded) / power available, summed over every clan
```

each clamped to 0..1 and shown as a whole percentage.

- **4500 is one full mine plus one full storage** — `Mine_MaxOre` 500 plus
  `Storage_MaxOre` 4000 — so **a lone full mine reads 500 / 4500 = 11%**.
  A mine that is full and not being drawn from sits there, which is the "a
  mine gives 11% constantly" the game shows. A full storage alone reads 89%;
  both full read 100%. (The scale is *read*; that it equals the two
  capacities is *measured*.)
- **Energy is a share of the whole map's power**, not a fill level. The
  denominator sums every clan's available power, so owning one generator of
  three equal ones, with nothing outstanding to recharge, reads 33%. Demand
  still to be met is subtracted first, so it dips while buildings charge. The
  missions place between 0 and 5 generators, most often one or two, so what
  one generator is worth depends on the map.
- **Confirmed in play on The Convoy** (`CAMPAIGN.03/Mission.02`), where the
  game showed Energy 33%. The map places three generators, all `gener01.dat`,
  one each for the player, `Enm1` and `Enm2` — a third of the map's power
  (*measured*). The Ore 11% on the same screen is consistent but not provable
  from the file: the player starts with no mine there — the two mines belong
  to `Enm1` and a neutral clan, both empty — so it is a mine taken or built
  and left to fill to 500.
- **How the HUD reaches the distributor**, every link read. `iron3d.dll`
  keeps a 0x68-byte record per clan from `world + 0x724`, and its `+0x50` is
  the object `ai.dll`'s `CreateSuperAI` returns (stored at
  `iron3d.dll:0x100391d9`). That SuperAI is 0x8b0 bytes with its vtable at
  `ai.dll:0x100341b8`, and embeds a clan-brain object at `+0x7c` which, while
  the SuperAI is being built, calls `CreateDistributor` — `ai.dll` imports it
  from `Behavior.dll` — and keeps the result at its own `+0x3bc`
  (`ai.dll:0x1000637d`). The HUD calls SuperAI slot 11, `0x10001fc0`, which
  returns `[self + 0x438]` — that same pointer, `0x7c + 0x3bc` — and then
  the distributor's slot 9, `return self + 0x30`, the block the distribution
  step writes.
- The numbers do not jump: every 0.05 s each displayed value steps by one
  point toward its target. A bar whose value is 0 flashes on a half-second
  timer.

## Against what the game looked like

The HUD shows ore and power as percentages, a mine gives about 11%, a
transport turret carries ore from the mine to a warehouse, one power plant
holds about 33%, a factory builds far slower when both are low, and
construction slows research.

- **Transport from mine to warehouse** — confirmed by the game's own tutorial
  (strings 103, 104). A mine also supplies consumers directly, at its
  efficiency times an off-board rate of 1; a warehouse adds a second outlet and
  a buffer a transport keeps full.
- **Slower when low** — *read*, for both the factory and research. Power and
  ore accrue in proportion to efficiency, and progress is the smallest of the
  three completion fractions, so whichever resource is short sets the pace.
  Power reaches it through the building's batteries: efficiency falls once they
  drop below a few percent.
- **Construction slowing research** — *read*, when the clan is short for long
  enough. Every building's batteries get the same fraction of what they lack,
  `Available / Total`, and a working factory adds its own lack to `Total`. The
  research centre slows once its batteries fall under 6%; with a surplus the
  fraction is 1 and building costs research nothing.
- **A mine ≈ 11%** — *read*, and exact. The ore bar divides held ore by 4500,
  one full mine plus one full storage, and a full mine holds 500: 11.1%.
- **One power plant ≈ 33%** — *read*, and confirmed on The Convoy. The energy
  bar is the clan's net power over the power of every clan on the map, and
  that map has three equal generators, one of them the player's. On a map
  with two it would read 50%, with one 100%. The earlier guess here, that it
  came from a component averaging three inputs, is withdrawn.
- **A cap on bots, after which factories stop** — *read*, and exact. It is the
  clan's mind count from the mission, 2 to 17, shown as "Available CPUs"; a bot
  alive or under construction holds one, and losing a bot gives it back.

## Not established

- Whether a clan's AI re-orders a build that was refused for want of a mind
  (`ai.dll:0x10007fd0` is where to look).
