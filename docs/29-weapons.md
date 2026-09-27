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
one from the magazine unless the magazine is −1 (`0x1002a5e3`). The wait
starts only once the barrel has finished its stroke, so **value 3 is not the
whole time between shots**
([Firing, from button to round](#firing-from-button-to-round--read-and-measured)).

### What refills the capacitor — *read*, and *measured*

**Nothing but the power tick.** The gun is priced like every other part: its
class slot 5 (`0x10029a40`) answers what it wants of its channel over `dt`
seconds — **its `power` figure times `dt`, plus whatever its capacitor lacks of
value 1** — and its slot 6 (`0x10029a90`) takes the tick's level `r` and puts
`r × want − power × dt` into `+0x120`, then sets the component's level `+0x4c`
to `charge ÷ value 1`. There is no other writer of `+0x120` than the parse
(full, `0x1002967f`), a shot (`0x1002a06a`) and a dock's rearm.

- **Whose batteries.** Its carrier's, through the control system every agent
  but a projectile shares — a bot's own fitted battery, a building's clan
  power ([23-economy.md](23-economy.md#bots-spend-power-through-the-same-code-priced-by-part--read-and-measured)).
  Weapons are served on the last channel, after the engines, the repair, the
  devices and the shield, so they get what is left.
- **Against which clock.** The power tick, **250 ± 31 ms**
  (`Control.dll:0x1000c756`), about four times a second, not the gun's own
  takt and nothing in a data file.
- **At what rate.** **Every one of the 158 shipped gun and builder components
  has a `power` of 0** (*measured*), so the ask is exactly the lack and the
  grant exactly `r ×` it: at full power a capacitor is **whole again on the
  next tick**, and short of power it closes the fraction `r` of its gap each
  tick. The game adds the difference unguarded and clamps nothing, which with
  a power of 0 and `r ≤ 1` can neither overshoot value 1 nor go backwards.
- **The numbers** (*measured*, 158 components on 133 records — 93 `EXTO` guns,
  58 `INTO` clips and 7 on `BTLU` units): capacitors run **0.1 to 6400**, and
  **every gun's capacitor holds at least two shots** — the tightest is 2.00
  (`e_gun_bl_17`, `e_gun_ml_18` and their clips) and the loosest 6666.67
  (`e_tur_bb_11`). So no gun is ever stopped by charge on a single shot; what
  costs is sustained fire, 0.04 to 31.22 units a second at full rate against a
  bot battery's 5 to 34.5.
- **A gun with no capacity has no level, ever.** All three writers of `+0x4c`
  stand behind a `value 1 > 0` gate — the tick (`0x10029ae2`), the load setter
  (`0x10029b25`) and **the shot** (`0x1002a03d`) — so on the five animals'
  guns, the only ones with value 1 at 0 (`a_l_01`…`a_l_05`, *measured*), the
  level keeps the 0 the base component's constructor leaves
  (`0x1001f29c`, `0x1001f2cf`). A shot does not set it: it skips the whole
  capacitor block.

**Barrels.** A component's entries name the section-2 channels its barrels
are. One shot fires the next barrel in turn (`0x1002a0d2`) — so a 9-tube
launcher looses one rocket every stroke and interval — **or every barrel at
once when record +8 carries bit `0x2000000`** (`0x10029fcc`). Seven
multi-beam lasers and the three builders set it (*measured*). Either way a
barrel whose channel takes the previous channel's value (flag `0x40`,
[30-turrets.md](30-turrets.md#aiming-and-the-camera--read-and-measured)) is
passed over (`0x10029ff5`).

**Firing** (*read*). The input tables send `MCMD_STATE` with
`CIS_CONTINUEFIGHT` (0x100) to `CICLS_MULTIGUN` while the fire button is held
and `CIS_SWITCHOFF` when it is released, after `MCMD_SELECT` picked a weapon
([14-controls.md](14-controls.md)). The gun keeps the state word
(`0x1002a100`); `CIS_SINGLEFIGHT` (0x200) fires once and clears itself
(`0x1002a09f`). The next section follows a shot from the button to the round.
The AI does not choose among its weapons: it scores each one
([How the AI fires](#how-the-ai-fires--read)).

## Firing, from button to round — *read*, and *measured*

### The button reaches the selected guns

The hero's left mouse button sends `MCMD_STATE` with index −1:
`CIS_CONTINUEFIGHT` on the press and `CIS_SWITCHOFF` on the release. Keys 0–8
send `MCMD_SELECT` with index −1 or 1–8 (*measured*, `hero.tbl`).

`World3D.dll`'s row handler keeps two bytes for each component the player may
drive. One says it may fire; the other says it is **selected**. It routes the
weapon commands this way (`World3D.dll:0x100109f8`):

| row | what happens |
|---|---|
| `MCMD_STATE`, index −1 | the state goes to **every selected gun** (`0x100105ed`) |
| `MCMD_STATE`, index *n* | the state goes to the *n*-th gun, selected or not |
| `MCMD_SELECT` *n* | **toggles** the *n*-th gun (`0x10010770`). Selecting resets it (state `0x1000`) and sends the *n*-th class-24 component state 1. Deselecting switches the gun off and sends that component state 2 |
| `MCMD_SELECT` −1 | selects and resets every gun, and sends every class-24 component `0x21` (`0x1001084a`) |
| `MCMD_MISSILE` | the state goes to the selected guns whose round's frame +116 carries `0x10` (`0x1001065f`) |
| `MCMD_FIRE_ALL` | the state goes to every gun (`0x100106d3`) |

**Which guns start selected** (`World3D.dll:0x1000ed20`, *read*):

- A gun whose round's frame +116 carries 4 starts selected. On the hero those
  are the cannon and the laser (`bb_h_01`, `bl_h_01`).
- Otherwise, when the driven object's type is `0x1020000`, the hero's, the gun
  starts deselected. On the hero
  those are the plasma rifle and the missiles.
- Any other machine's guns start selected.
- So **in Mission 01 the button fires the cannon and the laser together**
  until a number key changes the set.
- The same method says who may fire at all
  ([below](#who-may-drive-a-units-guns--read)).

**The class-24 components are the arms.** The only controller that has any is
the hero turret, which pairs four of them with its four guns (*measured*).
Each arm's channels play frames 42–48 on the arm nodes.

**An arm is an item** (*read*). Type 24 is built as the base component
(`0x10020800`, the factory's default at `0x1002d6ec`), the class a door or a
control pod is ([27-ownership.md](27-ownership.md#capture--read)).

- **Its update** (`0x10020900`) moves a progress (`+0x94`, starting at 0) by
  0.45 a step: toward 1 while the state word's low bits are 1, toward 0 while
  they are 2 (`0x10020a28`). Every channel heads for the progress, and the
  state clears when the progress passes its end.
- **So 1 and `0x21` unfold an arm**, frames 42 → 48, and **2 and `0x22` fold
  it**. The update reads only the bits 1, 2, 4 (wrap) and 8 (back and forth),
  so `0x20` changes nothing.
- **It takes 0.5 s.** All four arms' channels run at 2 a second, and the three
  steps of 0.45, 0.45 and 0.1 last 225, 225 and 50 ms (*measured*).
- **At the start** all four records hold state `0x21` (+0x18 = 33,
  *measured*). The handler then sends `0x21` to the cannon's and laser's arms
  and `0x22` to the plasma rifle's and the missiles' (`0x1000ef4e`,
  `0x1000ef9e`). So two arms unfold and two stay folded.
- **What an arm is for**: a gun on it is not ready until it is out
  ([below](#a-gun-is-ready-once-its-arm-is-out--read-and-measured)).

### Who may drive a unit's guns — *read*

`World3D.dll`'s row handler is the unit's **manual controller**
(`CreateManManager`, interface `0x19`). `AniMesh.dll` builds one for every
agent (`0x10003470`).

- **Its permission method** (`0x1000ed20`) takes a component, or −1 for the
  unit, and two bits. Bit 0 lets that component take input and bit 1 lets it
  fire; they are the bytes the rows test.
- **Its caller is `Wizard.dll`** (`0x10003890`), which re-sends the bits for
  every component whenever its words change. No other call of it was found; the
  search followed the interface `0x19` pointers `Wizard.dll` and `iron3d.dll`
  keep.

The Wizard decides who drives:

- **A mode word** (`+0x1fc`) is set by message 7: 1 is the player, 0 the AI,
  and 2 freezes both (`0x10001cd1`).
- **One word per group** (`+0x200`–`+0x21c`) is set through a mask
  (`0x10002070`): 0 follows the mode, 1 gives the group to the AI, 3 to the
  player, 2 to neither.
- **The groups are the power channels** of
  [23-economy.md](23-economy.md): interface `0x204` slot 9 (`0x1002c3a0`) looks
  a component's class up in the same table (`Control.dll:0x1003ccc8`). Group 4
  is the turret, the guns, the arms and the builder module (classes 1, 2, 22,
  24, 30); 2 the camera, radar and seeker; 3 the engines; 5 the shields and
  armour.
- **The player's side** gets bits 3 from the manual controller. **The AI's
  side** gets them from `MBehaviour`'s mode set (`Behavior.dll:0x100067b0`),
  where the unit's bits 2, 4 and 8 become the flags `0x10` (movement), `0x20`
  and `0x40` (the fight module).

`iron3d.dll` hands a unit to the player at `0x10074ff0`:

- **The hero** gets every group at once (mask `0xfff`, word 3). Every one of
  its guns may fire, and none of its AI runs.
- **A bot** gets groups 4, 5, 2 and 0 for the player, and message 7 with 1, so
  the groups left at 0 follow the player too. Its record's `+0x9c` changes
  this: 1 leaves the unit's own word with the AI, so its movement takt runs;
  2 gives the AI the unit, its weapons and its shields, and sends message 7
  with 0.
- **Letting go** sends message 7 with 0 and hands the words back.

So a unit's guns take the button once the player has taken it over. The
selected byte then decides which of them the button reaches.

### The gun's takt: a stroke, then the interval

A component is woken when its next-event time (`+0x10`, in ms) passes. The
time driver catches up, running several events in one frame if it has to
(`0x1002d2e2`). At each wake the gun's fire method either continues a barrel
stroke or starts one. It starts one only if all of these hold:

- its node has life: slot 2 (`0x10021820`, called at `0x10029cc3`) answers 1 at
  a life of 0, and the gun reports 5 as for an empty magazine (`0x10029f8d`). A
  stroke already under way is continued before the test (`0x10029cb9`), so its
  round still leaves. A gun part's node 0 is its turret's socket
  ([28-chassis.md](28-chassis.md#the-order-parts-load-in-and-what-a-slot-keeps--read-and-measured)),
  so a turret destroyed takes the gun's nodes, and the gun, with it
  ([26-damage.md](26-damage.md#what-a-damaged-node-a-destroyed-part-and-a-dead-unit-draw--read-and-measured));
- it has rounds (state 5 otherwise);
- its capacitor holds value 2 (state 6 otherwise);
- its ready byte `+0x118` is set (state 7 otherwise, `0x10029d27`). The
  constructor sets it (`0x100295b3`), and a turret's takt then rewrites it
  once its arm is out
  ([below](#a-gun-is-ready-once-its-arm-is-out--read-and-measured));
- its state word is non-zero.

Values 8–10 add a target gate: no shot at a target farther than value 8
(`0x10029e37`) or further off the barrel than value 10 allows (`0x10029edd`),
and a value-9 lock in seconds counted down first (`0x10029f2c`). The files
leave them zero, but **the gun fills them from its round**, so every gun has
a range gate and every guided gun a lock
([A guided gun waits for a lock](#a-guided-gun-waits-for-a-lock--read-and-measured)).

**A barrel stroke has four steps** (`0x1002a190`):

| step | what happens | read at |
|---|---|---|
| 1 | the barrel's channel heads for 0.5, and the gun's own section-5 group runs (component record +0xc) | `0x1002a1df` |
| 2 | **the round leaves** | `0x1002a266` |
| 3 | the channel heads for 1.0 | `0x1002a5a8` |
| 4 | the channel snaps back to 0 and the magazine loses one | `0x1002a5c1` |

- **Timing.** After steps 1 and 3 the gun sleeps until the channel arrives:
  1000 × distance ÷ rate ms, so 500 ÷ rate each (`0x10022120`).
- **After step 4.** The shot is done: the capacitor pays value 2, the gun
  sleeps value 3 ms, and the next barrel is up.
- **A stroke always finishes**, even if the button is released during it.

So **one shot takes 1000 ÷ rate ms of stroke plus value 3**, give or take a
frame at each wake, and the round leaves halfway through the stroke. On the hero
(*measured*):

| gun | round | barrel point | channel rate | value 3, ms | one shot, ms | shots a second |
|---|---|---|---:|---:|---:|---:|
| cannon | `bb_h_01` | `Mgun_d` | 4 | 0 | 250 | 4 |
| plasma rifle | `bp_h_01` | `Plaz_d` | 2 | 250 | 750 | 1.33 |
| laser | `bl_h_01` | `Laz_d` | 4 | 200 | 450 | 2.22 |
| missiles | `bm_h_01` | `Roc_d1`, then `Roc_d2` | 1, 2.5 | 1,250 | 2,250, then 1,650 | 0.51 |

The cannon's value 3 of 0 is therefore not unlimited fire. Its barrel's
stroke limits it to four shots a second. The stat panel's `1000 ÷ max(1,
value 3)` does not count the stroke. The missile barrels' channels carry flag
4: they are timed but not animated. Every one of the 388 barrel channels in
the install has a rate (*measured*).

### A gun is ready once its arm is out — *read*, and *measured*

The ready byte is set by the turret, through the gun's **mount**. A mount is a
follower channel: a section-2 channel flagged 8, which joins the turret's list
at load (`0x10009120`).

**Pairing** (`0x10027170`, when the turret starts):

- The turret's channels are its yaw and pitch, then the followers in load order.
  A follower also flagged `0x40` is passed over.
- The *n*-th follower left takes the next gun (class 2 or 30) and the next arm
  (class 24), each found from just after the last one taken (`0x1002c3c0`), or
  none.
- *Measured*: on the hero turret, followers 4, 9, 21 and 25 take the cannon,
  plasma rifle, laser and missiles, and arms 8 to 11. A fitted gun part carries
  one follower of its own. On every controller but two, each follower finds a
  gun; `e_gun_bl_03` and `e_gun_tl_02` have a follower and no gun, and **they
  are fitted on nothing**, so the takt never meets an unpaired one: summed over
  the merged control system of each of the 458 shipped assemblies, 1,015
  followers and not one without a gun
  ([The two guns nothing carries](#the-two-guns-nothing-carries--measured)).
  No channel carries the `0x2000` flag the takt treats apart.

**Each takt** (`0x10027ecd`), `0x10028200` moves each mount and returns its
gun's ready byte (`0x10027f51`):

1. **The arm's progress *p*** is read, or 1 where there is no arm. While
   *p* < 1 the mount heads for (1 − *p*) × its initial value + *p* × the pitch
   target, and **the gun is not ready**.
2. **Once the arm is out**, the mount heads for the pitch channel's target.
3. **A falling round adds an elevation.** The gun keeps its round's top speed
   (`+0x94`) and a gravity flag (`+0x98`), which is 1 when the round's mode is
   not 0 (`0x100297ef`). With *g* = the world's 10 × that flag and *d* the target
   from `TurretCenter` — the point a turret in `CIS_POINTTRACE` traces, less
   `TurretCenter`'s position (`0x1001b4f0`, handed in by the turret's takt,
   `0x10027f06`–`0x10027f51`) — the mount solves for the flight time, *t*² =
   2 (*A* ∓ √*D*) ÷ *g*² with *A* = *v*² − *g d*z and *D* = *A*² − *g*²|*d*|²,
   the lower arc first (`0x10028401`). It rises by the angle between the launch
   and the straight line × 0.83 ÷ the channel's span, signed by the z of
   `TurretCenter`'s direction.
   No solution leaves **the gun not ready**.
4. **Otherwise the gun is ready.** A mode-0 round skips step 3 (its gravity is 0,
   `0x100283e2`), and so does a turret in `CIS_MANUALCONTROL`. Every hero round is
   mode 0; the four mode-3 rounds are the flamers' (*measured*).

So on the hero (*derived*) **the cannon and the laser are ready half a second
after their arms start unfolding**, and **a gun selected with its number key is
ready half a second after the key**. The plasma rifle and the missiles are
never ready until selected, and a gun deselected mid-fold stops being ready at
once.

#### A gun with no follower is ready from birth — *read*, and *measured*

A gun that gets no mount is never touched by the turret's takt. Its ready byte
(`+0x118`) has **two writers**: the gun's constructor, which sets it to **1**
(`0x100295b3`), and the mount step (`0x10027f51`). The shot's test
(`0x10029d1d`) is the only other reference to the field as a byte in
`Control.dll`. So a gun with no mount is **permanently ready**, takes no
elevation, and its round leaves along its barrel's own direction.

**A fitted gun part carries its own follower**, which joins the turret's list at
load (`0x10009120`), so the pairing finds it even when the turret record has none
of its own. *Measured*: `e_bnt_lt_01`, the Small Bunker's turret, has three
channels and not one flagged 8, but each `e_gun_fc_08` bolted to it holds a
follower (node 2, flags `0x8`, no control point) beside its class-2 gun — so the
bunker's two flamers are mounted, and a flamer's lobbed round is lifted like any
other. **The case this page once worried about does not exist in the shipped
data.**

*Measured* over the 458 assemblies, summed across each one's merged control system:
**1,015 followers against 1,034 guns**, and **19 assemblies fit more guns than
followers**, so 19 guns hang on nothing:

| gun | round mode | fittings | where |
|---|---:|---:|---|
| `a_l_01`, `a_l_02`, `a_l_03` | 0 | 1 each | the tushkan, the bird and the tent animal, whose root controller has a turret and a gun and no follower at all |
| `e_gun_lc_01`, `e_gun_lc_03` | 0 | 1, 5 | the gun a builder fills its battle slot with |
| `e_gun_ls_10`, `e_gun_ms_12` | 0 | 5, 5 | a builder module's **second** class-30 beam: the module brings one follower and two beams |

**Every one fires a mode-0 round**, so no lobbing gun is ever left unmounted, and
each of the 19 is a gun that wants no elevation anyway — an animal bites, a builder
beam reaches out to a site. Never the other way round: no assembly has a follower
that would find no gun
([The two guns nothing carries](#the-two-guns-nothing-carries--measured)).

### Where the round leaves, and which way

- **The muzzle** is the barrel channel's control point (+0x14), in world space,
  position and direction (`0x1002a302`): `Mgun_d` is 0.895 along +y on
  `Gun01_m1o1`.
- **The direction converges on the sight** when the gun sits on a turret and
  its round's controller is mode 0 (`0x1002a34c`). Every round but the four
  lobbed `bf_*_01` is mode 0.
  - The turret gives each of its guns two points: the yaw channel's second
    point and the pitch channel's point (`0x10028130`). On all 59 turret
    components those are **`TurretCenter` and `TargetDirect`** (*measured*).
  - The gun casts a ray from `TurretCenter` along `TargetDirect`, from 5 m to
    1,000,000 m, into the world's segment query (IWorld slot 7, `0x1002a768`).
  - The first hit, pushed out to at least 100 m (`0x1002a7be`), is the aim
    point. The round flies from its muzzle straight at it (`0x1002a610`).
- **With no hit** the round keeps its barrel's own direction.
- **No spread and no lead** are applied to the player's shot.

**What the sight ray meets** (*read*, `Terrain.dll:0x10024fd0`):

- **IWorld slot 7 walks the world's object tree** from its root
  (`0x100250c0`). Each object whose class bit is in the query's mask is tested
  against its bounding sphere and then through its interface `0x18` slot 6.
  Its children are walked the same way, and the nearest hit wins.
- **The sight ray's mask is `0xfff`**, every class (`Control.dll:0x1002adc0`),
  and it excludes no batch and no triangle.
- **A round's own query differs** (`0x1001d9d0`): it asks for classes `0x41e`
  only, and passes through triangles flagged 4 or 32 and batches flagged 8 or
  `0x200` ([26-damage.md](26-damage.md)). So the sight can stop on a tree's
  leaves that the round then flies through.
- **The ground, and it is the first thing walked** — *read*. The world's
  **root object is the landscape**: `CLightning::Init` asks the world for its
  root through the same slot `0x30` the sight ray uses (`0x10071c33`,
  `0x10024feb`), asks that root for its class through the same slot `0x2c` the
  walk uses (`0x10071c41`, `0x10025101`), and **panics *"Root object is not a
  landscape"* unless the class is 1**. The walk admits an object whose
  `1 << class` meets the query's mask, through a table that is `1 << N` for
  N = 0..15 (`0x1002510f`, `Terrain.dll:0x1009a5f0`), and the sight ray's mask
  `0xfff` holds bit 1. The landscape answers interface `0x18` at its `+0x138`
  (`0x1001a174`, `0x1001a202`), and **slot 6 of that interface's vtable
  (`0x1009a3f8` + 0x18) is `CLandscape::GetFirstIntersectedFace`**
  (`0x100205c0`) — the very routine a round's ground test runs. So the sight
  ray does converge on the ground.
  - The control is the same enumeration: it also finds the object lists the
    walk descends into, the children at `[vtable+0x3c]` (`0x100254e4`,
    recursing at `0x10025520`), and an object that fails the class mask or has
    no interface `0x18` is skipped while its children are still walked.
- **The sight ray excludes no face, so a lake stops it** — *read*. Its query
  is `[0xfff, 0, 0, ?, 0, 0, 0, 0]` (`0x1002a68e` → `0x1001bca0`, then
  `0x1002a6c1`–`0x1002a6cd` zeroing the last four): no required or excluded
  world flags and no required or excluded class. A round's excludes world
  flags `0x208` *and* class `0x24`, and it is the **class** that carries the
  water: against the landscape `0x24` becomes the surface word's `0x02` and
  `0x01`, of which `0x02` is the water sheet on 3630 of 275882 faces, while
  `0x208` becomes the flags word's `0x20` and `0x80`, on none
  ([26-damage.md](26-damage.md#the-query-record-and-what-a-round-excludes--read-and-measured)).
  **So the sight stops on a lake's surface where the round it aims flies
  through to the bed.**

### The round's start

The round is created facing its direction with z up (`CreateObject` 9,
`0x1002a387`) and set up before it joins the game (`AddObjectToGame`,
`0x1002a58f`):

- **Owner** (property `0x7f`, `0x1002a3dc`): the id of the object whose control
  system holds the gun. That is **the whole robot**, since a unit's parts
  share one control system
  ([28-chassis.md](28-chassis.md#a-fitted-part-takes-over-its-slot--read-and-measured)).
  The hit test skips that id.
- **Level ratio:** property `0xb4`, the gun's `+0x180`.
- **Velocity:** its top speed along its own y, plus the shooter's world
  velocity (`+0x200`, which the body adds to its position each step,
  `0x10015920`). It is set with `SetWorldSpeed` (`0x1002a4b4`).
  - `SetTangSpeed` behind it sets the **free-flight** byte (`0x100044a0`), and
    while that byte is set the integrator does not pull the velocity toward
    the command (`0x100153f4`). So a round keeps its launch speed.
  - A mode-0 round's sideways components of that velocity, in its own frame,
    bleed off at 0.003 m/s per elapsed ms (`0x1000ceec`).
  - A round whose command (property `0x20`) is already non-zero at spawn gets
    `SetTangAccel` with it too. No such round was found.
- **Target:** the gun's target, through the round's interface `0x204` slot 16
  (`0x1002a514`), which sets its seeker's target (`0x1002cb00`).

### Guided rounds follow the turret's target

A gun's target (`+0x108`) is its turret's (`0x10028130`). The turret's target
is set through the unit's interface `0x204` slot 16, which also sets the
unit's own seeker.

**The AI and the player's target list set it** (*read*):

- **One writer.** The turret's target is written by its set-target method
  (`0x100280d0`), and only the stream restore writes it besides.
- **One way in.** That method is reached only through the device interface's
  slot 16, `SetTarget` (`0x1002cb00`), which sets the unit's seeker too.
- **The AI's calls.** Each call at that slot's offset with a target argument
  was listed in every module. Five are in `Behavior.dll`'s fight code:
  - `0x10024b1b` and `0x10024f8f` aim, in the fight module's takt;
  - `0x10025107` clears the target with (0, 0) when the module is reset
    (`0x10025070`, from the mode set once flag `0x20` is on, and from
    `0x10031b30`);
  - `0x100253dc` sits in `0x10025320`, which only the takt's playback branch
    calls (`0x10004f13`), and `0x10025aa5` in `0x10025a00`, which only the
    fight takt calls. Neither was read further.
- **The sixth caller** is the gun handing its target to its round
  (`0x1002a514`).
- **The fight module runs only while `MBehaviour` flag `0x40` is set**
  (`Behavior.dll:0x100050c5`). Taking the hero over clears it
  ([above](#who-may-drive-a-units-guns--read)).
- **The player's target comes in sideways.** `iron3d.dll` never calls slot 16,
  which is why the search above missed it: it calls **interface `0x202` slot
  13** (`Control.dll:0x1002edf0`), which passes its (id, part) pair straight to
  the device interface's slot 16 and ignores its own index argument. A machine's
  control system installs that same method (vtable `0x1003d1fc`, constructor
  `0x100314c5`). The next section follows it.

This page once concluded from the search that nothing sets the hero's turret
target in first-person play, so its guided rounds fly straight. That was
wrong: the player always has a target while anything is on its radar
([25-sensors.md](25-sensors.md#the-players-target--read-and-measured)).

### The player's target reaches the turret — *read*

Whenever the player's target list sets a target (`iron3d.dll:0x10090b6a`,
`0x100915d8`), and when a unit changes hands (`0x10060679`, `0x100606b2`,
`0x100606eb`), `0x10091a80` hands it to the unit's turrets:

- **The unit record's weapon entries** (`+0x68`, 36 bytes each, built by
  `0x1009fe50` from `0x10075b25`) keep a gun's component index at `+4` and, at
  `+8`, the index of the turret listed before it (`0x100759d3`).
- **For each entry whose turret may take input** (bit 0 of the manual
  controller's slot 6, `0x10091aff`), it calls interface `0x202` slot 13 with
  the target's object id and part 0 (`0x10091b13`). So the turret gets
  `SetTarget` (`0x1002cb00`, `0x100280d0`), and the unit's seeker gets the same
  pair.
- **The turret relinks its guns** (`0x10028130`): each gun takes the target
  (`+0x108`) — **except an unguided gun on a turret whose state word is
  `0x200`**, `CIS_MANUALCONTROL`, which gets none (`0x10028164`; value 9 below
  zero marks it unguided, below). Every turret is built in `0x200`
  (`0x10027103`) and no turret record sets another (*measured*), and nothing
  on the player's side changes it. So on a player's unit **only the guided
  guns receive the target** (*derived*).
- **A relink restarts each gun's lock** (`0x1002a160`): state 2, the lock back to
  value 9.

**Lead is the AI's too** (*read*). Property `0x54` on a turret is its lead speed
`+0xa8` (`Control.dll:0x1002ea10`). The fight module copies a gun's round speed
into it (`Behavior.dll:0x10024ba2`), and a turret in `CIS_POINTTRACE` aims at
the point where a target moving at its velocity (property `0x27`) meets a round
at that speed (`Control.dll:0x10028640`). Nothing sets it for the player.

### A guided gun waits for a lock — *read*, and *measured*

**The gun fills values 8–10 from its round** when it links it, in the code that
also keeps the round's speed and gravity (`0x100297ef`):

- **Value 8, a range.** The round's range, its frame's `+108`, when that is
  positive (`0x100297e0`). With a seeker, the smaller of that and the seeker's
  reach, value 1 (`0x10029896`).
- **Value 10, a cone.** The cosine of the seeker's value 0 (`0x100298bc`), or −1
  with no seeker (`0x100298d5`).
- **Value 9, a lock.** The seeker's value 2 × 0.001, in seconds (`0x100298f1`),
  or −1 with no seeker (`0x10029907`). The gun's lock (`+0x170`) starts there.

**The gate** runs each time the gun would start a stroke, once its rounds,
capacitor and ready byte have passed
([The gun's takt](#the-guns-takt-a-stroke-then-the-interval)), and only while
value 8 is positive (`0x10029d3a`):

1. **No target.** With value 9 positive the gun reports state 2 and does not
   fire (`0x10029f60`). With value 9 at or below zero it fires as if there were
   no gate.
2. **Too far.** When the target's point (its part's where it names one,
   `0x1002a8c0`) is farther from the unit's position than value 8
   (`0x10029e28`, squared — value 8 is squared first unless it is `FLT_MAX`
   already, `0x10029dfc`): state 7, no shot, and the lock goes back to value 9
   (`0x10029e37`). **Report 7 is two refusals**: this one, and the ready byte
   clear before the gate is even reached (`0x10029d27`,
   [A gun with no follower is ready from birth](#a-gun-with-no-follower-is-ready-from-birth--read-and-measured)).
   With no part named, the point is **the centre of the target's node sphere**:
   interface `0x20` slot 3 asked with 2 and the all-zero request
   (`0x1002a95b`–`0x1002a97f`), the sphere
   [24-motion.md](24-motion.md#finding-the-ground--read) reads; only an object
   that does not answer `0x20` gives its matrix's position (`0x1002a9a5`). A
   tower's is its middle, not its foot on the ground.
3. **Off the barrel.** With value 10 positive, the barrel point's direction and
   the line from the unit to the target are normalised. A dot product no greater
   than value 10 gives state 8, no shot, and the lock back to value 9
   (`0x10029edd`). Otherwise the state is 1.
4. **The lock.** While value 9 and the lock are both positive, a gun in state 1
   takes the time since its last wake × 0.001 off the lock and does not fire
   yet (`0x10029f2c`). In any other state it does not fire either.
5. **The shot.** Past the gate the gun fires when its state word asks, and
   starting the stroke puts the lock back to value 9 (`0x10029fbb`).

So the lock counts down whether or not the button is held: it needs a ready
gun and a target in range and inside the cone. It counts only at the gun's
wakes, and a stroke's wakes continue the stroke before the gate
(`0x10029cb6`), so it does not count during a stroke (*derived*).

*Measured*, on the hero turret `o_tur_ht_02`:

| gun | round | range, m | seeker: cone rad, reach m, value 2 ms | value 8, m | value 10 | value 9, s |
|---|---|---:|---|---:|---:|---:|
| 1, cannon | `bb_h_01` | 500 | none | 500 | −1 | −1 |
| 2, plasma rifle | `bp_h_01` | 150 | 0.25, 500, 250 | 150 | 0.9689 | 0.25 |
| 3, laser | `bl_h_01` | 1,000 | none | 1,000 | −1 | −1 |
| 4, missiles | `bm_h_01` | 350 | 0.85, 500, 4,000 | 350 | 0.6600 | 4 |

So for the hero (*derived*):

- **The plasma rifle and the missiles do not fire without a target.** Nor do
  they fire at one more than 150 m or 350 m away, or more than 0.25 rad or
  0.85 rad off the barrel. The target is the player's
  ([25-sensors.md](25-sensors.md#the-players-target--read-and-measured)).
- **They need the target held**: 0.25 s before each plasma bolt and 4 s before
  each missile. The missile's lock outlasts its 1,250 ms interval, so a
  launcher held on a target fires about 4 s after each stroke ends.
- **The cannon and the laser never meet the gate.** Their relink gives them no
  target ([above](#the-players-target-reaches-the-turret--read)), and an
  unguided gun with no target fires.

**What the HUD reads of the lock** (*read*, and *measured*). The gun's slot 10
(`0x10029be0`) keeps a share at `+0x17c`, and the time driver calls it every
frame (`0x1002d317`):

- in report 1 the share is 1 − lock left ÷ value 9, or 1 with value 9 not
  positive;
- in report 4 it is the wait's progress;
- otherwise it is left alone;
- it is held to 0..1.

Interface `0x202` slot 10 hands it out as property `0xf00`
(`0x1002eaa0`). Linking a round copies the 27-word block of the round's frame,
from its `+20`, into the gun's `+0x9c` (`0x100297a4`). Property `0x64` hands
that block out (`0x1002e60b`). The HUD reads its `+0x60`, which is the frame's
`+116`, and draws a lock only for the value 16. *Measured*: `+116` reads 16 on
exactly the 20 rounds of `weapon.rlb` that carry a class-17 seeker, and on no other
round. What
the HUD draws is [35-hud.md](35-hud.md#the-guided-lock--read)'s.

*Measured* over the 66 rounds in `weapon.rlb`: every one has a range, so every
gun that loads one keeps a range gate. 17 of the 20 seekers carry a lock.
The guns of `ba_b_04`, `ba_b_05` and `ba_m_04` (value 2 of 0) fire untargeted.

### How a round ends — *read*, and *measured*

A round's controller names three groups in its 84-byte block: entry 2 runs on
a hit (`+0x4e4`), entry 3 at the map edge (`+0x4e8`) and entry 4 at the end of
its range (`+0x4ec`). On all 66 rounds (*measured*):

| group | action | what it does | rounds |
|---|---|---|---|
| hit | 17 (`0x100033d0`) | clears invulnerability and **kills the round** through `ILifeSystem`, so its own `.ndp` explosion goes off where it stopped | 63 |
| hit | 15 (`0x10003341`) | marks it dead and takes it out of the collision pass, with no explosion | the 3 builder beams |
| map edge | 15 | the same | all 66 |
| range | 27 (`0x100030ce`) | **explodes it with a named `.exp`**: `<round>_end.exp` on bullets, beams and shells, `<round>r.exp` on rockets and missiles | 58 |
| range | 17 or 15 | as above | the animal shots and builder beams |

Each group also starts with action 0, which stops the body (`0x10002926`).
Where a round carries a tracer, the group stops it (action 19) or starts a
hit effect (action 10). The hero's missile explodes at range with
`bm_h_01r.exp`, 200 in 10 m. Its hit `.exp` is 170 in 7 m. The other three
hero rounds hit directly (*measured*).

**A range is the path flown** (*read*). Each tick the round's remaining range
(`+0x4c8`, from `.ctl` +108) is compared with the length it moved that tick,
the distance from where it began the tick to where it is now (`0x1000cfc6`):

- **A longer move** is cut to what remains, and `+0x664` is set
  (`0x1000cfd0`–`0x1000d069`), so block entry 4 runs next.
- **Either way** the length comes off the remaining range (`0x1000d070`).
- **A guided round that turns** spends its range along its curve, not as
  distance from the muzzle (*derived*).

**Nothing else changes over the flight.** A round hits for its `.ndp` hit
points plus its `.exp` damage, times the level ratio
([26-damage.md](26-damage.md#a-hit-from-the-round-to-the-node--read)), and no
term in that is distance. So a round hits as hard at the end of its range as
at the muzzle, and not at all beyond it (*derived*).

**The hero's four rounds** (*measured*):

| gun | round | m/s | range, m | at range | what it plays |
|---|---|---:|---:|---|---|
| cannon | `bb_h_01` | 350 | 500 | `bb_h_01_end.exp`, kind 2, 1 | `bb_h_01_end` |
| plasma rifle | `bp_h_01` | 150 | 150 | `bp_h_01_end.exp`, kind 2, 1 | `pls_h_end` |
| red laser | `bl_h_01` | 10,000 | 1,000 | `bl_h_01_end.exp`, kind 2, 1 | `bl_h_01_end` |
| missile | `bm_h_01` | 70 | 350 | `bm_h_01r.exp`, kind 3, 200 in 10 m | `exp_m_mis` |

- **The three puffs are one effect**: 1.5 s of two type-7 smoke bursts
  (`smoke_g`, `smoke_g_add`, bit 8 set) and two type-4 `glow_eng` sprites,
  under the `Smoke` settings switch
  ([11-effects.md](11-effects.md#which-effects-run-the-settings-switch--read-and-measured)).
- **The missile's is a blast**: 3 s of `expl5`, `fire_smoke_w`, a light and
  `hit_mis_metal.wav`, under `Explode`.
- **Where and how big.** All four `.exp` carry placement 0. Action 27 plays
  them through node 0's damage stage, so each goes off at the round's
  bounding-sphere centre along its second axis, scaled by the `.exp` radius.
  That axis is the flight direction, since a round is created facing it
  (*derived*)
  ([11-effects.md](11-effects.md#what-an-explosion-plays--read-and-measured)).
- **A puff strikes nothing** (*derived*). A kind-2 hit takes its node from
  the round's contact record, and a round that ran out of range has none.
- **A round that leaves the map box first** is removed with no puff. That
  includes the box's doubled top, which is 2 × the land's highest point
  ([26-damage.md](26-damage.md#the-hit-test--read-and-measured)).

### A beam outlives its round — *read*, and *measured*

**None of the three groups deletes the round at once** (*read*):

- **Action 15** (`0x10003341`) marks the owner word `0xfffe` and, unless the
  agent is a building or carries `+0x104` bit `0x10000000`, sets the death time
  `+0x59c` to the clock `+0xe4` plus the controller's `+92` (`+0x4b8`,
  `0x10003375`–`0x1000339b`), then sends the object message `0x15` with 7.
  That is the same death time a dead unit gets
  ([26-damage.md](26-damage.md#what-a-damaged-node-a-destroyed-part-and-a-dead-unit-draw--read-and-measured)).
- **Action 17** kills the round through `ILifeSystem`, which takes the same path.
- **Action 27** (`0x100030ce`) swaps node v4's explosion for the named `.exp`, deals
  that node minus the life at `+0x590` (`0x1000330d`–`0x10003319`, through
  `0x10010f30`), runs its stage, and puts the old explosion back. Node 0 is out
  of life, so the round is dead.

Each way, **the round stays in the world for its controller's `+92` ms**, with its
body stopped by action 0, until the control tick calls `KillGameObject`. Its
effect manager, and every effect on it, goes with it.

**Every beam round restarts its bolt as it stops** (*measured*):

- 23 rounds in `weapon.rlb` create an effect with a type-5 bolt at load: the 16
  lasers (`bl_*`), the 4 tasers (`bt_*`) and the 3 builder beams (`bld_*`).
- On all 23, the hit, edge and range groups each run action 10 on effect id 0,
  that bolt, in time mode 1, before their 17, 15 or 27.
- The lasers' and tasers' bolt effects last **0.75 s** and their `+92` is 3000.
  The builder beams' `builder_tail` lasts 10 s and their `+92` is 11000.
- The cannon, the plasma rifle and the other rounds stop their tracers with
  action 19 instead.

**`hero_laser_bullet`**, the hero's laser round's bolt effect (*measured*):

- header time mode 0, duration 0.75 s, flags `0x1010`: keep running while the
  attach point is hidden, and take the manager's target point;
- two bolts over the window 0 to 1, fading 1 → 0: `NE_Laser_R`, widths +24 and +28
  of 0.4, and `NE_Laser_Y`, 0.1;
- both materials add (flags 8) the grey, alpha-striped `LASER.0` with a black
  diffuse, `NE_Laser_R` under an ambient of (255, 26, 26) and `NE_Laser_Y`
  under (255, 255, 0).

**Where a beam starts** (*read*):

- As the round is made, the gun asks it for interface `0x13`, its effect manager,
  and calls slot `0x44` (`0x1002a517`–`0x1002a56d`). It passes the pair (the
  shooter's id, node 0), 2, and the muzzle's world point.
- The manager (`0x10004c50`) keeps the pair at `+0x94`/`+0x98`. It looks the
  shooter up, takes that node's matrix (`AniMesh` interface `0xb`, slot `0x10`),
  and stores the point in the node's frame at `+0x9c`
  (`[0x1001e0bc]` slot `0x84`).
- On every manager tick (`0x10003d6c`–`0x10003e34`) the manager takes the node's
  matrix again and carries the point back into the world (slot `0x18`). It then
  hands the point to every instance that asked for it (`0x10007b60`), and a bolt
  takes it as its start (`0x10003070`).

So **a beam runs from the muzzle to where the round is**:

- the muzzle end rides on the shooter's body, and the far end stays where the
  round stopped;
- the beam is drawn at full strength while the round flies, since mode 0's
  time is 0 until something sets it;
- once the round stops, action 10 runs the bolt's time over 0.75 s, and its
  fade falls straight from 1 to 0 (`0x10002dd4`);
- the bolt's effect has no flag 2, so it holds at the fade's 0, which draws
  nothing (`Terrain.dll:0x1002887e`), until the round goes 3 s after it stopped.

A laser round crosses its 1,000 m in a tenth of a second, so the beam stands for the
flight and **0.75 s after it, fading** (*derived*). The manager updates an instance
only every 100 ms ([11-effects.md](11-effects.md#how-an-effect-runs--read)), so the
fade steps and its last step can land up to 0.1 s later.

**Measured** on the recording of Mission 01 (30 fps, the game drawing about 20):

- The hero fires the cannon and the laser together from 132 s. The laser's muzzle
  glow shows every 0.47 to 0.5 s: a 250 ms stroke and 200 ms of wait.
- Consecutive beams overlap, so the beam never goes out while the button is held.
- The last glow is at 135.07 s, and the round leaves 125 ms into the stroke,
  about 135.19 s.
- Measured on its middle, the beam stays at the brightest the video holds until
  135.73 s, dims, and is gone by 135.97 s. That is 0.78 s after the round
  left, and 0.9 s after the glow.
- Over the lavender sky it is pink with a white core. The only red anywhere in
  its textures and materials is `NE_Laser_R`'s ambient, so the ambient colours it
  (*seen*).

### What a shot plays — *read*, and *measured*

- **Guns with a shot group.** 34 guns name a section-5 group at record +0xc.
  It runs when a barrel starts its stroke and holds only actions 10 (start an
  effect), 4 (create one) or 10 with 11 (*measured*). An example is the
  builder's `gunf_builder`.
- **The hero's guns name none.**
  - The turret's load group creates `hero_cannon`, `hero_prifle` and
    `hero_redlaser` at the barrel points, and the `*_sfx` effects at the
    `GH_*_sfx` points (action 4, `0x10002a8d`).
  - It binds the first three to the barrel nodes and the sound effects to the
    arm nodes (action 14, `0x10003031`).
- **Nothing starts them. They follow a node** (*read*):
  - **Action 14's second argument is a node** of the model, rebased by the
    part's first node (`0x10003053`), not a control point.
  - **An effect in time mode 4 takes its time from that node's phase**, through
    the owner's IAnimation slot 9 (`Effect.dll:0x10005d3f`,
    `AniMesh.dll:0x10005600`).
  - **The phase is the value of the channel that plays the node**: channels hand
    it over through IAnimation slot 10 each step (`Control.dll:0x10021a30`).
- **The gun effects follow the barrels** (*measured*). `hero_cannon`,
  `hero_prifle` and `hero_redlaser` are bound to nodes 13, 9 and 22, the three
  barrels. Each barrel's value is 0 at rest, 0.5 as the round leaves, then 1,
  then 0 again.
  - `hero_cannon` shows one sprite set from 0.01 to 0.5 and another from 0.51
    to 1, with its smoke and `H_fire_cannon.wav` from 0.01 to 1.
  - `hero_redlaser` and `hero_prifle` sound `H_fire_*.wav` from 0.25 to 1.
  - So the flash and the report play through each stroke, and on no other
    frame.
- **The `_sfx` follow the arms** (*measured*). The four are bound to nodes 11,
  7, 19 and 15, the first channel of each arm, and each is one `H_gh_*.wav`
  audible from 2 to 20 m, a one-shot with its trigger at 0.15. They are the
  arms' own sounds, not the shots'.
- **An arm sounds both ways** (*derived*). The arm channels carry no flag, so
  the node's value is the channel's: 0 folded, exactly 1 out, since an arm's
  progress ends on its end value and its channels head for it
  ([above](#the-button-reaches-the-selected-guns)). A one-shot sound plays as
  its time crosses the trigger going up, and takes a time of exactly 1 as 0
  ([11-effects.md](11-effects.md#type-2-is-a-sound--read-and-measured)). So
  `H_gh_*.wav` plays **as a gun is selected**, when its arm passes 0.15 on the
  way out, and **again as it is deselected**, on the first update after the
  arm leaves 1. An arm folded before it was all the way out stays silent.
- **Selecting a gun also calls the game** (*read*): after the arm's state 1,
  `World3D.dll`'s toggle calls the callback its host handed it with (0, `0xe`)
  (`0x10010805`, the pointer at `0x1013b59c`, set at `0x10013ec2`).
  Deselecting does not. What the host does with `0xe` is not traced;
  `ui/game_resources.cfg` binds no weapon-selection sound (*measured*).
- **The missiles** have no barrel effect. Their launch is the round's own
  `hero_gunfire_missile` (mode 1, 5 s, `H_fire_missile.wav` in its first
  tenth).
- **The rounds' own effects** start when the round loads: `hero_cannon_bullet`,
  `hero_prifle_bulletA`/`B`, `hero_laser_bullet`, and the missile's engine,
  smoke and `hero_gunfire_missile`.
- How a sound emitter behaves across its window, once or looping, is
  [read in 11-effects.md](11-effects.md#type-2-is-a-sound--read-and-measured).

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
| Blast, m | the round's blast reach (`0x100136c0`): node 0's explosion radius, raised to each other node's distance from node 0 plus its radius, whatever the explosion's kind — a direct hit's 1 m shows as 1.0 ([38-designs.md](38-designs.md#a-parts-box--read)) |
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

| round | cone, rad | follows within, m | turns, rad/s | lock, ms (value 2) |
|---|---:|---:|---:|---:|
| missiles, tiny to large (`bm_t/l/m/b_01`) | 0.80–0.95 | 500 | 1.2–1.6 | 3,000–5,000 |
| huge missile (`fm_h_01`) | 1.57 | 700 | 1.3–1.4 | 3,500 |
| winged SSMs (`bm_m/b_04`) | 0.70 | 500 | 0.5 | 7,000–10,000 |
| howitzer shells (`bb_m/b_02`) | 0.26–0.27 | 350–400 | 0.35–0.40 | 500–750 |
| rockets (`br_*`, `fr_l_01`) | no seeker | — | 1.52, unused | — |

*Measured*: every missile round looks wider and turns faster than both guided
shells. A howitzer shell corrects gently inside a 15° cone; a missile looks
through about three times the angle and turns three to four times as fast; the
winged SSMs look wide but turn slowly.

**The steering** (*read*, `0x1000cd0a`):

- **Two angles.** The seeker's heading, in the round's frame, becomes the angle
  of its z against its y and of its x against its y (`0x1000cd6d`).
- **±π/2 when the target is abeam.** When the forward part is 0 the angle is a
  quarter turn, signed; `0x100430d4` holds π × 0.5, set at load (`0x1000d9f0`).
  That is all the "scale on the steering command" is.
- **One tick.** Each angle is divided by the tick (× −1000 ÷ ms,
  `0x1000cd7d`) and clamped to the turn rate. A round asks to swing onto its
  target within one tick, and the turn rate is what slows it.

**Value 2 is the gun's lock**, in ms. The seeker's own methods read value 0
(`0x100247a0`) and value 1 (`0x100248d6`), and no request for value `0x302`
is aimed at a seeker. That scan once made this page call value 2 unread.
But the gun asks the round's device getter for ids 10, 11 and 12 when it loads
the round, and those are the seeker's values 1, 0 and 2 (`0x1002b738`,
`0x1002b75e`, `0x1002b784`). Value 2 becomes the gun's value 9
([A guided gun waits for a lock](#a-guided-gun-waits-for-a-lock--read-and-measured)).

A seeker's target is the gun's target, handed to the round as it leaves
([The round's start](#the-rounds-start)). A turret's gun has its turret's
target, and a unit the player drives has the player's: the target list is
that unit's (`iron3d.dll:0x10091a80`). **The seeker steers at the target's
node sphere's centre**, asked as the gate asks it (`Control.dll:0x100248b6`).
A winged SSM (`bm_m_04`: mode 0, 45 m/s) flies straight at that point, so a
crest between it and a tower on a hill takes the missile.

## How the AI fires — *read*

`Behavior.dll`'s **fight module** (`0x10023ff0`) runs from the behaviour takt
while `MBehaviour` flag `0x40` is set. It never picks one weapon. It aims
every turret and lets every gun fire when that gun's own score allows.

**The target.** The module picks one target for the unit on a timer, by the
selector's mode of [25-sensors.md](25-sensors.md): none, fixed, nearest,
units only, or weighted by the areal figure (`0x100240ae`).

**Each turret** in the module's table (`MBehaviour+0x64c`: 52 bytes a turret,
88 a gun) is aimed at it (`0x10024b1b`–`0x10024c51`):

- the unit's turret target is set (interface `0x204` slot 16);
- the turret's lead speed is set to **its fastest gun's** round speed
  (property `0x54`, [below](#the-lead-is-the-turrets-fastest-gun--read-and-measured));
- the turret is put in `CIS_POINTTRACE` (`0x400`).

**Each gun is scored.** Its record (`0x1001b4b0`) keeps its round's damage
(property 6), its round's speed *v* (property `0x54`), its magazine (property
`0x800`), and the round frame's flags and range. Then:

- **Heavy rounds are held back.** A gun whose round does **more** than 10,000
  damage (property 6 against `0x1005994c`, `0x10024d02`–`0x10024d2e`) fires only at
  a target whose id has 3 in its `0x0f000000` nibble (`0x10024d30`–`0x10024d3e`).
  - **The nibble is the object id's class** (*read*). `iron3d.dll`'s game-message
    handler switches on `(id >> 24) & 0xf` (`0x1006016b`, `0x100606fe`) and hands
    class 3 to the level's **building** list `+0x71c` and class 4 to its **unit**
    list `+0x720` — the two lists [42-selection.md](42-selection.md#not-established)
    and [36-factory.md](36-factory.md) select from. `World3D.dll:0x10001853` filters
    an object list by the same field, its flag `0x40` admitting class 3 and `0x20`
    class 4. So **the AI fires a winged SSM only at a building** (*derived*), which
    is what a 45–60 m blast at 700 m is for.
  - **The bar separates cleanly** (*measured*): 3 of the 62 `e_gun_*` records with
    a round do 10,000 or more — `e_gun_bl_17` (`bm_b_04`, 100,000) and
    `e_gun_bl_18` and `e_gun_ml_18` (`bm_m_04`, 60,000), the three winged SSM
    launchers — and the next gun down, the huge missile launcher's `fm_h_01`,
    does 3,000.
  - **The nibble is in no shipped file** (*measured*): over the 864 objects the 33
    missions place, the logical id's nibble is 0 on the 463 owned ones and `0xf`
    on the 401 scenery, never 3. It is a runtime id, minted as the object is
    registered.
- **Distance** (`0x1001b9f0`). **A round whose frame flags (`.ctl` `+116`, the
  record's `+0x34`) carry bit `0x10` scores 1.1, and bit 8 scores 1.0, wherever the
  target stands.** Any other rises from 0 to 1 over the first 5 m (0 m for an
  animal, `+0x44`), holds 1 out to (*v* + 1) ÷ 2, and falls to 0 at 2 (*v* + 1),
  then is multiplied by 1 − height ÷ *v*.
  - ***v* is the round's top speed**, `.ctl` `+48`: property `0x54` is interface
    `0x202` slot 3 (`Control.dll:0x1002e580`, case at `0x1002e5c6`), which answers
    a class-2 or class-30 gun's `+0x94`, set to `+0xb8` by the gun's link
    (`0x100297f5`), the probe round's frame (`.ctl` `+20`…`+127`, property `0x11`,
    `0x1000dd52`) being copied to the gun's `+0x9c`.
  - *Measured* over the 66 round controllers, whose `+116` is 0, 4, 12 or 16: 12
    on the four lobbed `bf_*_01` (score 1.0); 16 on `bm_*`, `bp_*`, `ba_b_04/05`,
    `ba_m_04/05`, `bb_b_02`, `bb_m_02` and `fm_h_01` (1.1); 4 on `bb_*_01`, `bl_*`,
    `bld_*`, `bt_*`, `ba_a_*` and `rg_b_01`, and 0 on `br_*` and `fr_l_01` (the
    band). So the Small Bunker's `bf_f_01` (*v* 45) scores 1 at any range, while
    the Small Tower's `bb_f_01` — its controller is `bb_f_02.ctl`, *v* 200 — holds 1
    to 100.5 m and scores nothing past 402 (*derived*: it clears a building's 0.85
    within about 146 m).
- **Aim.** It is multiplied by two more factors, one for the turret and one for
  the gun, each `1 − θ × d ÷ R` (`0x10024cea`, `0x10024db4`):
  - *d* is the distance from the unit to the target (`0x10024377`).
  - *R* is the target's **outer radius**: its variable `0x206`, which
    `MBehaviour` answers from `+0x688` (`0x1000a79b`). That is the distance from
    the target's origin to the centre of the sphere its mesh interface gives, plus
    the sphere's radius (`0x1000648c`, `0x1000cbe3`). With no target *R* is 5
    (`0x1002411c`). A target reporting 0 is asked to recompute and read again
    (`0x10024452`).
  - *θ* comes from interface `0x202` slot 10 (`Control.dll:0x1002eaa0`, called
    at `0x10024c92` for the turret and `0x10024d54` for the gun). The slot
    writes the component's property `0xf00` and returns a word by class:

    | class | word returned | property `0xf00` |
    |---|---|---|
    | turret (1) | `+0xec`, its aim stage | computed by `0x10028bb0` (`0x1002c142`) |
    | gun (2), builder (30) | `+0x11c`, its report | `+0x17c`, 1 − lock left ÷ value 9 (`0x10029c1c`) |
    | door (12), class 11 | `+0x9c` | |
    | any other | 0 | |

    The turret's stage is set to 3 when it is given a target (`0x1002811c`), 2
    at `0x1002864d`, 1 at `0x10028730`, and 0 when its aim reaches 1
    (`0x10027aee`). The gun's takt stores report codes 2 to 8 at other points
    (`0x100295ba`–`0x1002a167`); their meanings were not listed here. The
    module turns the word into *θ*: 0 gives 0; 1 gives (1 − property) × π;
    anything else gives π. When the firing unit is an animal, the gun's factor
    is 1 (`0x10024d9e`).
- **Threshold** (`0x10024e7a`–`0x10024ec5`):
  - 0.45 for a unit whose chassis profile can fly (`0x10014670`, `+0xc`);
  - otherwise 0.85 when `MBehaviour+0x614` is **0.5 or more** or the unit's Type
    carries `0x80000000`, a building, and 0.45 when not. The compare
    (`0x10024e8f`–`0x10024ea0`, the 0.5 at `0x100595c0`) goes to 0.85 unless the
    speed is below 0.5, so exactly 0.5 takes the high bar.

  **The score must pass the bar, not reach it** (`0x10024f07`–`0x10024f1e`): the
  product of the three factors is compared with it, and only a greater one goes
  on to the shot.

  **`MBehaviour+0x614` is the unit's live forward top speed**, in m/s — *read*.
  The fight module sits at `MBehaviour+0x5fc`, and its refresh copies six live
  figures into `+0x5fc`–`+0x610` and the same six again into `+0x614`–`+0x628`
  (`0x1001bbe0`, from `0x1001c1a0`). They come from interface `0xa` slot 13 with id
  `0x12`, the **live** parameter block at control `+0x470` (`Control.dll:0x1000dd6f`;
  id `0x11` is the authored one at `+0x46c`), and `+0x614` is that block's `+0x1c`,
  which is file `.ctl` +48, the forward component of the top-speed triple
  ([24-motion.md](24-motion.md#how-the-ai-asks-for-speed--read)). That is the
  limited speed — authored × the gear × the engine's drive and condition × the load —
  not the authored one. The engine's debug line prints it as `ns`
  (`'%X %s%s ns%2.1f bs%2.1f li%d cl%d …'`, `0x1000d2f4`), and the unit takt uses
  the same 0.5 to decide whether to drive the unit at all
  (`0x100051b0`, [24-motion.md](24-motion.md#what-sets-the-live-limits--read)).

  So **the bar is 0.5 m/s**, and a machine shot down to a crawl starts firing on
  half the aim a healthy one needs. A building has no speed at all, which is why
  its Type is tested apart.

  *Measured*: 19 of the 382 shipped assemblies with a root controller are
  **authored** below the bar, all at 0.2 m/s, and they are exactly the fixed gun
  towers and the two practice targets. Those are units, not buildings, so nothing
  lifts their bar: **an AI tower fires at 0.45 where a warbot needs 0.85.** Every
  other assembly is authored above 1 m/s, so only damage brings one under.
  Placed, they are 33 of the 296 units the mission files put down: 5 practice
  targets on Mission 01, which carry no gun, and 28 towers with one — `12tower` on
  C01 M03, `23tower` on C02 M02, `31towcan` and `31towm` on C03 M01, `33tower` on
  C03 M03, the seven `33tow*` on C03 M04, `42towm` on C04 M02 and `m7_tow` on
  Multi.05 (*measured*). The bar only counts outside the go, attack and search
  tasks, whose free fire drops it to 0, so what the low bar changes is how far a
  tower fires while it is not attacking: the Small Tower's `bb_f_01` (*v* 200)
  scores 0.45 out to about 266 m where 0.85 stops at 146 m (*derived*).

  **A building's guns take 0.85, and the Small Bunker's flamers fire on a
  hovering flyer** (*read*, and *derived*). A building takes no task of its own
  (`0x100179c0` refuses one), so unless a script orders it to attack it is not
  in free fire, and the bar is what it fires by. Its Type gives it 0.85. The
  flame, `bf_f_01`, carries frame flags 12, and bit 8 returns a distance score of
  1 before the height term is reached (`0x1001ba00`–`0x1001ba0a`): its score does
  not fall with the target's height at all. So once the bunker's turret has settled on a target and its gun
  reports 0, the score is 1 × 1 × 1, which passes 0.85 at any range and any
  height the turret can look at. The Small Bunker's sight looks from −15° to 75°
  (*measured*, its pitch channel's two ends); a flyer's walk points are 15 m over
  the ground, which 80 m and 40 m out from the Mission 03 bunker stands 3.5° and
  2.1° above its sight, the bunker's own sight standing high. The engine holds to
  this, and its test puts the enemy's flyer there: the player's bunker traces it
  and fires 4 and 5 flames in 6 s, and 4 with the flyer 40 m up and 20.5° over
  its sight. **So the bunker fights flyers as well as walkers**; what keeps its
  fire off a target is its turret not having settled, not the target's height.
  An earlier line in the queue had the walker's 0.85 limit the bunker to targets
  near its own ground level. That holds only for a round without bit 8 or
  `0x10`, which loses 1 − height ÷ *v* of its score, and neither of the bunker's
  guns fires one.

  Past the bar, the gun fires **one shot** (`CIS_SINGLEFIGHT`, `0x200`,
  `0x10024fa2`). Before that it must have both factors above 0, a clear line
  (`0x10025c60`, `0x10024981`) and its timer run out. An animal instead needs
  the dot product at `0x10024330` above 0.85 (`0x10024f3f`).
- **Its own timer.** The shot also waits for the gun's randomised timer: 30 ÷
  magazine s plus up to as much again when the magazine holds more than 2,
  otherwise 0.5 s plus up to 1.5. Both are divided by the difficulty profile's
  value (`0x1001b5ec`, `0x1001b650`).
- **During the go, attack and search tasks** every gun fires on its timer,
  line permitting, whatever its aim (`0x10024e58`). All three factors are
  forced to 0.5 and the bar to 0. The task numbers 2, 3 and 5 are what these
  tasks' slot 0 returns: go `0x10035710`, attack `0x10035950`, search
  `0x100359e0`. This holds only while the record `0x10014bd0` fills has an id
  at `+0x30`, which was not traced.

So a unit fires every gun whose score clears the bar, each on its own timer.

*Derived* from the aim factors: once *d* is more than *R* ÷ π, *θ* = π
drives a factor below 0. For a hero-sized target (*R* about 2) that is 0.7 m.
So **outside the go, attack and search tasks, an AI gun fires only when
both of these hold**:

- its turret has settled (stage 0), or has almost finished aiming;
- the gun's report is 0, or 1 with its property at 1 (for a guided gun, its
  lock has run out).

A turret still turning at 100 m to a hero needs its aim more than 99.3% of the
way there. Every aiming pass sets the turret to `0x400` (`0x10024c51`), so an
AI turret is out of the `0x200` in which its relink withholds a target
(`Control.dll:0x10028164`). **An AI unit's unguided guns therefore get the
target, and keep their range gate.**

*Derived*, from the distance score alone: a laser or taser (10,000 m/s)
scores fully out to 5,000 m, a 350 m/s cannon to 175 m, and a 70 m/s missile
only to 35 m and not at all beyond 142 m. `weapons.ai_distance_score` and
`weapons.ai_fire_wait` compute them.

### The lead is the turret's fastest gun — *read*, and *measured*

The fight module's table is two vectors deep. **A turret record is 52 bytes**
(`0x1002522c`) and a **gun record 88** (`0x1002520b`); the turret keeps the turret
component at `+4`, its own vector of gun records at `+8` (count `+0xc`, data `+0x14`)
and, at **`+0x1c`, the index of one of them**. The builder (`0x1001be10`) files each
class-1 component as a new turret record and each class-2 or class-30 component
after it into **the last turret's** list (`0x1001bf82`–`0x1001bfcf`), and sets
`+0x1c` to 0 (`0x1001bf6a`).

**`+0x1c` is the fastest gun.** The module's refresh (`0x1001c1a0`, called from the
unit takt at `0x10005037`, from `MBehaviour`'s variable and debug paths at
`0x1000a7c5` and `0x1000d0ab`, and from the go and attack tasks at `0x1003be2f` and
`0x1003c2b1`) zeroes `+0x1c` and a running best (`0x1001c2a9`, `0x1001c2b0`) and
then, for each gun in turn, keeps the index whose record `+0x14` is **strictly
greater** (`0x1001c37d`–`0x1001c398`). That `+0x14` is property `0x54`, the round's top
speed, read when the record is filled (`0x1001b52b`, `0x1001b545`). The aiming pass
then reads gun `+0x1c`'s component (`0x10024b5c`–`0x10024b6a`), asks it for property
`0x54` and writes that to the turret's own (`0x10024ba2`).

- **Those are the only two writers.** `+0x1c` is written in exactly two places in
  `Behavior.dll`, the builder's 0 and the refresh's pick. A **third copy of the same
  loop is dead code**: `0x1001bb30` is a standalone picker over the same field that
  nothing calls — no `call` or `jmp` reaches it and its address appears nowhere in
  the file, in a vtable or otherwise.
- **So a turret leads for one gun and misleads for the rest** (*derived*). A turret
  in `CIS_POINTTRACE` aims at where a target moving at its velocity meets a round at
  the lead speed ([The player's target reaches the turret](#the-players-target-reaches-the-turret--read)),
  and every gun on that turret fires down the same barrel line.

*Measured* over the 458 assemblies: **407 turret records**, 312 of them carrying two
guns or more, and on **160** the guns' round speeds differ. On **85** of those 160
the fastest is not even the first gun fitted. **91 of the 160 are led by a 10,000 m/s
beam** — a laser or a taser — and a lead speed of 10,000 is no lead at all: the
turret aims where the target stands, so the missiles beside the laser are fired with
no allowance whatever. The mixed turrets' fastest-over-slowest runs 1.06 to 285.7,
median 111.1.

## The weapons the player builds — *measured*, with *derived* rates

Damage a round is the panel's; damage a second is `damage × shots a second`,
× the beams on a salvo gun. It ignores armour, shields and the level ratio
([26-damage.md](26-damage.md)). *Shots a second* is the stat panel's `1000 ÷ max(1,
value 3)`; a gun also waits out its barrel's stroke first, so it fires more slowly than
this ([Firing, from button to round](#firing-from-button-to-round--read-and-measured)).

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

## The two guns nothing carries — *measured*

`objects.rlb` holds 71 `e_gun_*` records and **61 are fitted** on at least one
of the 458 `UNITS/**/*.dat`. `e_gun_bl_03` and `e_gun_tl_02` are fitted on
**none**, and nothing else in the install names them either:

- They are two of the four `e_gun_*` absent both from the 395-entry part list
  every `.trf` carries — the same 395 entries in all 29 trees — and from
  `objects.dlb`. The other two are `e_gun_ll_11` and `e_gun_ml_16`, whose
  controllers are empty outright. **The catalogue cannot offer what the
  research tree does not list** ([38-designs.md](38-designs.md#the-catalogue--read-and-measured)).
- Their controllers (`guns.rlb/o_gun_ba_03.ctl`, `o_gun_ta_02.ctl`) hold one
  channel each — node 1, flags `0x8`, rate 1.5, frames 1 → 3, control point −1 —
  and **no components at all**.
- Their `.cpt` is 4 bytes, one empty stream and no control point. Only those
  four guns and the five `e_gun_fs_*` fortification builder modules, which have
  no gun to point, have one that small. **A gun with no muzzle point could not
  fire even if it had a gun component**
  ([Where the round leaves](#where-the-round-leaves-and-which-way)).

**So the unpaired follower is never reached.** Summed over the merged control
system of each of the 458 assemblies, **1,015 followers and not one without a
gun**; no assembly has more followers than guns. They are art left in the
archive after the gun was taken out.

*Control*: the same sweep finds `e_gun_bl_15` fitted 20 times, `e_gun_bc_06`
28 and `e_gun_tl_04` exactly **once** — a single fitting is visible, so a zero
means zero. And the part list all 29 trees share names `e_gun_bs_01`, which is
assembled nowhere, so "in the tree" and "assembled" are detected apart.

## Not established

- ~~Whether a target the hero's AI set before the player took over survives
  into first-person play.~~ Moot: the player's target list sets the turret's
  target whenever the player's target changes
  ([The player's target reaches the turret](#the-players-target-reaches-the-turret--read)).
- ~~Whether the landscape is one of the objects IWorld slot 7 walks, so that the
  sight ray converges on the ground and not only on objects.~~ **Answered**: it
  is the **root** the walk starts from, class 1, and its interface `0x18` slot 6
  is `GetFirstIntersectedFace`; the ray excludes no face, so unlike a round it
  stops on a lake's surface
  ([Where the round leaves, and which way](#where-the-round-leaves-and-which-way)).
- ~~The vector a falling round's mount solves for.~~ The traced point less
  `TurretCenter`'s position; a turret in `CIS_MANUALCONTROL` gets no lift and its
  gun is ready. `TurretCenter`'s vector is (0, −1, 0) on node 0 in 57 of 59 turret
  `.cpt`, so on an upright turret the lift is upward (*measured*). ~~Still open: how
  a lobbed round's gun fitted as a part on a turret with no follower channel, the
  Small Bunker's `e_gun_fc_08` on `e_bnt_lt_01`, is raised; the gun's gate (report
  7, `0x10029d27`, `0x10029e37`) refuses a target past value 8, `.ctl` `+108`.~~
  **The case does not arise**: a gun fitted as a part brings its own follower, so
  the Small Bunker's flamers are mounted and lifted like any other, and of the 19
  shipped guns that do hang on no follower none fires a lobbed round
  ([A gun with no follower is ready from birth](#a-gun-with-no-follower-is-ready-from-birth--read-and-measured)).
  Report 7 is two refusals, not one: `0x10029d27` is the ready byte and
  `0x10029e37` the range, which is value 8, the round's `.ctl` `+108`, already read
  in [A guided gun waits for a lock](#a-guided-gun-waits-for-a-lock--read-and-measured).
- ~~`e_gun_bl_03` and `e_gun_tl_02` carry a follower and no gun. The turret's
  takt reads the paired gun without a check, so either they are never fitted or
  the follower pairs with a later part's gun~~ — **measured**: never fitted, and
  absent from every research tree and from `objects.dlb`
  ([The two guns nothing carries](#the-two-guns-nothing-carries--measured)).
- ~~The fight module's two aim factors (interface `0x202` slot 10), and what
  tasks 2, 3 and 5 are.~~ Answered: `1 − θ × d ÷ R` from the turret's aim stage
  and the gun's report; the tasks are go, attack and search
  ([How the AI fires](#how-the-ai-fires--read)).
- ~~Which of a turret's guns lends its round speed as the lead (the turret
  record's `+0x1c`), what `MBehaviour+0x614` measures against the 0.5 of the
  threshold, and the target id's nibble 3 that frees the winged SSMs.~~ All
  three answered. The lead is **the fastest** gun's: the refresh keeps the index
  of the strictly greatest round speed and a standalone picker over the same
  field (`0x1001bb30`) is dead code
  ([The lead is the turret's fastest gun](#the-lead-is-the-turrets-fastest-gun--read-and-measured)).
  `+0x614` is the unit's **live forward top speed**, so the bar is 0.5 m/s, and
  19 shipped assemblies — the fixed towers and the targets — are authored under
  it ([How the AI fires](#how-the-ai-fires--read)). The nibble is the object
  id's **class**, 3 being a building and 4 a unit, so a winged SSM is held for
  buildings ([above](#how-the-ai-fires--read)). Which object answers class 2,
  the one bit an outer camera's query drops from a round's, is still open
  ([30-turrets.md](30-turrets.md#not-established)).
- ~~Which fight-module bar a building's guns must clear. With the walker's 0.85
  the Small Bunker's flamers only fire at a unit close to its own ground level,
  never at hovering flyers, so the warbots do the fighting.~~ **Read**: a
  building takes 0.85 by its Type, as a machine at 0.5 m/s or more does, while a
  flyer and anything slower, the fixed towers among them, take 0.45. The premise
  was wrong: the bunker's flame scores 1 at any height, its round's bit 8
  returning before the height term (`0x1001ba00`), so with its turret settled it
  passes 0.85 on a flyer 15 m up as on a walker, and the engine's bunker fires 4
  flames in 6 s on a flyer hovering 80 m out
  ([How the AI fires](#how-the-ai-fires--read)).
- The order record whose `+0x30` id lets the go, attack and search tasks fire
  without aim (`0x10014bd0`); whether a self-given attack carries one; and
  the height term in the distance score.
- ~~A seeker's value 2: read by nothing found.~~ Answered: it is the gun's lock
  ([A guided gun waits for a lock](#a-guided-gun-waits-for-a-lock--read-and-measured)).
- ~~What an AI turret's state word is while it fights.~~ `0x400`, set on every
  aiming pass, so its unguided guns keep the target and the range gate
  ([How the AI fires](#how-the-ai-fires--read)). Whether the relink runs
  between that set and the gun's shot was not followed.
- ~~Which matrix `AniMesh` interface `0xb` slot `0x10` hands the effect manager for
  its argument 2, with which the manager keeps a beam's muzzle on the shooter's
  node 0 and carries it back into the world each tick
  ([A beam outlives its round](#a-beam-outlives-its-round--read-and-measured)).
  That it is the node's world matrix is *derived* from the point going both ways
  through it.~~ — **read**: the node record's `+0x20`, the object's world matrix
  times the node's chain of keyed poses as the pose walk builds it
  (`AniMesh.dll:0x10005320`,
  [13-control.md](13-control.md#a-buildings-load-group--read-and-measured)).
- What the host does with the (0, `0xe`) call `World3D.dll` makes when a gun
  is selected (`0x10010805`), and whether an arm's channel meets exactly 1 in
  the arm's own channel update, which the arm sounds depend on (the turret
  update's step, `0x100289f0`, is read; the base component's is not).
