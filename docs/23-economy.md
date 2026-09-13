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

## Power is shared in two tiers — *read*

`Behavior.dll:0x10019e80` runs once per clan per distribution step and logs
every quantity it uses. Generators are the building types `0x80000002`,
`0x80010000`, `0x80020000` and `0x80040000`, and each **gives
`Transfer_Power_Out × dt`**. Mines and storages are `0x80000004` and
`0x80000008`. Every other building keeps a charge and **asks for what it
lacks**: its need times one minus its current fill.

The step then totals supply (`fPowerAvailable`) against demand
(`fPowerUsage`), with demand split by a per-building priority (`fPriority`)
into a first tier and the rest, and fills them in order:

```
if Available ≤ FirstTier:       A = Available / FirstTier;  B = 0;  Take = 1
elif Available < Total:         A = 1;  B = (Available − FirstTier) / (Total − FirstTier);  Take = 1
else:                           A = 1;  B = 1;  Take = Total / Available
```

**`fPowerA`** is the share of first-tier demand met, **`fPowerB`** the share of
the rest, and **`fPowerTake`** the share of the generated power actually drawn —
below 1 only when there is a surplus, which is simply not taken. Each consumer's
charge is then topped up by what it lacked times its tier's share; a generator
is set straight to full. Ore goes through the identical formula as `fOreA`,
`fOreB` and `fOreTake`.

*Unknown:* what sets a building's `fPriority`, so which buildings are first
tier.

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
- **`KPD` is the sum of the institute's components' efficiencies.**
  `0x100198e0` totals property `0xe00` over the building's control components;
  `0x10019880`, the `SetPowerUsage` call, writes property `0x500` on each.

A component's efficiency is computed in `Control.dll:0x1002bb40` and depends on
its type: **for type 10 it is `(p₃₀₀ + p₃₀₁ + p₃₀₂) × 1/3`**, the mean of three
inputs, and type 21 sums five.

*Unknown:* what properties `0x300`–`0x305` are, and how the time budget
accrues.

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
- **Construction slowing research** — *guess*. The factory draws 4 power and 5
  ore from the same pool the institute draws 3 and 1 from, so while it builds
  the institute's tier gets a smaller share. Which tier each sits in is
  unread, so how much smaller is too.
- **A mine ≈ 11%** — *read*, and exact. The ore bar divides held ore by 4500,
  one full mine plus one full storage, and a full mine holds 500: 11.1%.
- **One power plant ≈ 33%** — *read*, and confirmed on The Convoy. The energy
  bar is the clan's net power over the power of every clan on the map, and
  that map has three equal generators, one of them the player's. On a map
  with two it would read 50%, with one 100%. The earlier guess here, that it
  came from a component averaging three inputs, is withdrawn.

## Not established

- `fPriority`, and properties `0x300`–`0x305`.
