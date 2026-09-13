# Weapons — guns, clips and rounds

What a weapon is made of, how it fires, what it spends, and what the stat panel
prints for it. The firing is in `Control.dll`'s gun class (component types 2
and 30); the numbers are in `guns.rlb` and `weapon.rlb`; the names are in
`objects.dlb` ([19-descriptions.md](19-descriptions.md)).

**Every claim is tagged**, as in [23-economy.md](23-economy.md): *measured*
is re-derived by `openparkan verify`, *read* comes from the disassembly at the
address given, *guess* fits the evidence and is not established.

## Three records make a weapon — *measured*

| record | `objects.rlb` | its controller | what the controller holds |
|---|---|---|---|
| the gun, `e_gun_<size><letter>_<NN>` (`EXTO`) | → `guns.rlb/o_gun_<size>a_<NN>` | one class-2 component: the gun | energy, rate, barrels, the round it fires, and a **slot** label `i_cNN_<size>` when it takes clips |
| the clip, `i_c<NN>_<size>_<mark>` (`INTO`) | → `guns.rlb/o_c<NN>_<size>_<mark>` | one class-2 component with a mass | the same energy, rate and round as its gun, and its own round count |
| the round, e.g. `bb_l_01` (`BULL`) | → `weapon.rlb` `.ctl`, `.ndp`, `.exp` | a projectile controller | speed, range, hit points, and the explosion that is its damage ([26-damage.md](26-damage.md)) |

So the `o_cNN` controllers are **ammunition clips**, not guns: the guns are the
`o_gun_*` controllers. The gun's letter splits by what it fires — `c` guns,
flamers, lasers and tasers; `l` rocket and missile launchers; `s` the mobile
builders (type 30, [32-builder.md](32-builder.md)); `f` the huge guns.

`openparkan.weapons.Armoury` reads all three: `gun(part)`, `clip(part)` and
`round(member)`.

## A gun is a capacitor, a magazine and a clock — *read*

The gun class (`Control.dll:0x100294c0`, 0x184 bytes, vtable `0x1003cbd8`)
reads four of its sixteen values; values 4–15 are zero on every shipped gun
and clip (*measured*).

| value | what it is | read at |
|---|---|---|
| 0 | the **magazine**: rounds the gun holds, an int; **−1 is unlimited** | parse `0x10029650` → `+0x124`; property 0x800 `0x1002bd14` |
| 1 | the **capacitor**: the charge the gun keeps ready; its draw tops it up ([23-economy.md](23-economy.md)) | parse → `+0x120` full; draw `0x10029a40` |
| 2 | the **energy a shot**, taken out of the capacitor | `0x1002a05e`; the stat panel's "wattage", MWt (property 0x1300) |
| 3 | the **interval**, ms between shots | `0x1002a085`; the panel's rate of fire is `1000 ÷ value 3` a second (`0x1002be7c`) |

**A shot** (`0x10029ca0`): no rounds left → no shot; the capacitor holding
less than value 2 → no shot, unless value 1 is 0 (the animals' guns fire
free); otherwise the gun fires, takes value 2 from the capacitor, sets its
level to `capacitor ÷ value 1`, and waits value 3 ms. Each round fired takes
one from the magazine unless the magazine is −1 (`0x1002a5e3`).

**Barrels.** A component's entries name the section-2 records its barrels sit
on. One shot fires the next barrel in turn (`0x1002a0d2`) — so a 9-tube
launcher looses one rocket every interval — **or every barrel at once when
record +8 carries bit `0x2000000`** (`0x10029fcc`). Seven multi-beam lasers and
the three builders set it (*measured*).

**Firing** (*read*). The input tables send `MCMD_STATE` with
`CIS_CONTINUEFIGHT` (0x100) to `CICLS_MULTIGUN` while the fire button is held
and `CIS_SWITCHOFF` when it is released, after `MCMD_SELECT` picked a weapon
([14-controls.md](14-controls.md)). The gun keeps the state word
(`0x1002a100`); `CIS_SINGLEFIGHT` (0x200) fires once and clears itself
(`0x1002a09f`). How the AI chooses among several weapons is not read here.

## Energy or clips — *measured*

- **Lasers and tasers need no ammunition.** All eight laser and four taser
  guns in the catalogue (the `_`-coded and huge ones aside) have magazine −1, no slot label, and no clip in any
  assembly. **The pumping laser is the exception**: it takes "Pumping shell"
  clips of 30–50.
- **Cannons, howitzers, the rail gun, flamers, rockets and missiles take
  clips**: 27 guns label their slot, and in the shipped assemblies 588 fitted
  guns hang exactly one clip while 318 hang none — each on the side its label
  says.
- **Every gun still pays energy a shot.** A cannon's is small (0.1–1), a
  laser's large (3.2–35), the rail gun's 12.5 — so at full rate lasers ask
  4.6–11.2 a second of their machine's power, cannons under 1 (*derived*).
- **The enemy variants have no magazine.** The eleven `_`-coded guns repeat a
  player gun's numbers with magazine −1, and the nine huge `fc`/`fl` guns fire
  every 2 s for 0.01 a shot (the huge missile launcher every 250 ms).
- **A launcher's magazine is its tubes**: 4, 9, 16 or 36, on 13 of 16. The
  winged SSMs hold 1 or 2 in three tubes.

## Clips — *measured*, and *read*

A clip carries its gun's values 1–3 and its gun's round — on all 58 clips in
`objects.dlb`, against 2 paired at random — and its own round count and mass,
both rising with the mark: 75 mm 300/400/500, 37 mm 300/600, medium cannon
60/100, rocket packs 9/18 or 16/32, winged packs 1/2 or 2/4.

**A fitted clip becomes the gun's magazine** — *read*, and *measured*. A clip is an
internal part: the loader re-parses the gun's class-2 component from the clip's record
([28-chassis.md](28-chassis.md#a-fitted-part-takes-over-its-slot--read-and-measured)).
The gun class's parser (`Control.dll:0x10029650`) sets the capacitor full from value 1
and the **rounds left** (`+0x124`) from value 0 — the clip's round count. The base
parser appends barrels only from a record's entries (`0x10021dc4`), and all 58 clips
declare none, so the gun keeps its own barrels (*measured*).

**There is no reload.** Every write of the rounds left was searched: the constructor
(`0x100295c4`), the parser (`0x100296d5`), a restore from a stream (`0x10029ba6`), each
shot taking one unless the magazine is −1 (`0x1002a5e3`), and `IControl`'s set
(`0x1002c374`), which is what the dock uses. A gun fires its clip round by round,
every interval, until it is empty, and then only a dock refills it.

A docked unit's guns gain 10% of the magazine a second, at least one round,
reading property 0x800 (magazine) and writing 0x700 (rounds left)
(`Behavior.dll:0x100181e0`, [27-ownership.md](27-ownership.md)).

## What the stat panel shows — *read*

`iron3d.dll:0x1006f300` fills a weapon's panel from its first device:

| row | source |
|---|---|
| Wattage, MWt | value 2, the energy a shot |
| Rate of fire, 1/s | `1000 ÷ max(1, value 3)` |
| Damage, HP | the round: the sum over its nodes of hit points plus level ratio × `.exp` damage (`Control.dll:0x10013620`) |
| Blast, m | the round's explosion radius — node 0's; the loop over the other nodes is not read (`0x100136c0`) |
| Range, m | the round controller's **+108** (`iron3d.dll:0x1006f4a0`), which `Control.dll:0x1000cfd0` also uses to hold a round within that distance of where it was fired |

The gun computes damage and blast from a sample round it spawns in its
slot-1 method (`0x100296f0`).

Seven lasers print a number instead of `damage` in `objects.dlb`. Each is
exactly **the beams it fires at once × one round** — 1350 = 3 × 450 on the
Large Red Laser, 840 = 2 × 420 on the Medium Dbl R.Las — counting the barrels
whose section-2 record names a control point (*measured*).

## The rounds — *measured*, and *read*

- **Speed** is the round controller's third triple, **range** its +108.
  Every laser and taser round flies at 10,000 m/s — a beam; lasers reach
  1,000 m, tasers only 90–130. Bullets fly at 250–500, rockets and missiles
  at 35–100, flame at 40–50.
- **Hit kind** is the `.exp` kind: bullets, beams, tasers and the rail gun hit
  one node directly; howitzer shells, flame, rockets and missiles blast.
  No other "damage type" field was found: a taser and a laser differ only in
  their numbers.
- **Missiles are guided and rockets are not.** The rounds of all ten missile
  launchers carry a class-17 seeker; those of the six rocket launchers do not.
  The two howitzer shells carry one too.

## Guided rounds differ in how hard they steer — *read*, and *measured*

A seeker keeps `cos(value 0)` as its cone (`Control.dll:0x100247a0`). Each tick
the round's own control system asks it for a heading (`0x1000ccc5`): the seeker
answers only while its target is within **value 1** m and inside the cone
(`0x100247c0`). The round then turns towards that heading on each axis at up to
its **controller's turn rate** — the frame's fourth triple, the same field that
turns a machine ([24-motion.md](24-motion.md)) — which clamps the command
(`0x1000cde5`). So how well a guided round follows a target is two numbers:
how wide it looks, and how fast it can turn.

| round | cone, rad | follows within, m | turns, rad/s | seeker value 2 |
|---|---:|---:|---:|---:|
| missiles, tiny to large (`bm_t/l/m/b_01`) | 0.80–0.95 | 500 | 1.2–1.6 | 3,000–5,000 |
| huge missile (`fm_h_01`) | 1.57 | 700 | 1.3–1.4 | 3,500 |
| winged SSMs (`bm_m/b_04`) | 0.70 | 500 | 0.5 | 7,000–10,000 |
| howitzer shells (`bb_m/b_02`) | 0.26–0.27 | 350–400 | 0.35–0.40 | 500–750 |
| rockets (`br_*`, `fr_l_01`) | no seeker | — | 1.52, unused | — |

*Measured*: every missile round looks wider and turns faster than both guided
shells. A howitzer shell corrects gently inside a 15° cone; a missile looks
through about three times the angle and turns three to four times as fast; the
winged SSMs look wide but turn slowly. The seeker's value 2 is not read by
either method.

## The weapons the player builds — *measured*, with *derived* rates

Damage a round is the panel's; damage a second is `damage × shots a second`,
× the beams on a salvo gun. It ignores armour, shields and the level ratio
([26-damage.md](26-damage.md)).

| weapon | code | kind | ammunition | energy a shot | ms between shots | shots a second | barrels | damage a round | blast m | round m/s | range m | damage a second | energy a second |
|---|---|---|---|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|
| Tiny 37mm Cannon | `T37Can` | GUN | clip 300/600 | 0.1 | 200 | 5.00 | 1 | 100 | — | 350 | 300 | 500 | 0.5 |
| Tiny Laser | `LTR` | LAS | energy | 3.2 | 700 | 1.43 | 1 | 150 | — | 10000 | 1000 | 214 | 4.6 |
| Tiny Missile Lr | `TML2T` | MIS | clip 2/4 | 0.2 | 1250 | 0.80 | 2 | 280 | 4 | 70 | 300 | 224 | 0.2 |
| Tiny Taser | `TTas` | TAS | energy | 1.8 | 800 | 1.25 | 2 | 160 | — | 10000 | 90 | 200 | 2.2 |
| Small Flame Thrower | `SFT` | FLM | clip 60/80/100 | 0.1 | 500 | 2.00 | 1 | 105 | 2 | 50 | 140 | 210 | 0.2 |
| Small Autocannon | `S75Can` | GUN | clip 300/400/500 | 0.15 | 250 | 4.00 | 1 | 150 | — | 350 | 350 | 600 | 0.6 |
| Small Red Laser | `SRLs` | LAS | energy | 5 | 700 | 1.43 | 1 | 235 | — | 10000 | 1000 | 336 | 7.1 |
| Small Missile Lr | `SML4S` | MIS | clip 4/8 | 0.25 | 1250 | 0.80 | 4 | 340 | 6 | 65 | 350 | 272 | 0.2 |
| Small Rocket Lr | `SRL9S` | ROC | clip 9/18 | 0.24 | 750 | 1.33 | 9 | 225 | 2 | 100 | 250 | 300 | 0.3 |
| Small Taser | `STasMK1` | TAS | energy | 2.5 | 1000 | 1.00 | 2 | 290 | — | 10000 | 110 | 290 | 2.5 |
| Medium Flame Thrower | `MFT` | FLM | clip 25/35/45 | 0.45 | 750 | 1.33 | 1 | 420 | 2.5 | 50 | 170 | 560 | 0.6 |
| Medium Cannon | `M125Can` | GUN | clip 60/100 | 0.55 | 900 | 1.11 | 1 | 500 | — | 300 | 400 | 556 | 0.6 |
| Medium Howitzer | `M125How` | GUN | clip 60/100 | 0.4 | 1200 | 0.83 | 1 | 400 | 2.5 | 100 | 350 | 333 | 0.3 |
| Medium Red Laser | `MRLas` | LAS | energy | 16 | 2250 | 0.44 | 1 | 750 | — | 10000 | 1000 | 333 | 7.1 |
| Medium Dbl R.Las | `MDRLas` | LAS | energy | 18 | 1600 | 0.62 | 3 (salvo 2) | 420 | — | 10000 | 1000 | 525 | 11.2 |
| Medium Dbl G.Las | `MDGLas` | LAS | energy | 20 | 1850 | 0.54 | 3 (salvo 2) | 500 | — | 10000 | 1000 | 541 | 10.8 |
| Medium Missile Lr | `MML4M` | MIS | clip 4/8 | 0.75 | 2000 | 0.50 | 4 | 810 | 10 | 60 | 400 | 405 | 0.4 |
| Medium winged SSM | `MWML1M` | MIS | clip 1/2 | 0.5 | 13000 | 0.08 | 3 | 60000 | 45 | 45 | 700 | 4615 | 0.0 |
| Medium Missile Lr | `MML9S` | MIS | clip 9/18 | 0.25 | 1250 | 0.80 | 9 | 340 | 6 | 65 | 350 | 272 | 0.2 |
| Medium Rocket Lr | `MRL9M` | ROC | clip 9/18 | 0.7 | 1500 | 0.67 | 9 | 680 | 3.5 | 90 | 300 | 453 | 0.5 |
| Medium Rocket Lr | `MRL16S` | ROC | clip 16/32 | 0.24 | 750 | 1.33 | 16 | 225 | 2 | 100 | 250 | 300 | 0.3 |
| Medium Taser | `MTas` | TAS | energy | 8 | 1750 | 0.57 | 2 | 900 | — | 10000 | 130 | 514 | 4.6 |
| Large Flame Thrower | `LFT` | FLM | clip 30/40/50 | 0.85 | 1500 | 0.67 | 1 | 790 | 3.5 | 40 | 200 | 527 | 0.6 |
| Large Rail Gun | `L80mmRG` | GUN | clip 75/100 | 12.5 | 1200 | 0.83 | 1 | 1200 | — | 500 | 600 | 1000 | 10.4 |
| Large Cannon | `L152mmC` | GUN | clip 40/60 | 1 | 2000 | 0.50 | 1 | 900 | — | 250 | 500 | 450 | 0.5 |
| Large Howitzer | `L152mmH` | GUN | clip 60/80 | 0.75 | 3000 | 0.33 | 1 | 700 | 4 | 85 | 400 | 233 | 0.2 |
| Lrg Pumping Laser | `LPumL` | LAS | clip 30/40/50 | 1.4 | 2500 | 0.40 | 1 | 1300 | — | 10000 | 1000 | 520 | 0.6 |
| Large Red Laser | `LTRedL` | LAS | energy | 28 | 3100 | 0.32 | 4 (salvo 3) | 450 | — | 10000 | 1000 | 435 | 9.0 |
| Large Green Laser | `LTGrnL` | LAS | energy | 32 | 3350 | 0.30 | 4 (salvo 3) | 500 | — | 10000 | 1000 | 448 | 9.6 |
| Large Blue Laser | `BTBluL` | LAS | energy | 35 | 3600 | 0.28 | 4 (salvo 3) | 550 | — | 10000 | 1000 | 458 | 9.7 |
| Large Missile Lr | `LML4L` | MIS | clip 4/8 | 2.2 | 4000 | 0.25 | 4 | 2100 | 12 | 55 | 450 | 525 | 0.6 |
| Large Missile Lr | `LML9M` | MIS | clip 9/18 | 0.75 | 2000 | 0.50 | 9 | 810 | 10 | 60 | 400 | 405 | 0.4 |
| Large Winged SSM | `LWML1L` | MIS | clip 1/2 | 0.7 | 16000 | 0.06 | 3 | 100000 | 60 | 35 | 700 | 6250 | 0.0 |
| Large Missile Lr | `LML16S` | MIS | clip 16/32 | 0.25 | 1250 | 0.80 | 16 | 340 | 6 | 65 | 350 | 272 | 0.2 |
| Large Winged SSM | `LWML2M` | MIS | clip 2/4 | 0.5 | 13000 | 0.08 | 3 | 60000 | 45 | 45 | 700 | 4615 | 0.0 |
| Large Rocket Lr | `LRL9L` | ROC | clip 9/18 | 2 | 1500 | 0.67 | 9 | 1900 | 4 | 80 | 450 | 1267 | 1.3 |
| Large Rocket Lr | `LRL16M` | ROC | clip 16/32 | 0.7 | 1500 | 0.67 | 16 | 680 | 3.5 | 90 | 300 | 453 | 0.5 |
| Large Rocket Lr | `LRL36S` | ROC | clip 36 | 0.24 | 750 | 1.33 | 36 | 225 | 2 | 100 | 250 | 300 | 0.3 |
| Large Taser | `LTas` | TAS | energy | 14 | 2750 | 0.36 | 2 | 850 | — | 10000 | 90 | 309 | 5.1 |

The enemy variants and the huge guns:

| weapon | code | kind | ammunition | energy a shot | ms between shots | shots a second | barrels | damage a round | blast m | round m/s | range m | damage a second | energy a second |
|---|---|---|---|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|
| Large Rail Gun | `_RG` | GUN | unlimited | 12.5 | 1200 | 0.83 | 1 | 1200 | — | 500 | 600 | 1000 | 10.4 |
| Large Flame Thrower | `_FT` | FLM | unlimited | 0.85 | 1500 | 0.67 | 1 | 790 | 3.5 | 40 | 200 | 527 | 0.6 |
| Large Cannon | `_152C` | GUN | unlimited | 1 | 2000 | 0.50 | 1 | 900 | — | 250 | 500 | 450 | 0.5 |
| Large Howitzer | `_152H` | GUN | unlimited | 0.75 | 3000 | 0.33 | 1 | 700 | 4 | 85 | 400 | 233 | 0.2 |
| Lrg Pumping Laser | `_PumL` | LAS | unlimited | 1.4 | 2500 | 0.40 | 1 | 200 | — | 10000 | 1000 | 80 | 0.6 |
| Large Rocket Lr | `_RL9L` | ROC | unlimited | 2 | 1500 | 0.67 | 9 | 1900 | 4 | 80 | 450 | 1267 | 1.3 |
| Large Missile Lr | `_ML4L` | MIS | unlimited | 2.2 | 4000 | 0.25 | 4 | 2100 | 12 | 55 | 450 | 525 | 0.6 |
| Large Missile Lr | `_ML9M` | MIS | unlimited | 0.75 | 2000 | 0.50 | 9 | 810 | 10 | 60 | 400 | 405 | 0.4 |
| Large Rocket Lr | `_RL16M` | ROC | unlimited | 0.7 | 1500 | 0.67 | 16 | 680 | 3.5 | 90 | 300 | 453 | 0.5 |
| Large Missile Lr | `_ML16S` | MIS | unlimited | 0.25 | 1250 | 0.80 | 16 | 340 | 6 | 65 | 350 | 272 | 0.2 |
| Large Rocket Lr | `_RL36S` | ROC | unlimited | 0.24 | 750 | 1.33 | 36 | 225 | 2 | 100 | 250 | 300 | 0.3 |
| Huge Cannon | `L152mmMC` | GUN | unlimited | 0.01 | 2000 | 0.50 | 1 | 640 | 2.5 | 200 | 500 | 320 | 0.0 |
| Huge Red Laser | `HRLM` | LAS | unlimited | 0.01 | 2000 | 0.50 | 1 | 800 | — | 10000 | 1000 | 400 | 0.0 |
| Huge Cannon | `H152mmBC` | GUN | unlimited | 0.01 | 2000 | 0.50 | 1 | 640 | 2.5 | 200 | 500 | 320 | 0.0 |
| Huge Pumping Laser | `HRLB` | LAS | unlimited | 0.01 | 2000 | 0.50 | 1 | 1700 | — | 10000 | 1000 | 850 | 0.0 |
| Huge DG Laser | `HDGLB` | LAS | unlimited | 0.01 | 2000 | 0.50 | 2 (salvo 2) | 550 | — | 10000 | 1000 | 550 | 0.0 |
| Huge TB Laser | `HTBLB` | LAS | unlimited | 0.01 | 2000 | 0.50 | 4 (salvo 3) | 470 | — | 10000 | 1000 | 705 | 0.0 |
| Huge Flame Thrower | `HFTB` | FLM | unlimited | 0.01 | 2000 | 0.50 | 1 | 790 | 6 | 45 | 250 | 395 | 0.0 |
| Huge Rocket Lr | `HMRL9B` | ROC | unlimited | 0.01 | 2000 | 0.50 | 9 | 1350 | 5 | 75 | 400 | 675 | 0.0 |
| Huge Missile Lr | `HBML9B` | MIS | unlimited | 0.01 | 250 | 4.00 | 9 | 3000 | 15 | 75 | 700 | 12000 | 0.0 |

## Not established

- Values 8–10, which the fire routine reads (`0x10029d3a`, `0x10029e50`) and
  every shipped gun leaves at 0.
- What marks a barrel to be skipped (bit 0x40 of its attachment record,
  `0x10029ff5`); the control-point word of section 2 matches it on the seven lasers.
- A seeker's value 2, and the scale on the steering command (`0x100430d4`, set at
  `0x1000d9fc`).
- How the AI picks a weapon when a unit has several.
