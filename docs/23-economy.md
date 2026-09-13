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
500 and idle.

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
  costs in the `.trf`, and a time budget taken from the clan — and the task
  computes **the smallest of its three completion fractions**.
- **Free technologies cost nothing.** While the clan's free-technology counter
  is above zero, starting a research decrements it and sets the ore and power
  budgets to zero. This is the mission's `FreeTechnoNum` property.
- **It draws `KPD × Use_Ore × dt` ore and `KPD × Use_Power × dt` power** each
  tick, the ore capped by the institute's own buffer, and then requests the ore
  still missing — half of it while more than 5 remains. `KPD` is logged by that
  name: Russian *КПД*, efficiency.
- **`KPD` is the building's efficiency**, below. `0x100198e0` totals property
  `0xe00` over one list of the building's components; `0x10019880`, the
  `SetPowerUsage` call, writes property `0x500` on each of them.

*Unknown:* how the time budget accrues.

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
| `0x100` | a value the owner reports for the component; slot 2 treats 0 as destroyed, so it is its condition — a *guess* |
| `0x200` | a level held at `+0x4c`, which starts at 1 and can be set only while the component's state at `+0x50` is non-zero and it is not destroyed |

So `0x300`–`0x305` are **the first six floats of the record, scaled by
condition and level**, and an intact building's `KPD` is its class-26
component's first float. What sets the level is unread.

**The data** — *measured*. Class 26 occurs only in `fortif.rlb`, once per
building controller (five times on the small main teleport), always with
fifteen zeros after the first value. Its first value follows the model number
in the building's name:

| | small, -17 | medium, -30 | large, -47 | enhanced, -67 |
|---|---|---|---|---|
| research centre | 1 | 3 | 5 | 7 |
| factory | 1 | 3 | 5 | — |
| core mine | 1 | 3 | 5 | — |

and every other building — storage, generator, tower, bunker, hangar, bridge,
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

## Construction — *read*

`M_Task_Construct::OnBehaviourTakt` (`Behavior.dll:0x1002a4f0`) is the factory
building a bot, and it has the same three budgets as research — ore, power and
time — with one difference in how ore is pulled.

- **Time** accrues as plain `dt`; **power** as `KPD × Use_Power × dt`.
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
- It **completes when all three are collected**, then decrements a clan counter
  at `+0x9ec` — the free-bot count, a *guess* from the mission's `FreeBotNum`.
- Until then its **progress is `min(time, ore, power)` as fractions, capped at
  1**, logged as "Construction in progress".

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
- **Construction slowing research** — *read*, when the clan is short. Every
  consumer gets the same fraction of what it lacks, `Available / Total`, and a
  building factory adds its own want to `Total`. With a surplus the fraction
  is already 1 and building costs research nothing.
- **A mine ≈ 11%** — *read*, and exact. The ore bar divides held ore by 4500,
  one full mine plus one full storage, and a full mine holds 500: 11.1%.
- **One power plant ≈ 33%** — *read*, and confirmed on The Convoy. The energy
  bar is the clan's net power over the power of every clan on the map, and
  that map has three equal generators, one of them the player's. On a map
  with two it would read 50%, with one 100%. The earlier guess here, that it
  came from a component averaging three inputs, is withdrawn.

## Not established

- What sets a component's level, the `0x200` factor — and so whether a short
  power supply lowers `KPD` itself.
- How research's time budget accrues.
