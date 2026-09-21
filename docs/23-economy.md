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
transport fills from the mine at 100 a second. A mine that has dug its 500 is
not emptied by a draw at all: its task writes its running total back over
what it holds
([below](#mission-03s-economy-tick-by-tick--derived)). A transport picks the
nearest mine with a free loading place and the nearest storage, leaves a mine
part-loaded only while that mine has not yet dug its 500, and waits beside a
full storage ([32-builder.md](32-builder.md#transporting-ore--read-and-measured)).

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
that was used. The tick passes the level function a mask of channels each time
(`0x1002d3c1`–`0x1002d439`, masks `0x1003cdbc`): 8, 1, **4 | 32** — channels 2
and 5 at one shared level — 16, and 2 for the batteries with what was used. A
group that wants nothing is passed over and its consumers' levels are not
written (`0x1002dcd7`). Each component's flow is summed into its channel
(`0x1002d3a0`, slot 9 of `+0xc`), and every positive flow, a battery's, into
the supply. The class numbers are the engine's `CICLS_` ids (*measured*:
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
| gun (2, 30) | `power` plus whatever its capacitor lacks of value 1; its level is the capacitor's fill (`0x10029a40`, `0x10029a90`). **`power` is 0 on all 158 shipped gun and builder components** (*measured*), so the draw is the lack alone and a full tick fills the capacitor outright. A shot takes value 2 out of the capacitor, and a gun whose value 1 is 0 fires without power and never gets a level at all ([29-weapons.md](29-weapons.md#what-refills-the-capacitor--read-and-measured)) |
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
  new bot claims an entry by its id (`0x1001e0e0`, "Attached to Brain"): the
  factory makes it with create flag 8 (`0x1002a9f4`), and the creator claims a
  mind for any scheme whose Type has no top bit, with that flag (`0x1001d459`–
  `0x1001d46a`).
- **Every robot a mission places holds one** — *read*, and *measured*.
  `iron3d.dll`'s mission loader (`0x100a3ea0`) sorts the objects by kind, and
  places the units' list (kind 1, `0x100a3f9a`) at `0x100a458c` through
  `0x10077480`. That hands `ArealMap.dll`'s `CreateObjectFromScheme` create
  flag 8 unless the game is the auto-demo (`+0xe5`, `0x100774c7`–`0x100774d1`).
  The export carries the factory's creator code: for a non-building with flag 8
  it takes the clan's first free entry (`ArealMap.dll:0x100152a9`–`0x10015307`)
  and writes the new object's id into it ("Attached to Brain",
  `0x10015f0e`–`0x10015f24`). **With none free the unit is not made at all**
  (`0x100152f6` → `0x10015590` returns 0). Buildings go through `0x10033cb0`,
  whose flags are 1 and 4, never 8. *Measured*: all 296 `UNITS\UNITS` objects
  in the install's missions are kind 1, the 37 heroes among them, and no other
  object is. **So the hero holds a mind**, and Mission 02's Plr, 2 minds and the
  hero alone, starts with one free
  ([36-factory.md](36-factory.md#mission-02--measured-derived-and-seen)).
- **A unit the hero's Enter captures holds none** — *read*. The Enter case calls
  the behaviour's slot 39, `Capture` (`iron3d.dll:0x1007202a`), which for a unit
  changes its clan, SuperAI and areal map and writes no mind
  (`Behavior.dll:0x10009051`–`0x100090a0`). So of Mission 04's three — the hero
  and `tut4_f1` placed, the HQ taken by Enter — the HQ is the one that holds
  none, and 3 minds leave the recording's one free
  ([34-progression.md](34-progression.md#mission-04-teleport-end-to-end--derived)).
- **Who else writes an id into a mind** — *read*. A search of every fetch of a
  mind entry (SuperAI slot 17, and `ai.dll`'s own element accessor `0x10006e40`)
  in `Behavior.dll`, `ArealMap.dll`, `ai.dll` and `iron3d.dll` finds one more:
  `MBehaviour::ReloadSuperAI` (`Behavior.dll:0x10008c70`), when a network mirror
  becomes a behaviour again. With none free that is the "Behaviour panic"
  (`0x1005e0d8`: *"SuperAI have no free brains of clan %d (Switch from Mirror to
  Behaviour)"*). The other writes store 0, a reservation, or −1, a free entry,
  and the save loader restores each entry as saved
  (`iron3d.dll:0x100a3156`). The search finds the factory's own claim
  (`0x1001e0e0`) and reservation (`0x1002a348`), the two known before it.
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
- **When the refusal is seen** — *read*. An order to the end of a factory's
  queue starts at once only if it is then the only task (`0x100178c0`); an
  order to the start, or replacing, always starts at once. A start that fails
  makes `MBehaviour::MakeNewOrder` (`0x10004280`) take its "Incorrect Order
  parameter" way out and return 0, and `AddOrder` (`0x10004a90`) returns that.
  An order queued behind another returns 1 and fails later, when it reaches the
  top. The player's factory panel orders by replacing (`iron3d.dll:0x10087181`).
- **Where the AI's builds come from** — *measured*. All 9 `ORDER_BUILDING_CONSTRUCT`
  orders in the 58 scripts sit in a `PBM_ROBOT_NEEDED_Start` handler, one per
  script. Each gives the order to the end of a factory's queue. It sets
  `fn8(ST_SOLVED)` behind a comparison of `fn15`'s result with 1 (`op5`, 8 of
  them) or with 0 (`op1`, `c1m3e`). The two opcodes are the relation table's
  own — `op5` is `!=` and `op1` is `==` (`ai.dll:0x1001211a`–`0x1001218b`,
  [15-behaviour.md](15-behaviour.md#values-one-type-into-another--read)) — so
  both guards read the same way: **the problem is marked solved when the order
  was not taken**.
- **What `fn15` answers** — *read*. Its handler (`ai.dll:0x10008054`, the
  table's fifteenth slot) gives the order through the unit's `AddOrder` and
  leaves 1 in the interpreter's result (`+0x50`) when `AddOrder` returns
  non-zero, 0 when it returns 0, and 5 when no object answers the id
  (`0x10008376`).
- **A build the AI cannot place is dropped too** — *read*, and *measured*, and
  the opposite of what this page derived before. The executor writes a call's
  result slot into the call's destination (`0x100122e5`), so `dT3` is `fn15`'s
  answer; the guard above then reads *refused* and the handler does
  `fn8(ST_SOLVED)` and returns. **All 9** of them do: a build the factory would
  not take **ends the problem as solved**, and nothing in the handler orders
  again. The earlier reading here — that the problem stays unsolved and is
  re-ordered on the next plan — had the comparison's polarity backwards.
- **8 of the 9 do not even try without a mind** — *measured*. `Problems0`
  opens with `dFreeMindNumber = fn49()` in **all 9** scripts, at node 0, and
  eight of the nine `PBM_ROBOT_NEEDED_Start` handlers open with
  `if dFreeMindNumber <= 0` → `fn8(ST_SOLVED)`, `return`. Only `c1m3e` orders
  without looking. So the usual path is not a refusal at all: the clan drops
  the problem before the factory ever hears of it.
- **What re-raises it is the want, not the build.** The 58 scripts raise
  `PBM_ROBOT_NEEDED` 108 times, from 13 scripts and 13 handler names that each
  want a robot for something — `PBM_BUILDING_CAPTURE_Start`,
  `PBM_ATTACK_UNIT_Start`, `All_Defence`, `PBM_N_OPTIMAL_TRANSPORT_Start`,
  `Problems0` itself. Because the refused problem was marked **solved** rather
  than left standing, function 2's duplicate test (`0x10004c50`, same code,
  same *p1* and *p2*) no longer blocks it, and the next want raises it afresh.
  So the answer to *retry, drop or queue* is **drop** at every level: the
  factory drops the task, the handler drops the problem, and the clan comes
  back only when something wants a robot again. `ai.dll:0x10007fd0`, named here
  before, is only a helper in the handlers' code that evaluates one argument.

So a clan with 5 minds can have at most 5 bots alive or under construction,
and its factories stop until one is lost. Buildings take no mind.

*Measured:* across all 101 shipped clans no clan is placed with more robots
than its minds, two sit exactly at the limit, and counting every owned object
instead, 18 would exceed it. A robot placed past the count would not be made
(*derived*, from the placement's claim above).

## Construction — *read*

The player gives the order from the factory screen, and the bot leaves by the
factory's creation vertex: [36-factory.md](36-factory.md#production--read).

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
- The numbers do not jump: each displayed value steps by one point toward its
  target when **more than** 0.05 s has passed since the last step
  (`iron3d.dll:0x1006da2f`, `fcomp` against the float 0.05 at `0x100e50a4`),
  tested once a drawn frame. At 60 frames a second three frames make 50 ms,
  which does not pass, so a step comes every fourth frame: 15 points a second
  (*derived*). A bar whose value is 0 flashes on a half-second timer.
  *Seen*, on *The Field Base* as command mode opens: Energy reads 5% at
  178.5 s, 28% at 180, 50% at 181.5, 74% at 183 and 97% at 184.5, 15 points a
  second.

## Mission 03's economy, tick by tick — *derived*

*The Field Base* (`CAMPAIGN.00/Mission.03`) is where the game teaches the
economy. The hero captures a Small Generator and a Small Bunker, a builder puts
a Small Mine on the lode, a transport carries its ore to the Small Storage, and
the Large Factory builds warbots from it. This section puts the pieces above
together into one model and checks it against a 960 × 720 recording of the
mission. The model predicts every build in the recording to within a second.
The mission's script and timeline are
[34-progression.md](34-progression.md)'s; placing and building the mine is
[32-builder.md](32-builder.md)'s.

### What the mission gives — *measured*

| object | Type | clan | profile (by Type) | efficiency | ore |
|---|---|---|---|---:|---|
| `gener01.dat`, Small Generator (`fr_l_gener`) | `0x80000002` | Ntrl | `prof_generator`: power out **10** | 1 | — |
| `sbunk01.dat`, Small Bunker (`fr_l_bunker`) | `0x80010000`, a generator Type | Ntrl | `prof_bunker`: power out **0.07**, use 1 | 1 | — |
| `sstore01.dat`, Small Storage (`fr_l_store`) | `0x80000008` | Plr | `prof_storage`: off-board **1**, use 1 | 1 | holds 0 of 4,000 |
| `lplant01.dat`, Large Factory (`fr_b_plant`) | `0x80000010` | Plr | `prof_plant`: use **4** | **5** | `FreeBotNum` 0 |
| `smine01.dat`, Small Core mine (`fr_l_mine`), the Mine scheme's first | `0x80000004` | built | `prof_mine`: off-board **1**, use **1** | 1 | 500 |
| `tut3_t.dat`, transport | `0x1002000` | Plr | | | holds 0 of 2,000 |
| `tut3_b.dat`, builder | `0x1004000` | Plr | | | holds 200 of 2,000 |

- **The profile is picked by Type** (`Behavior.dll:0x10008a80`): a mine
  `prof_mine`, a storage `prof_storage`, a plant `prof_plant`, a generator
  `prof_generator`, and the three bunker Types `prof_bunker`.
- **The one lode** lies at (1026.1, 942.7). It starts found, and its amount is
  1e19.
- **The player clan has 7 minds.**
- **Nothing is free.** The factory's `FreeBotNum` is 0, so every bot is paid
  for.
- **The `prebuild` object** in `mission.cfg` names `tut3_p1.dat` and
  `tut3_p2.dat` from `UNITS\UNITS\PREBLD`. Both are priced from `tut3_pl.trf`,
  and every part of each is researched:

  | design | name | unit box | ore | power |
  |---|---|---|---:|---:|
  | `tut3_p1.dat` | SSW-X Warrior (`R_L_01`, two `e_gun_lc_01`) | 7 / 0 t, 43 kph, 8 %, 18 %, 300 m | 125.6 | 72.7 |
  | `tut3_p2.dat` | SWW-X Warrior (`R_L_03`, two `e_gun_ll_05`) | 6 / 2 t, 64 kph, 7 %, 8 %, 300 m | 118.6 | 63.7 |

- **The transport and the builder** are both on `R_L_03`. Their top speeds
  come out at 24.0 m/s and 22.8 m/s, and the second is the "82 kph" the
  recording's Builders page shows.

### What `prebuild` does — *read*, and *seen*

`iron3d.dll` reads the mission's `prebuild` object (`0x1004ddc9`) and loads each
of its values into the warbot designer's recent projects, the five 0x7c-byte
slots from `+0xb674` that [36-factory.md](36-factory.md#projects--read-and-measured)
reads. For each value:
1. the slots shift down by one (`0x1004ded7`, four moves);
2. slot 0 is cleared;
3. `units\units\prebld\` and the value are loaded into slot 0
   (`0x1004df52`, `0x10056c10` at `0x1004df74`);
4. the filled slots are counted again.

So the last model named is project 0. *Seen*:
- at 215.5 s the factory page shows *SWW-X Warrior*, `model2`, with two recent
  projects;
- at 217 s the player has picked *SSW-X Warrior*, and batch production starts
  it.

### A mine digs to 500, and then a draw does not empty it — *read*

**The order.** `iron3d.dll` keeps a record per building. Its setup
(`0x10032d30`, slot 3 of the record's vtable `0x100e5c4c`) gives every mine
(`0x80000004`) `ORDER_BUILDING_MINE` (10), replacing (`0x10032de3`,
`0x10032e42`). The same setup files a player's plant for the factory panel and
a player's institute for the research panel.

**The target** (`M_Task_Mine::SetTarget`, `Behavior.dll:0x1002cd10`):
- It adds up, as `ToMine`, the amounts of every lode within **250 across the
  ground** of the mine. The distance is `0x10020f70`, a length of x and y only.
- It marks each of those lodes found.
- It refuses the order when the total is 0.

**The start** (slot 6, `0x1002cef0`):
- it stamps the clock;
- it sets the progress to 0;
- it calls `SetPowerUsage(Use_Power)`, so a small mine draws 1 a second from
  then on.

**Each takt** (`0x1002cf60`), with `dt` the time since the last in seconds:
1. **Nothing left ends it.** A takt that finds `ToMine` at or below zero
   returns 0 at once and the task is over, writing nothing (`0x1002cf68`).
2. **Efficiency.** `KPD` is the mine's efficiency, or 1 when its profile uses
   no power.
3. **Below 0.1** the task logs `BAD KPD`. It sets its running total, its
   progress **and the mine's held ore** to 0 (`0x1002cff6`, then the setter
   at `0x1002d010`).
4. **Otherwise it digs** `dt × Mine_OrePerSecond × KPD` (`0x1002d06c`).
   - The dig is capped so that the running total does not pass the ore
     property's maximum, 500 (`0x1002d09e`).
   - It takes `ToMine` less the total **before either changes**, then adds the
     dig to the total and takes the same dig off `ToMine` (`0x1002d0bd`–
     `0x1002d0df`). So a takt moves the two **towards each other**, one dig
     each.
   - `ToMine` is written into `MBehaviour+0x9e8`.
   - **The mine's held ore is set to the running total** (`0x1002d0f9`).
   - The progress is the total over `ToMine`, both as they now stand.
5. **When the amount it took in step 4 — `ToMine` less the total, as they stood
   before the dig — is no more than that dig**, it logs "All Ore mined...".
   The last dig is banked all the same: the total, the held ore and `ToMine`
   are written first, then the progress goes to 1, `SetPowerUsage(0)` is called
   and the task ends (`0x1002d142`).

**So `ToMine` bounds the output and never scales it** (*read*). The rate is
`Mine_OrePerSecond × KPD` whatever the lodes hold; what the lodes decide is
only when the task stops. And because a takt moves the total up and `ToMine`
down by the same dig, the two meet **half way**: a mine digs about `ToMine / 2`
before it hears "All Ore mined", or 500, whichever comes first. A lode of 600
yields 350, not 600. Only the ore actually dug comes off `ToMine`.

*Measured* over all 29 missions: **15 placed mines in 8 missions, and every one
has exactly one lode within 250** — so its `ToMine` is that lode's amount
alone, from 999,999 up to 10²⁰. **0 of the 15 carry less than 1000**, so the
halving never binds on shipped data: every shipped mine digs its full 500 and
then stops digging, with `ToMine` down by exactly 500 and the task still
running. Control: of the other buildings in the missions that carry lodes, 6 of
95 stand within 250 of one
([31-packages.md](31-packages.md#mineral-lodes--read-and-measured)).

The running total only grows, and every takt writes it over the mine's held
ore. Once a mine has dug its 500:
- **A draw is put back on its next takt.** The distribution step lowers the
  held ore (`0x1001a918`), and so does a transport loading (`0x10032144`).
  Neither touches the task's total.
- **A full mine never runs dry**, and a transport loads a mine that holds
  500 up to its own 2,000 (*derived*).
- **A lode gives up only the 500 dug**, so none of the shipped lodes,
  10⁴ to 10²⁰, is ever exhausted in play (*derived*).
- *Seen*:
  - both of the recording's transport loads are 2,000 (below);
  - the Ore bar holds at 11% through the loading, with no dip.

**Which order runs first on a built mine** (*derived*). `CreateObjectFromScheme`
gives a new building order 18 **to the start** of its queue
(`Behavior.dll:0x1001e02c`, `push 2`), and an order to the start begins at once.
The mine's order 10 must already be queued behind it, because:
- the construction sphere plays out;
- the mine digs as soon as it ends;
- the mine draws its 1 a second from the moment it appears.

The recording shows all three: Energy falls from 100% to 89% by 196 s, the mine
starts digging about 41 s later, and the sphere lasts 41 s.

### Ore reaches the factory at one a second from each holder — *read*

**Each distribution step** (`0x10019e80`, every 192–255 ms, with `dt` in seconds):

- **A mine or a storage offers** `min(held, dt × KPD × Transfer_Ore_OffBoard)`
  (`0x1001a8c6`). That is **1 a second** on a small mine and on a small storage.
- **Every other building asks** for its ore property's maximum less what it
  now holds (`0x1001a1af`), or nothing when that is under 0.01.
- **The ore is shared out** as the step's two-tier formula above.

**The construction task** keeps the factory's side of it
(`M_Task_Construct::OnBehaviourTakt`, `0x1002a4f0`). Each takt:
1. **It works out its request**, `(cost − collected) × k × KPD × 0.2 + 0.07`.
2. **It takes what arrived.** `0x10015540` moves out of the factory's ore
   property as much of the request as it holds, and the task adds that to its
   collected ore.
3. **It asks again.** The property is set to 0 held, with the request as its
   maximum (`0x1002a604`), so the factory's want in the next step is exactly
   the request.

A request is at least 0.07 a takt and, early in a build, about 5. So in Mission
03 the supply sets the pace (*derived*):
- **1 ore a second** while only the mine holds ore;
- **2 ore a second** once the storage has some.

The warehouse doubles the flow as well as buffering it, which is T03_H01's
*"your Factory's production … will be limited by the Mine's parameters"*.

### A transport's round — *read*, and *seen*

The rules are [32-builder.md](32-builder.md#transporting-ore--read-and-measured)'s.
With the mine full as above:

| step | time |
|---|---|
| loading, 100 a second into 2,000 of room | 20 s |
| the walk to the storage: the lode and the storage are 429 m apart, the transport's top speed is 24 m/s | at least 18 s |
| unloading, 100 a second while the storage has room | 20 s |
| the walk back | at least 18 s |

*Seen*, on the Ore bar (held ore over 4,500):
- **From 303.5 to 323.5 s** it rises from 11% to 55%, 2.2 points a second:
  2,000 into the empty storage.
- **From 395 to 415 s** it rises from 55% to 99%: 2,000 more, filling the
  storage.
- **The round** is 91.5 s, 13 s more than the two walks at full speed, the
  loading and the unloading add up to.

### Power — *read*, and *measured*

- **Supply.** A generator gives `Transfer_Power_Out × dt` to its clan. After
  both captures that is **10.07 a second**: the generator's 10 and the bunker's
  0.07. No other clan in the mission has power.
- **Draw.** A building's efficiency component takes `(0.01 + usage)` a second
  from its batteries, and the distribution step tops them up again.
- **Usage is set only by a task at work.** `SetPowerUsage` (`0x10019880`) is
  called by construction (`0x1002a308` at the start, `0x1002a722` to 0 once the
  power is in), by mining (`0x1002cf49`, `0x1002d17e`) and by research. So in
  Mission 03:
  - an idle building draws 0.01;
  - the mine draws 1.01 from its order's start;
  - the factory draws 4.01 while a build collects its power.
- **What the Energy row shows** is `(available − demanded) / available`
  ([What the HUD shows](#what-the-hud-shows--read)). With the batteries full,
  the demand is what they lost since the last step. The bunker is a generator
  Type and asks for nothing. So the row reads (*derived*):
  - **over 99%** with nothing at work: the storage, the factory and the mine
    draw 0.01 each;
  - **89.8%** with the mine at work, (10.07 − 1.03) ÷ 10.07;
  - **50.0%** while the factory also collects a build's power,
    (10.07 − 5.03) ÷ 10.07.
- *Seen*:
  - 100% from 184 to 194 s;
  - 87–92% from 196 s, 88–89% most of the time;
  - four dips to 47–55%, each lasting 3.3–4 s (below).

### A warbot from the Large Factory — *read*, and *measured*

- **Cost.** A build of *SSW-X Warrior* is priced as 125.6 ore and 72.7 power
  ([38-designs.md](38-designs.md#the-price--read)). The ore is divided by the
  Large Factory's efficiency of 5, giving **25.12**. The time budget is 5 s.
- **Power** comes in at `KPD × Use_Power` = 20 a second, so it is collected
  in **3.6 s** and then stops.
- **Ore** sets the pace:
  - **25.1 s** with the mine alone;
  - **12.6 s** once the storage holds ore.

### Against the recording — *seen*

The recording is sampled every 2 s on the resource rows and every 5 s on the
panel. Each build's power collection shows as an Energy dip toward 50%, so the
dips time the builds. The first build starts when batch is clicked. Each later
one starts as the bot before it is finished, because batch production gives the
file again on the frame the factory is idle
([36-factory.md](36-factory.md#production--read)).

| build | its power drawn (dip) | its bot done | predicted done | the bot seen |
|---|---|---|---|---|
| 1 | 217.8–221.4 s | 260.7 s | mine digging from 235.3 s, + 25.1 s = 260.4 s | *SSW-4 Warrior [escaping]* at 261 s |
| 2 | 260.7–264.4 s | 286.6 s | 285.8 s | *SSW-5* at 291 s |
| 3 | 286.6–290.2 s | 308.3 s | 16.9 ore from the mine by 303.5 s, the rest at 2 a second: 307.6 s | *SSW-6* at 311 s |
| 4 | 308.3–311.6 s | 320.7 s | 320.9 s | *SSW-7* at 321 s |
| 5 | 320.7–324.7 s | — | 333.3 s | *SSW-8* at 341 s; no further dip |

- **The mine**:
  - the Ore bar reads 3% at 238 s, 5% at 240 s, 7% at 242 s, 9% at 244 s and
    11% at 246 s: digging at 50 a second, from about 235.3 s;
  - 41 s of construction sphere before that puts the mine's creation at about
    194 s, which is when the Energy row drops to 89%;
  - it stays at 11% (500) from 246 s until the first unload.
- **Build 1's power** is in by 221.4 s, but the build waits on ore until the
  mine has dug.
- **The Ore row** is up from 178.5 s, reading 0% in red with its figure
  blinking. It stays at 0% until the mine digs: the storage is empty and no
  mine exists.

### For an engine

Tick the economy on its own timers, per clan:

1. **Buildings.** Each building has its profile by Type, its efficiency (the
   class-26 value), its held ore and its ore maximum.
   - A mine's maximum is 500, a storage's 4,000, a transport's and a builder's
     2,000.
   - A consumer's maximum is its task's request.
2. **A mine** is given order 10 as it joins; a built one runs it once its
   construction sphere ends. While the order runs, the mine:
   - digs `dt × 50 × KPD` a takt into a running total capped at 500;
   - sets its held ore to that total;
   - draws 1 power a second while its order runs;
   - with `KPD` below 0.1, zeroes both its total and its held ore;
   - with no lode within 250 across the ground, refuses the order.
3. **Every 192–255 ms**, per clan:
   - every mine and storage offers `min(held, dt × KPD × 1)`;
   - every consumer asks for maximum − held;
   - both are shared by `min(1, offered / asked)`, and each holder gives its
     share.
4. **A build** (paid; `FreeBotNum` 0):
   - its ore cost is the design's price over the factory's efficiency;
   - power comes in at `KPD × Use_Power × dt`, 20 a second in the Large Factory,
     and the draw stops once it is collected;
   - its ore request each takt is `(cost − collected) × k × KPD × 0.2 + 0.07`;
   - it takes what arrived since;
   - it is done when time (5 s), ore and power are all in, and its progress is
     the smallest of the three.
5. **A transport**:
   - loads at 100 a second up to 2,000 (a full mine gives it all);
   - walks to the storage and unloads at 100 a second while there is room;
   - walks back, and repeats.
6. **Energy** = (the player clan's power out − its batteries' losses since the
   last step) over every clan's power out.
   - Every building draws 0.01 a second.
   - A working mine draws 1 more, a factory collecting a build's power 4 more.
7. **The rows** show ore held in mines and storages over 4,500, and Energy,
   each stepping one point toward its target every fourth frame at 60 frames
   a second.
8. **`prebuild`** pushes each named design into the factory's recent projects,
   the last named first.

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

- ~~Whether a clan's AI re-orders a build that was refused for want of a
  mind.~~ It does not: **it drops it**, and the guess this line rested on was
  backwards. `op5` is `!=`, not equality, so all 9 handlers mark the problem
  `ST_SOLVED` when `fn15` does not answer 1; and 8 of the 9 never order at all
  without a free mind, because they open on `if dFreeMindNumber <= 0`. The
  problem comes back only when one of the 108 raises in the corpus wants a
  robot again ([The bot
  limit](#the-bot-limit-is-the-clans-mind-count--read-and-measured)).
- **Which of Mission 03's units hold a mind, and what the factory's "Available
  CPUs" counts** as a build runs. ~~Which hold one~~ — **read**: all three
  placed robots, the hero among them
  ([above](#the-bot-limit-is-the-clans-mind-count--read-and-measured)), so 4 of
  the 7 are free before any build, as seen. Still open: the 4 from 221.5 s, when
  the finished bot should hold the mind its build reserved, and the five
  warbots built on four free minds.
  - The player clan has 7 minds and starts with the hero, a builder and a
    transport.
  - *Seen*, the figure reads:
    - 4 at 215.5 s, before any build;
    - 3 from 218 to 220.5 s, while the first build collects its power;
    - 4 from 221.5 s;
    - 3 at 262 s, during the second build's power.
  - Five warbots are built (*SSW-4* to *SSW-8*), and no sixth build starts.
- **The ore an ore place moves by itself.** A transport or builder standing in a
  mine's loading place or a storage's unloading place, with its property `0x208`
  at 0, also exchanges ore through the building's place tick (`0x10019482`,
  `0x100195b8`, through `0x100155f0`), at the smaller of the building's and the
  unit's rate. Not established here:
  - which way the ore goes;
  - what `0x208` is;
  - what the tick's divisor is.
  The model above leaves it out; beside the task's 100 a second it is at most a
  few ore a second.
- **The 13 s a recorded transport round takes** beyond two walks at full speed:
  its held speed on Tut_3's slopes and the distance between its two places are
  not measured.
