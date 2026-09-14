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

- it has rounds (state 5 otherwise);
- its capacitor holds value 2 (state 6 otherwise);
- its ready byte `+0x118` is set (state 7 otherwise, `0x10029d27`). The
  constructor sets it (`0x100295b3`), and a turret's takt then rewrites it
  once its arm is out
  ([below](#a-gun-is-ready-once-its-arm-is-out--read-and-measured));
- its state word is non-zero.

Values 8–10 add target gates: no shot at a target farther than value 8
(`0x10029e37`) or further off the barrel than the angle value 10
(`0x10029edd`), and a value-9 delay in seconds counted down first
(`0x10029f2c`). They are zero on every
shipped gun, so no gate applies.

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
  gun; `e_gun_bl_03` and `e_gun_tl_02` have a follower and no gun. No channel
  carries the `0x2000` flag the takt treats apart.

**Each takt** (`0x10027ecd`), `0x10028200` moves each mount and returns its
gun's ready byte (`0x10027f51`):

1. **The arm's progress *p*** is read, or 1 where there is no arm. While
   *p* < 1 the mount heads for (1 − *p*) × its initial value + *p* × the pitch
   target, and **the gun is not ready**.
2. **Once the arm is out**, the mount heads for the pitch channel's target.
3. **A falling round adds an elevation.** The gun keeps its round's top speed
   (`+0x94`) and a gravity flag (`+0x98`), which is 1 when the round's mode is
   not 0 (`0x100297ef`). With *g* = the world's 10 × that flag and *d* the target
   from `TurretCenter`, the mount solves for the flight time, *t*² =
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
- **The ground.** The landscape answers interface `0x18` as well
  (`Terrain.dll:0x1001a174`). That it is one of the objects the ray walks was
  not traced.

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

**Only the AI sets it** (*read*):

- **One writer.** The turret's target is written by its set-target method
  (`0x100280d0`), and only the stream restore writes it besides.
- **One way in.** That method is reached only through interface `0x204` slot 16
  (`0x1002cb00`).
- **Every caller was enumerated.** Each call at that slot's offset with a
  target argument was listed in every module. Five are in `Behavior.dll`'s
  fight code:
  - `0x10024b1b` and `0x10024f8f` aim, in the fight module's takt;
  - `0x10025107` clears the target with (0, 0) when the module is reset
    (`0x10025070`, from the mode set once flag `0x20` is on, and from
    `0x10031b30`);
  - `0x100253dc` sits in `0x10025320`, which only the takt's playback branch
    calls (`0x10004f13`), and `0x10025aa5` in `0x10025a00`, which only the
    fight takt calls. Neither was read further.
- **The sixth caller** is the gun handing its target to its round
  (`0x1002a514`).
- **Nothing else.** `iron3d.dll`, `World3D.dll`'s manual controller and
  `Wizard.dll` never call it.
- **The fight module runs only while `MBehaviour` flag `0x40` is set**
  (`Behavior.dll:0x100050c5`). Taking the hero over clears it
  ([above](#who-may-drive-a-units-guns--read)).

So in first-person play the hero's turret keeps whatever target its AI last set,
or none. With no target a seeker gives no heading
([Guided rounds](#guided-rounds-differ-in-how-hard-they-steer--read-and-measured)),
so the plasma bolt and the missile fly straight (*derived*).

**Lead is the AI's too** (*read*). Property `0x54` on a turret is its lead speed
`+0xa8` (`Control.dll:0x1002ea10`). The fight module copies a gun's round speed
into it (`Behavior.dll:0x10024ba2`), and a turret in `CIS_POINTTRACE` aims at
the point where a target moving at its velocity (property `0x27`) meets a round
at that speed (`Control.dll:0x10028640`). Nothing sets it for the player.

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
  audible from 2 to 20 m in the window 0.15 to 1. They are the arms' own sounds
  as they unfold, not the shots'.
- **The missiles** have no barrel effect. Their launch is the round's own
  `hero_gunfire_missile` (mode 1, 5 s, `H_fire_missile.wav` in its first
  tenth).
- **The rounds' own effects** start when the round loads: `hero_cannon_bullet`,
  `hero_prifle_bulletA`/`B`, `hero_laser_bullet`, and the missile's engine,
  smoke and `hero_gunfire_missile`.
- How a sound emitter behaves across its window, once or looping, belongs to
  [11-effects.md](11-effects.md#not-resolved).

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

**Value 2 is read by nothing found.** The seeker's own methods read value 0
(`0x100247a0`) and value 1 (`0x100248d6`). No request for value `0x302` in
the install is aimed at a seeker. The same scan finds the radar's
(`0x10024721`), the detect shield's (`0x1002bf67`) and the deflector's
(`0x1002bfbb`).

A seeker's target is the gun's target, handed to the round as it leaves
([The round's start](#the-rounds-start)). A turret's gun has its turret's
target.

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
- the turret's lead speed is set to one of its guns' round speeds
  (property `0x54`);
- the turret is put in `CIS_POINTTRACE` (`0x400`).

**Each gun is scored.** Its record (`0x1001b4b0`) keeps its round's damage
(property 6), its round's speed *v* (property `0x54`), its magazine (property
`0x800`), and the round frame's flags and range. Then:

- **Heavy rounds are held back.** A gun whose round does 10,000 damage or more
  fires only at a target whose id has 3 in its `0x0f000000` nibble
  (`0x10024d30`). Those are the three winged SSM launchers (*measured*).
- **Distance** (`0x1001b9f0`). The score rises from 0 to 1 over the first 5 m,
  holds 1 out to (*v* + 1) ÷ 2, and falls to 0 at 2 (*v* + 1). It is then
  multiplied by 1 − height ÷ *v*.
- **Aim.** It is multiplied by two more factors from the turret's and the gun's
  aim errors (interface `0x202` slot 10, `0x10024c92` and `0x10024d54`).
- **Threshold.** The gun fires **one shot** (`CIS_SINGLEFIGHT`, `0x200`,
  `0x10024fa2`) when the product clears 0.45, or 0.85 on a building and in
  some other states (`0x10024ebb`).
- **Its own timer.** The shot also waits for the gun's randomised timer: 30 ÷
  magazine s plus up to as much again when the magazine holds more than 2,
  otherwise 0.5 s plus up to 1.5. Both are divided by the difficulty profile's
  value (`0x1001b5ec`, `0x1001b650`).
- **During tasks 2, 3 and 5** every gun fires on its timer whatever its score
  (`0x10024e58`).

So a unit fires every gun whose score clears the bar, each on its own timer.

*Derived*, from the distance score alone: a laser or taser (10,000 m/s)
scores fully out to 5,000 m, a 350 m/s cannon to 175 m, and a 70 m/s missile
only to 35 m and not at all beyond 142 m. `weapons.ai_distance_score` and
`weapons.ai_fire_wait` compute them.

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

## Not established

- Whether a target the hero's AI set before the player took over survives into
  first-person play. Nothing clears it on takeover. Whether the fight module
  ever runs for the hero before then was not followed: `iron3d.dll:0x100a3cd2`
  sends all of a player's objects message 7 with 0, but when it runs is not
  read.
- Whether the landscape is one of the objects IWorld slot 7 walks, so that the
  sight ray converges on the ground and not only on objects.
- The vector a falling round's mount solves for when its turret has no target
  (it is then the turret's aim triple, `Control.dll:0x10027e07`), and which way
  the `0.83 × angle` elevation turns on a hanging turret.
- `e_gun_bl_03` and `e_gun_tl_02` carry a follower and no gun. The turret's takt
  reads the paired gun without a check, so either they are never fitted or the
  follower pairs with a later part's gun; not traced.
- The fight module's two aim factors (interface `0x202` slot 10), which of a
  turret's guns lends its round speed as the lead (the turret record's `+0x1c`),
  what tasks 2, 3 and 5 and `MBehaviour+0x614` are, and the target id's nibble
  3 that frees the winged SSMs.
- A seeker's value 2: read by nothing found.
