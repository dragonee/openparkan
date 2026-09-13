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

## How ore reaches a consumer — *open*, and a correction

An earlier version of this section said **a mine feeds nobody** until a
transport carries its ore to a storage. **That was wrong, or at least not
established, and the game says so itself.** The tutorial text in
`TextRes.dll`:

> string 103 — "if you don't have a Warehouse, your Factory's production, for
> instance, will be limited by the Mine's parameters"
>
> string 101 — "the Mine works kinda slow, and if your base doesn't have a
> Warehouse, you'll always have to wait to accumulate enough raw materials"

So a mine does supply consumers directly, slowly; a warehouse stores ore and
"teletransports" it to them; and a transport ships ore from mine to warehouse
(string 104).

What the claim rested on, and which link is suspect:

- The distribution step draws ore from a holder at the variable it reads as
  id `0x1002`, capped by the holder's contents — *read*, at
  `Behavior.dll:0x1001a8c6`.
- Taking ids as `0x1000 +` position among a profile's floats, `0x1002` is
  `Transfer_Ore_OnBoard`, which is **0 in a mine's profile and 20 in a
  storage's** — *measured*.
- But a placed mine does not use its profile's capacity: missions give every
  mine 500, the engine's `Mine_MaxOre`, where the profile says 1000. The same is
  probably true of its rate, which would come from `Mine_OrePerSecond`. Either
  that or the id numbering is the link that breaks.

*Unknown:* how a mine's output reaches the pool without a warehouse.

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
- The numbers do not jump: every 0.05 s each displayed value steps by one
  point toward its target. A bar whose value is 0 flashes on a half-second
  timer.

## Against what the game looked like

The HUD shows ore and power as percentages, a mine gives about 11%, a
transport turret carries ore from the mine to a warehouse, one power plant
holds about 33%, a factory builds far slower when both are low, and
construction slows research.

- **Transport from mine to warehouse** — confirmed by the game's own tutorial
  (strings 103, 104). A mine also supplies consumers directly, more slowly;
  how is *unknown* — see the correction above.
- **Slower when low** — *read*, for both the factory and research. Power and
  ore accrue in proportion to efficiency, and progress is the smallest of the
  three completion fractions, so whichever resource is short sets the pace.
- **Construction slowing research** — *guess*. The factory draws 4 power and 5
  ore from the same pool the institute draws 3 and 1 from, so while it builds
  the institute's tier gets a smaller share. Which tier each sits in is
  unread, so how much smaller is too.
- **A mine ≈ 11%** — *read*, and exact. The ore bar divides held ore by 4500,
  one full mine plus one full storage, and a full mine holds 500: 11.1%.
- **One power plant ≈ 33%** — *read*, and conditional. The energy bar is the
  clan's net power over the power of every clan on the map, so one generator
  reads a third when three equal generators exist and nothing is left to
  recharge. The earlier guess here, that it came from a component averaging
  three inputs, is withdrawn.

## Not established

- **The variable ids.** The code reads profile variables by id — `0x1005` for
  what a generator gives, `0x1002` for an ore holder's rate, `0x1001` for its
  contents — and reading them as `0x1000 + position among the floats` makes
  every use sensible: a generator's only non-zero power field is
  `Transfer_Power_Out`, and the mining code takes the smaller of one side's
  rate and the other's contents. The loader that assigns them has not been
  read.
- That the clan record's slot `0x2c`, which the HUD calls before the stats
  getter, returns the distributor. The getter it then reaches returns exactly
  the distributor's totals block, which is why the reading holds; the call
  itself was not followed into `ai.dll`, which creates the distributors.
- `fPriority`, and properties `0x300`–`0x305`.
