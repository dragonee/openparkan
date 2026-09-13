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

## Ore has to be carried — *measured*, and *read*

The distribution step (below) draws ore out of a holder at its
`Transfer_Ore_OnBoard` rate, capped by what it contains — *read*, at
`Behavior.dll:0x1001a8c6`. That rate is **0 on a mine and 20 on a storage**
(*measured*). So a mine fills itself and feeds nobody: its ore reaches a
factory or an institute only once a transport has moved it into a storage.
The transport carries 2000 and loads and unloads at 100 a second.

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

## Against what the game looked like

The HUD shows ore and power as percentages, a mine gives about 11%, a
transport turret carries ore from the mine to a warehouse, one power plant
holds about 33%, a factory builds far slower when both are low, and
construction slows research.

- **Transport from mine to warehouse** — the rates are *measured* and their
  use is *read*: ore is drawn from a holder at its on-board rate, which is 0
  on a mine and 20 on a storage.
- **Slower when low** — *read*, for research. The draw rates scale with
  efficiency and the task tracks its scarcest budget. The factory's
  construction task has not been read, so the factory itself is *unknown*.
- **Construction slowing research** — *guess*. The factory draws 4 power and 5
  ore from the same pool the institute draws 3 and 1 from, so while it builds
  the institute's tier gets a smaller share. Which tier each sits in is
  unread, so how much smaller is too.
- **One plant ≈ 33%** — *guess*. A type-10 component's efficiency is a mean of
  three, so one input of three fully met is exactly a third. Nothing yet ties
  that to the HUD.
- **A mine ≈ 11%** — *unknown*. The HUD's formula is not read.

## Not established

- **The variable ids.** The code reads profile variables by id — `0x1005` for
  what a generator gives, `0x1002` for an ore holder's rate, `0x1001` for its
  contents — and reading them as `0x1000 + position among the floats` makes
  every use sensible: a generator's only non-zero power field is
  `Transfer_Power_Out`, and the mining code takes the smaller of one side's
  rate and the other's contents. The loader that assigns them has not been
  read.
- The HUD's ore and power percentages, and the factory's construction task.
- `fPriority`, and properties `0x300`–`0x305`.
