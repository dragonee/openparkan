# Units — a whole robot on one sheet

What a unit assembly *is*, put together from the pages that read its pieces: the
chassis ([28-chassis.md](28-chassis.md)), the turret ([30-turrets.md](30-turrets.md)),
the internal parts ([25-sensors.md](25-sensors.md), [26-damage.md](26-damage.md),
[23-economy.md](23-economy.md)), the weapons and their clips
([29-weapons.md](29-weapons.md)), what moves it ([24-motion.md](24-motion.md)),
the orders it takes ([31-packages.md](31-packages.md)) and, for builders and
transports, what they carry ([32-builder.md](32-builder.md)).

`openparkan.units` builds the sheet (`Workshop.describe`) and prints it
(`render`); `openparkan unit <name>` does both, and `openparkan unit` alone lists
every assembly with its role, chassis, turret and firepower. Every figure on the
sheet is a part's own, read by the pages above; the few totals are marked
*derived*. `openparkan verify` describes all 458 assemblies and checks that the
pieces fit (`check_units`).

## How a unit is put together — *measured*

A `.dat` is a tree ([07-objects.md](07-objects.md#unitsdat--unit-and-building-assemblies)):

- **the chassis** at the root, a `r_<size>_<nn>` record whose controller
  declares six labelled slots — engine, battery, shield generator, detection
  shield, repair unit, armour — each taking one size of part;
- **the internal parts** under it, one per slot, `i_eng`, `i_pws`, `i_fsh`,
  `i_dsh`, `i_rps`, `i_arm`;
- **the turret**, `e_tur_<size><t|b>_<nn>`, the chassis's own size, `t` standing
  and `b` hung under a flyer. Its Type — the `.dat` class word — is the unit's
  role: warrior, transport, builder, HQ or hero;
- under the turret **a radar and a deflector** part, of the sizes its controller
  names, and **the guns** in its `Base_*` sockets, a builder's module on
  `Base_LU_01`;
- under each gun that takes clips, **one clip** of the family its slot names.

*Measured* over the shipped assemblies: 374 stand on a robot chassis and 372 of
them carry a turret (the two target dummies do not); all 865 fitted guns sit in
one of their turret's sockets and resolve to a round; all 588 clips belong to
their gun's slot; and all 2,956 internal parts read as their class.

## What the sheet shows

| line | what | from |
|---|---|---|
| role | Type, role and behaviour profile | the `.dat` class word; `Behavior.dll:0x10008a80` |
| chassis | name and code, locomotion, size; top speed, acceleration, payload and slope limit; body weight and node-0 hit points; build cost; the built-in battery and engine; the slots | `objects.dlb`, the chassis profile's `ChassisType`, the controller frame, `.ndp` density × slot volume |
| turret | name and code, mounting, HQ mark; sockets and how many are filled, built-in guns, hit points, the radar and deflector sizes it takes | the turret mesh's `Base_*` nodes, its controller and `.ndp` |
| each part | family, name, where it is fitted, and the numbers that say what it does: an engine's drive and draw, a battery's capacity and output, a shield's sector strength, recharge and price, a detection shield's cuts and camouflage, a repair unit's regeneration, armour's share kept and the hit that goes through whole, a radar's range, a deflector's share | the part's class values |
| each weapon | name, code, socket; energy, unlimited, or the fitted clip and its rounds; shots a second, damage a round, blast, guidance, range; damage and energy a second | `openparkan.weapons` |
| firepower | the damage a second of all its guns, the energy a second they ask, the longest reach — *derived*, before armour, shields and the level ratio | |
| cargo | 2,000 ore, loaded and unloaded at 100 a second, on builders and transports | `profiles.TRANSPORT_*` |
| orders, wingman | the packages each menu offers the unit, and whether it is too big to capture | `openparkan.packages.packages_for` |

What it leaves out, because it is not established: which of a turret's radar
slot and fitted radar the control system uses; whether a fitted repair unit's
values replace the chassis slot's; the magazine a fitted clip gives, which the
sheet takes to be the clip's round count (a *guess*,
[29-weapons.md](29-weapons.md#clips--measured-and-a-guess)); the whole unit's
weight and spare payload.

## An example

`openparkan unit w_b_trk1`:

```
Medium Track Chs (M-42t)  (w_b_trk1.dat)
  role      warrior, Type 0x1008000, profile prof_war.var
  chassis   Medium Track Chs M-42t (R_M_04), tracked, size m
            95 km/h, 30 m/s2, payload 36 t, brakes past 34 deg
            body 6,000 kg, 950 HP; build 15 E / 70 O
            built in: battery 10,000 at 250/s, engine 1 drawing 20/s
            slots engine m, battery m, shield generator m, detection shield m, repair unit m, armour m
  turret    Medium Battle Trt 3m1 (e_tur_mt_01), upright
            3 sockets, 3 filled, 720 HP; takes i_rdr_m, i_def_m
  armour            ARMOUR MA.Mk1 on the chassis: keeps 62% of a small hit; stops nothing from 2,688
  engine            Medium engine on the chassis: drive 0.9; draw 3.1/s at full speed
  battery           Medium Battery on the chassis: holds 12000; gives 15/s
  shield generator  Med Shld generator on the chassis: per sector 2500 x 6; recharge 30/s; costs 0.05 a point
  detection shield  Med detect.shld on the chassis: hides mass/electronics/drive 0.9/0.62/0.01; camouflage none
  repair unit       Med repair unit on the chassis: regenerates 30 HP/s; costs 0.05 a point
  radar             Medium sensor module on the turret: range 300 m; sensitivity 0.05/0.7/25
  deflector         Medium deflector on the turret: stops 85% of a sector
  weapon    Medium Howitzer M125How at Base_C_01: Med Howitzer HE clip, 100 rounds; 0.83/s, 400 a round, 2.5 m blast, guided, 350 m; 333 dmg/s, 0.33 E/s
  weapon    Medium Rocket Lr MRL16S at Base_LU_01: Rocket pack II, 32 rounds; 1.33/s, 225 a round, 2 m blast, 250 m; 300 dmg/s, 0.32 E/s
  weapon    Medium Rocket Lr MRL16S at Base_RU_01: Rocket pack II, 32 rounds; 1.33/s, 225 a round, 2 m blast, 250 m; 300 dmg/s, 0.32 E/s
  firepower 933 a second (derived), weapons ask 0.97 E/s, reach 350 m
  orders    Standby, Route, Seek and destroy, Guard, Refit; too big to capture
  wingman   Standby, Seek and destroy, Attack, Refit, Follow me

Large Track Chs (L-42t)  (w_b_trk1.dat)
  role      warrior, Type 0x1008000, profile prof_war.var
  chassis   Large Track Chs L-42t (R_B_04), tracked, size b
            90 km/h, 26 m/s2, payload 80 t, brakes past 34 deg
            body 20,000 kg, 4,500 HP; build 20 E / 120 O
            built in: battery 10,000 at 250/s, engine 1 drawing 20/s
            slots engine b, battery b, shield generator b, detection shield b, repair unit b, armour b
  turret    Large Battle Turret 4L1 (e_tur_bt_01), upright
            4 sockets, 4 filled, 3,000 HP; takes i_rdr_b, i_def_b
  armour            ARMOUR LA.Mk3 on the chassis: keeps 39% of a small hit; stops nothing from 2,644
  engine            Large engine on the chassis: drive 1; draw 11.2/s at full speed
  battery           Large Battery on the chassis: holds 31000; gives 34.5/s
  shield generator  Lrg Shld generator on the chassis: per sector 3350 x 6; recharge 50/s; costs 0.06 a point
  detection shield  Large detect.shld on the chassis: hides mass/electronics/drive 0.98/0.982/0.05; camouflage none
  repair unit       Lrg repair unit on the chassis: regenerates 50 HP/s; costs 0.06 a point
  radar             Large sensor module on the turret: range 400 m; sensitivity 0.05/0.7/25
  deflector         Large deflector on the turret: stops 85% of a sector
  weapon    Large Rocket Lr LRL9L at Base_LU_02: Rocket pack II, 18 rounds; 0.67/s, 1900 a round, 4 m blast, 450 m; 1267 dmg/s, 1.33 E/s
  weapon    Large Rocket Lr LRL9L at Base_RU_02: Rocket pack II, 18 rounds; 0.67/s, 1900 a round, 4 m blast, 450 m; 1267 dmg/s, 1.33 E/s
  weapon    Large Cannon L152mmC at Base_RU_01: Lrg Cannon AP clip, 60 rounds; 0.50/s, 900 a round, 500 m; 450 dmg/s, 0.50 E/s
  weapon    Large Cannon L152mmC at Base_LU_01: Lrg Cannon AP clip, 60 rounds; 0.50/s, 900 a round, 500 m; 450 dmg/s, 0.50 E/s
  firepower 3433 a second (derived), weapons ask 3.67 E/s, reach 500 m
  orders    Standby, Route, Seek and destroy, Guard, Refit; too big to capture
  wingman   Standby, Seek and destroy, Attack, Refit, Follow me
```

A large tracked warrior: four guns in four sockets — two nine-tube rocket
launchers and two 152 mm cannons, every one with a clip — the chassis's six
slots filled at size `b`, and a turret that takes a large radar and deflector.
Being size 4, it is offered neither capture package.
