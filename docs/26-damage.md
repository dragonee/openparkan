# Damage, shields, armour and repair

Everything a hit does happens in `Control.dll`, in the same control system
that runs a building's power ([23-economy.md](23-economy.md)). The object hands
it out as **`ILifeSystem`** — interface id `0x16` in its `QueryInterface`
(`0x100076f0`), the sub-object at `+4`, vtable `0x1003b59c` — and
`Behavior.dll` refuses to run a unit without it ("Behaviour panic: Cannot
receive ILifeSystem"). Bots and buildings go through the same code.

**Every claim is tagged**, as in [23-economy.md](23-economy.md): *measured*
is re-derived by `openparkan verify`, *read* comes from the disassembly at the
address given, *guess* fits the evidence and is not established.

## Hit points — *read*, and *measured*

Each node of a model has a life, built from its `.ndp` record
([07-objects.md](07-objects.md#ndp-is-a-damage-table-one-record-per-node)):

- **Maximum** = the `.ndp` hit points × the object's **volume scale** (its three
  scales multiplied, `+0x548`, `0x10009ee0`) × the **level ratio** (`+0x660`,
  below). Changing either scale rescales every node's life and maximum in
  proportion (`0x10009f90`). The volume scale is 1 on every unit and building,
  which are built from their `.dat` with the placement's matrix alone, and the
  placement's scale cubed on **vegetation and rock**, which are not
  ([below](#vegetation-and-rock-carry-node-life--read-and-measured)).
- **Damage** lowers a node's life, clamped at 0 (`0x10010f30`). A part's
  *condition* — the `0x100` bit on component values — is `life / max`.
- **A node at 0 is destroyed.** A node that is destroyed, or steps up a damage
  stage, destroys its child nodes (`0x10011130`). If it is node 0 or carries
  node flag bit 1, the object's owner word (`+0x550`) becomes `0xfffe`
  (`0x10011098`) and **the object dies** — unless its agent kind (`+0x50`, the
  kind `LoadControlSystem` is given) is 3, which is only marked (`0x100110ab`).
  A radar never detects an object whose word reads `0xfffe`
  ([25-sensors.md](25-sensors.md#a-scan-is-a-sphere-a-falloff-and-three-tests--read)),
  and **so does everything else that walks the world**: the word is the one
  thing the whole interface asks before it uses an object, in 37 places in
  `iron3d.dll` alone
  ([27-ownership.md](27-ownership.md#the-37-compares-in-iron3ddll--read)).
  The pass-over-wrecks reading is no longer a guess joining two reads.
  The word is also **one-way**: the interface's own setter refuses to write it
  and refuses to write anything over it (`Control.dll:0x1000f1f0`), so a wreck
  cannot be given back to a clan.
- **Agent kind 3 is a building** — *read*. An agent with a parent takes the
  parent's kind (`AniMesh.dll:0x10003174`, `IGameObject` slot 11) instead of
  one from its `objects.rlb` tag; a building's agent is loaded by
  `Terrain.dll`'s `CBuilding` — the object `LoadBuilding` makes for a `FORT`
  record — with the building as its parent (`Terrain.dll:0x10055e95`), and
  the building's slot 11 returns 3 (`0x10057da0`). The same agent build gives a
  kind-3 agent a hall-way graph and a collision manager of its own
  (`AniMesh.dll:0x10003488`).
- **A building whose node 0 dies becomes a shell** — *read*. Its owner word
  reads `0xfffe`, the id the behaviour code calls `DETID_KILLED`; on its next
  takt the behaviour sees it and sets its killed flag `0x1000`
  (`Behavior.dll:0x10004d65`), and from then on the takt returns at once — no
  radar module, no docks or places, no building takt (so no repair decision
  and no production), no fire control (`0x10004d54`). Only the behaviour's
  constructor clears that flag (`0x10003c85`). The object is not removed: its
  model and its other nodes stay, and they can still be shot apart.
  `iron3d.dll` compares with `0xfffe` in 37 places, and **all 37 are now
  enumerated and grouped**
  ([27-ownership.md](27-ownership.md#the-37-compares-in-iron3ddll--read)): the
  lists and pickers, the HUD, the target list, the mode stack and the two
  component tests all pass the object over, so to the interface the building is
  gone while its model stands. The walker over registered objects by id and
  mask (`0x1007dee0`) is one of the 13 in the first group.
- **Which buildings that can happen to** — *measured*. 12 of the 30 building
  tables in `fortif.rlb` give node 0 one hit point beside parts of
  40,000–500,000 — mines, plants, stores, the generator, the hangar and the medium
  main teleport — and on **all 12 node 0 has no level-0 geometry**, where it has
  on all 18 others. A round, which strikes level 0 only
  ([the hit test](#the-hit-test--read-and-measured)), can never hit that node;
  that a blast cannot either, having no sphere to overlap, is a *guess*.
- **Damage stages.** A node with *N* stages — its mesh blocks in a row that
  have a level-0 slot, block 0 included — is in stage
  `N − ceil(N × life / max)`; each step up plays the node's `.exp` explosion
  (`0x10011220`). Which block that draws, and what happens at the last stage,
  is [below](#what-a-damaged-node-a-destroyed-part-and-a-dead-unit-draw--read-and-measured).

What the shipped tables give node 0 (*measured*):

| | node 0 hit points |
|---|---|
| chassis `r_t_*` | 90–120 |
| chassis `r_l_*` | 190–450, and one of 83,000 |
| chassis `r_h_*` | 380–1,400 |
| chassis `r_m_*` | 480–1,100 |
| chassis `r_b_*` | 1,500–4,500, and two of 10,000 and 80,000 |
| guns (`guns.rlb`, one node) | 120 small and medium, 500 big, up to 3,000 |
| internal parts (`intsys.rlb`, `o_*`) | 1 |
| bunkers, research centres, towers | 40,000–100,000 |
| bridges | 120,000–200,000 |
| mines, plants, stores, Outposts, generators, the medium `mtp` | 1 on node 0; parts 40,000–500,000 |

## What a damaged node, a destroyed part and a dead unit draw — *read*, and *measured*

`Control.dll` keeps a node's life in a 44-byte record (`+0x55c`): `+4` its
parent, `+8` life, `+0x14` life ÷ maximum, `+0x18` its stage, `+0x1c` its
stage count, `+0x24` its explosion, `+0x28` its status. It tells the unit's
mesh what to draw through `IAnimation` (`+0x20`), whose `AniMesh.dll` node
records are 0x130 bytes (`+0x1a8`): `+8` the mesh node, `+0x14` a flags word,
`+0x18` the parent, `+0x1c` the stage value, `+0x124` life ÷ maximum.

**What the loader takes from the mesh** (`0x1000f940`).

- The parent: query `0xd`, the mesh record's `+0x18` (`AniMesh.dll:0x1000523d`).
- Status bits from query `0xe`, **the mesh node's own flags word**:
  `0x200` → status 1, a vital node (`0x1000f9aa`, `test ah, 2`); **`0x100` →
  status 2, a node that is never hidden** (`0x1000f9bd`); `0x400` → 4 and `0x1`
  without `0x80` → 8, a node that copies its parent's life fraction or its stage
  (`0x1000f9d0`, `0x1000f9e3`; `0x1001130e`–`0x10011344`).

  **That query `0xe` is the mesh word, and not the runtime record's own, is
  read.** `AniMesh.dll:0x100051f0` is the node query: it strides the 0x130-byte
  records from `+0x1a8` and dispatches ids `0xa`–`0x10` through the table at
  `0x100052fc`, anything else answering nothing (`0x100052f5`). Id `0xa` answers
  the **runtime** record's flags word, its `+0x14` (`0x10005221`) — the word
  `IAnimation` slot 8 writes, whose bit 1 is *hidden* and bit 4 *flying*
  ([below](#what-a-damaged-node-a-destroyed-part-and-a-dead-unit-draw--read-and-measured)).
  Id `0xe` (`0x10005242`) is a different word: it takes the pointer at the
  record's `+0x12c`, dereferences it, indexes it by the record's `+8` at a
  **stride of 38 bytes** and reads the `uint16` at offset 0, zero-extended —
  the mesh's stream-1 node record and its first field
  ([07-objects.md](07-objects.md#nodes-slots-and-levels-of-detail)). Ids `0xd`,
  `0xf` and `0x10` are the parent, the area and the volume, as above. The four
  bits the loader tests settle it from the other side: bit 0 of the mesh word is
  the documented *interior* bit, measured on 1564 sub-objects, while bit 0 of
  the runtime word is *hidden*, which nothing is at load — read that way status
  8 could never be set on anything.
- **The stage count**: `IAnimation` slot 18 (`AniMesh.dll:0x10005840`) counts
  variants 0, 1 and 2 in a row whose level-0 slot exists; none gives 1
  (`0x1000fa0b`).

**The stage** (`0x10011220`) is `N − ceil(N × life / max)`, held to *N*
(`0x10011346`–`0x10011389`). When it changes, a rise plays the node's `.exp`
when it names one (`0x100113b2`–`0x100113be`). Then:

- **The block drawn.** `IAnimation` slot 17 is given the stage held below *N*,
  `min(stage, N − 1)` (`0x100118b1`–`0x100118c4`), into the node's `+0x1c`
  (`AniMesh.dll:0x10005810`). The mesh draw asks every node for
  `slot_index[stage × 5 + level]` (`0x100124d0` with variant −1, which reads
  `+0x1c`; `0x10014e65`), and a node with no slot there draws nothing.
- **The last stage hides the node.** At stage *N* — which only a life of 0
  gives — a node without status 2 goes through `0x10011920` with 0
  (`0x100118cd`–`0x100118e2`). That sets status `0x20`, and sets **flag 1 in
  the mesh record's flags word** through `IAnimation` slot 8 (`0x10011a52`;
  slot 8 is `AniMesh.dll:0x10005500`, which clears the flags it is given when
  its operation word is odd and sets them otherwise). It also takes the node's area
  and volume (queries `0xf`, `0x10`) and its mass out of the totals, and
  recomputes the live limits
  ([24-motion.md](24-motion.md#what-sets-the-live-limits--read)).
  Leaving stage *N* again, under repair, clears flags 1 and 4
  (`0x1001199e`).
- **A node with flag 1 is gone from the world.** The mesh draw skips it
  (`AniMesh.dll:0x10014e57`, and the second draw loop at `0x100150a2`). So do
  the node visitors of the two walk-face queries (`0x1000ce90`, `0x10015b60`),
  the collision push (`0x1000dfe0`), the segment query (`0x10010dc0`) and the
  point-in-part test (`0x100106d0`), each answering nothing for it, and the
  subtree draw a building's parts go through (`0x100101d0`). A hidden part is
  not drawn, not stood on, not collided with and not struck. Its object's
  sphere is not changed
  ([What a hidden node is left out of](#what-a-hidden-node-is-left-out-of--read)).

**Children go with their parent** (`0x10011130`). After a tick's hits the
walk runs from node 0 down (`0x10013127`, `0x10013136`). A node whose stage
rose, or which is destroyed, hands each child a hit of minus its life
(`0x100111df`–`0x100111ed`). The child is marked `0x80` when the parent is
flying off (status `0x40` or `0x80`) and `0x100` otherwise
(`0x100111ab`–`0x100111d4`).

**A destroyed part is first knocked off** (*read*). Before that walk, every
node with status `0x10` is offered to the control system's main-interface
slot 25 (`0x100102a0`, from `0x10012fce`–`0x10012fe2`). It takes a node:

- with a parent (`+4` not −1), on an agent that is not a building (kind 3);
- whose status has none of `0x1ce` (`0x100102dc`) — not never-hidden, not
  copying, not already flying or marked;
- while the byte at `0x100424a0` is set. The static initialiser
  `0x10006350` (listed at `0x1003e1bc`) sets it to 1 and `0x100424a4` to
  3000.0, and nothing else writes either.

The part gets status `0x40`, flag 4 in its mesh flags word (`0x10010741`),
and a 0x88-byte flight record in the list at `+0x568` (`0x10010720`) that
ends 3000 ms on (`0x10010613`). **While it flies its stage is skipped**
(`0x10011306`), and so is that of each child it takes down (`0x80`). They
stay drawn, and they hang off the flying part (*derived*: a child's model
matrix is its parent's times its own,
[07-objects.md](07-objects.md#node-poses)). The flight update
(`0x100131da`–`0x100135b6`) moves the part, lowers its velocity's z each tick
(`0x100133a2`) and hands its matrix to the mesh (`IAnimation` slot 5,
`0x100133fe`). At its end time
(`0x10013494`), or earlier when a world query ends it (`0x100134c1`), the
record goes and the part's status turns `0x40` into `0x100`
(`0x10013568`–`0x1001357c`). The walk from node 0 then runs (`0x100135b6`): the
part reaches stage *N*, **its `.exp` plays and it is hidden**, and its
children with it.

**A dead unit is deleted** (*read*). When node 0 or a vital node is destroyed
and the agent is not a building (`0x100110ab`) nor carries `+0x104` bit
`0x10000000`:

- the owner word becomes `0xfffe`;
- the death time `+0x59c` becomes now + **the controller's `+92`**, in ms
  (`+0x4b8`, [13-control.md](13-control.md#the-frame-is-the-live-objects-parameter-block);
  `0x100110b1`–`0x100110ce`; the clock `+0xe4` counts milliseconds);
- message `0x15` with 7 goes to the object at `+0x34` (`0x100110ee`);
- node 0 takes a hit of minus the unit's total life, and its stage runs
  (`0x10011105`–`0x1001110f`).

That last step is what a **vital** node's death buys: node 0 is taken with it,
and the walk from node 0 then takes everything else. The vital bit is not one of
the `0x1ce` that bar a knock-off, so the vital node itself is offered to the
knock-off like any other destroyed part.

The control tick compares the death time with the clock and, once it has
passed, calls `World3D.dll!KillGameObject` with the object's id
(`0x1000c93f`–`0x1000c977`, `0x1000d080`–`0x1000d0af`). That looks the object
up and deletes it (`World3D.dll:0x100088a0` → `0x100087e0`). Node 0 dying has
already destroyed and hidden every node below it that is not never-hidden, so
**a unit vanishes with its explosions and is removed a moment later**. A
building is only marked, and its never-hidden nodes stay as the shell
(above).

**What the shipped files give** (*measured*):

- **Stage counts** across the 2340 nodes of the meshes the library's records
  name: 1 on 1598, 2 on 130, 3 on 15, and no level-0 slot on 597 (counted as
  1).
- **Never hidden** (mesh node flag `0x100`) on 60 nodes, all on the 33
  `bu_*` building records: the shells.
- **Vital** (mesh node flag `0x200`) on **90 nodes of 1845**, on **9 of the 435
  meshes**, in three archives, and it is a coherent 90: every one of them is a
  geometry-bearing segment of an **articulated limb or body chain**.

  | mesh | vital / nodes | which |
  |---|---:|---|
  | `R_H_02` (the hero chassis) | 9 / 10 | the body `B_Dn` and both legs, `LL_Up`…`FL_Dn`; only the empty socket `Base_TL` is not |
  | `R_B_05` (the Transformer) | 12 / 32 | the two leg spines `LUU`…`LDD`, `RUU`…`RDD`; not node 0, the body panels or the foot pads |
  | `R_B_07` (the L-7f monster) | 4 / 11 | the body chain `Mnst1`…`Mnst4`; not the shells or the turbines |
  | `A_L_01`, `A_L_02`, `A_L_04`, `A_L_05` | 13/16, 7/19, 17/27, 17/27 | body, neck, head, tail, legs and wings; not the last segment of a tail or wing |
  | `o_tur_ha_02` (the hero turret) | 8 / 36 | its two arm segments each side and the body `B_Md`, `B_Up`; not a gun pod, the radar, the deflector or an empty pivot |
  | `o_tur_la_06` | 3 / 10 | `LTdu`, `LTdl`, `LTdr` |

  No building carries the bit — 0 of the 273 `fortif.rlb` nodes — and neither
  does any wheeled or tracked chassis. The bit is disjoint from every other one
  the loader reads: 0 of the 90 also carry `0x100`, `0x400` or `0x1`. It is not
  a variant of the unexplained `0x10`, which is broad where this is narrow —
  `0x10` is on **861 of the 1845** nodes and in all nine archives, `0x200` on
  90 in three, and they coincide on 8, each of them the leaf of a flagged chain
  (`0x210`). **What the flag buys**: the hero's 2,881 hit points over ten nodes,
  `a_l_05`'s 130,010 over 27 or the L-7f's 1,050,001 over eleven need not be
  chewed through node by node — shoot any one limb off and the thing is
  finished. *Not established*: why the
  bit falls on these nine models and not on the other walkers, whose legs
  (`R_B_01`, `R_M_01`, `R_L_01`) carry it nowhere.
- **The controller's `+92`**: 0 on 333 records, then 5000 on 118, 1000 on 42,
  2000 on 22, 3000 on 21, 11000 on 3. On the chassis: 2000 on 20, 5000 on 2,
  3000 and 1000 on one each.
- **Mission 01's dummies.** Neither has a vital node or a never-hidden node,
  and every node names an explosion.

  | chassis | node | parent | stages | hit points | explosion | controller `+92` |
  |---|---|---|---:|---:|---|---:|
  | `r_h_01` (`l_targ.dat`) | 0 `ASbs` | — | 1 | 500 | `explode_aim_S` | 3000 |
  | | 1 `ASd1` | 0 | 2 | 800 | `explode_aim_S` | |
  | | 2 `ASd2` | 0 | 2 | 800 | `explode_aim_S` | |
  | | 3 `ASd3` | 2 | 2 | 600 | `explode_aim_S` | |
  | `r_h_03` (`M_targ.dat`) | 0 `ALbs` | — | 1 | 1400 | `explode_aim_L` | 5000 |
  | | 1–5 `ALd1`…`ALd5` | 0 | 2 | 1000 each | `explode_aim_L` | |

**So, shooting a dummy** (*derived* from the reads above):

- A part at or below half its life draws its damaged block, with an
  explosion.
- At 0 it is knocked off. It flies, drawn, for three seconds, then explodes
  and is gone.
- `ASd2` reaching its damaged block kills its child `ASd3`: `ASd3` explodes and
  is hidden where it stands, since its parent is not flying.
- `ASd2` destroyed takes `ASd3` along on its flight.
- The base, node 0, reaching 0 hides it and every part still standing, each
  with its explosion. The unit is deleted three seconds later on `l_targ`,
  five on `M_targ`.

## Vegetation and rock carry node life — *read*, and *measured*

A tree and a stone are agents like any other, and they take damage.

**Read.** `iron3d.dll` builds a placed tree or stone by handing
`World3D`'s `AddNewObjectToGame` the `objects.rlb` library, the record name and
the **type 10** (`0x100a4331`–`0x100a4334`,
[04-missions.md](04-missions.md#the-scale)). The agent that comes out is built
by the same code as a robot's (`AniMesh.dll:0x10003100`–`0x100033d1`): the
`STAT` tag makes its collision kind 10 (`0x100031be`), it is given the
sub-object at `+0x6f4` that only kinds 3, 4 and 10 get
(`0x100031cf`–`0x100031e4`), and then — before any
branch on the kind — its **control system is loaded**
(`0x100032e7`, `Control.dll!LoadControlSystem`) and its **`ILifeSystem`,
interface `0x16`, is queried out of it and kept at `+0x160` (`0x1000330d`).
Only the later branch, `kind − 3` then `− 1` (`0x100033d1`), is a building's or
a unit's alone. So a tree gets the whole of [Hit points](#hit-points--read-and-measured)
above.

The life loader says so itself. Its last act is to write the object's total
maximum to `+0x58c` and `+0x590` and a **threshold** to `+0x594`: the total
times **0.3 for an agent of kind 10**, times **0.2 for anything else**
(`0x1000fa82`–`0x1000faaf`, the two constants written once by
`0x100063c5`/`0x100063cf`). A branch on kind 10 inside the life loader is
reachable only if scenery has a life system. What the threshold does is the
wreck's burn: on the control tick, once the total falls below it, an effect
`+0x4f4` starts, the mark moves to the current total, and the object takes
0.1 of that mark spread over its nodes through the repair's own update
(`0x10012bdc`–`0x10012c37`); the effect `+0x4f8` stops again if it climbs back
(`0x10012b0a`–`0x10012b20`). A tree is given the longer fuse.

Nothing else treats scenery apart: its agent kind is 10, not 3, so node 0's
death **kills and deletes** it rather than leaving a shell.

**Measured.** All **81** `STAT` records in `objects.rlb` name a `.ndp` and a
`.ctl`; the tables hold **123 node rows**, none of them 0.

| | records | nodes | hit points | explosion |
|---|---:|---:|---|---|
| stones `s_stone_*` | 14 | 1 each | 500,000 or 1,000,000 | `explode_stone` |
| trees `s_tree_*` | 66 | 1 to 11 | 1 (a leaf) to 1,500,000 | `explode_tree`, `explode_tree_30`, `explode_leaf` |
| `mtcheck` | 1 | 1 | 1,000,000 | `explode_rbr_l` |

`s_tree_04` is the clearest: a trunk of 3,000 with ten leaves of 1 apiece, each
leaf playing `explode_leaf`. The three `s_tree_30/31/32` are 100 hit points, so at
scale 1 a single light bullet's 150 fells one. **The control**, the same query by tag: 63 of 63
`BTLU` units and 146 of 146 `EXTO` parts name a table that reads, and **0 of 34**
`FORT` records do — so the query discriminates and the scenery's 81 are not an
artefact of it. Every scenery `.ctl` has **no components at all**: no armour,
no shield, nothing to draw power. Their `+92` is 5,000 ms on 79 of the 81, so a
felled tree lies for five seconds before it is removed.

**A round can strike it.** 74 of the 81 scenery meshes carry a level-0 triangle
a round does not pass through; on the other 7 — `s_tree_33`, `34`, `45`, `46`,
`47`, `48` and `mtcheck` — every level-0 triangle is flagged 4 or 32 and a round
flies through the whole model ([the hit test](#the-hit-test--read-and-measured)).
Of the 71 records the shipped missions place, 66 can be struck, covering 372 of
the 401 placements.

**And a scaled tree really is tougher.** Only vegetation and rock are built at
their placement's scale ([04-missions.md](04-missions.md#the-scale)), and the
control system re-reads its mesh's scale on every tick and rescales every node's
life and maximum by the three factors multiplied (`0x10007ac6` → `0x10009ee0`).
216 of the 401 placed trees and stones stand at a scale other than 1, from 0.2
to 21, so the cube runs from 0.008 to 9,261: Mission 01's `s_tree_04` at 3 has a
trunk of 81,000 rather than 3,000, its `s_tree_41` at 0.4 is 64 hit points
rather than 1,000, and Mission 02's `s_stone_10` at 21 holds
**4,630,500,000**. A big rock is scenery in the arithmetic as well as the
fiction.

## The difficulty ratio — *read*, and *measured*

`Iron_3D.ini` carries `[LEVEL_RATIO] EASY=0.5 MEDIUM=0.7 HARD=1.0`, and
`GAME_LEVEL` picks one: `iron3d.dll` stores the setting at `+0x150`
(`0x10028e93`) and `0x10076010` reads it, 0 `EASY`, 1 `MEDIUM`, anything else
`HARD`. The shipped file says `GAME_LEVEL=1`.

That code gives the ratio, as object property 180, to **every warrior, HQ and
hero** (`Type` `0x1008000`, `0x1010000`, `0x1020000` — the `prof_war`,
`prof_hq` and `prof_hero` profiles) **of a clan not allied with the player's**:
when the mission has loaded (`0x1007d4b0`) and when one is created after. Not
builders, transports or explorers, and never the player's own. Property 180
does three things (`0x1000e980`, `0x1002ba30`):

- scales every node's life and maximum — **enemy combat units have half the
  hit points on easy**;
- scales the fight shield's sector maximum (`+0x11c`, `0x100255c0`);
- is copied to every class-2 gun (`+0x180`), and a gun hands it to each round
  it fires (`0x1002a3fb`) — which scales **the round's damage** (below).

### The difficulty block every behaviour holds — *read*, and *measured*

`behpsp.res` carries five difficulty profiles, `diff_strong`, `diff_normal`,
`diff_weak`, `diff_slow` and `diff_stupid`, of seven variables each. A
behaviour keeps the block they would fill at `+0x8d4`, and **no unit is ever
given one: every behaviour holds the block's compiled defaults.**

- **The defaults** (`Behavior.dll:0x10019b90`, the block's constructor, run
  from `MBehaviour`'s at `0x10003940` on `+0x694` + `0x240`):

  | `+` | variable | default | strong | normal | weak | slow | stupid |
  |---:|---|---:|---:|---:|---:|---:|---:|
  | 0 | `Speed_MaximumFactor` | **1** | 1 | 1 | 1 | 0.7 | 1 |
  | 4 | `Router_RandomError` | 0 | 0 | 0 | 0 | 0.5 | 1 |
  | 8 | `Fire_MissAngle` | 0 | 0 | 0 | 0 | 0 | 0 |
  | `0xc` | `Fire_FreqFactor` | 1 | 2 | 1 | 0.5 | 1 | 1 |
  | `0x10` | `Decision_RepairOn` | **0.5** | 0.8 | 0.5 | 0.1 | 0.5 | 0.5 |
  | `0x14` | `Decision_RepairOff` | **1** | 0.9 | 0.8 | 0.3 | 0.8 | 0.8 |
  | `0x18` | `Decision_Dormancy` | 0 | 0 | 0 | 0 | 0 | 5 |

  The profiles' columns are *measured* (`openparkan.profiles`). The defaults
  match `diff_normal` on six of the seven, and not on `Decision_RepairOff`.
- **The one loader** is `MBehaviour` slot 25 (`0x1000a1a0`, in the vtable at
  `0x100592b4`), which opens a resource file, finds a member by name and binds it
  into one of five blocks by its kind: 1 `+0x7c0`, 2 `+0x820` (the behaviour
  profile), 3 `+0x8a0`, 4 `+0x694`, **5 `+0x8d4`** (`0x1000a2ed`–`0x1000a2fa`,
  binder `0x10019bd0`). The block is read through `0x100146a0` alone — five
  calls, the repair decision's two, the fire control's (`0x10023f9f`) and
  `SetTarget`'s two — and a raw scan of `Behavior.dll` for the displacements
  `+0x8d4` to `+0x8f0` finds only the binder's `lea` and that getter.
- **Nothing names a `diff_*` member** (*measured*, as a search with a control).
  A case-blind search of every string in the sixteen modules for `diff` finds
  seven — `fDifficulty`, `Movement_StopDifference`, `TargetDifference` and four
  log texts — and no profile's name; a byte search of every file in the install for
  `diff_` finds `behpsp.res` alone, the archive that holds them. The control is
  the same search for names that are loaded: `prof_war` is found in
  `Behavior.dll`, which loads it by name through slot 25 with kind 2
  (`0x10008ae5`), and `chas_fly` in `objects.rlb`, whose chassis records name it.
  Only `diff_*.var` carries the seven variables (*measured*, all 32 members).

So `diff_slow`'s 0.7 speed cap never applies
([24-motion.md](24-motion.md#how-the-ai-asks-for-speed--read)), and the repair
decision reads 0.5 and 1 ([below](#repair-a-units-own-repair-unit-switched-on-and-off--read-and-measured)).

## A hit, from the round to the node — *read*

**The `.exp` is the damage.** Its first word, which
[11-effects.md](11-effects.md) took for a count, is the **hit kind**
(`0x1000ebc0`):

| kind | shipped | what it does |
|---|---|---|
| 1 | 26 — animals, scenery, building and robot debris, `aim` | nothing but the effect |
| 2 | 54 — `bb_*_01` bullets, `bl_*` lasers, `bt_*`, `bp_h`, `bp_l`, `rg_b`; damage 1 on all | the whole damage to the one node struck |
| 3 | 64 — `ba`, `bf`, `bm`, `br`, the big `bb` and `bp`, `fm`, `fr`, `explode_rbr_bomb`; damage 1–800,000 | a blast: shields, then every node in reach |
| 4 | none | handled: shields only, never a node |

and the first two floats are the **damage** and the **radius** — absolute on a
round (agent kind 9), times the node's bounding radius otherwise. When a node
reaches a new damage stage it sends a hit whose damage is

    level ratio × .exp damage + the life the node lost since its last stage

(`0x10011794`). A round dies from full health, so **a round hits for
`ratio × (its .ndp hit points + the .exp damage)`**. The data bears that out
(*measured*): all 29 direct-hit rounds have an explosion damage of 1 and hit
points ending in 9 — 149, 899, 1,699 — so that the sum is round on 28 of them,
where the hit points alone are round on none. A light bullet `bb_l_01` hits
for 150, a big one `bb_b_01` for 900, lasers for 200–1,700.

The hit is queued on the object that exploded and applied on its tick
(`0x10012ce0`):

- **Kind 2** goes to the object and node the round struck. The hit learns
  them from the round's own collision object: when node 0's stage explodes, it
  takes the contact record's face reference — object, node, batch, triangle in
  batch, triangle, as the mesh test wrote it (`Control.dll:0x1001d9d0`) — and
  copies its **first three** into the hit's `+0xc`, `+0x10` and `+0x14`
  (`0x10011479`). A round stopped by a shield bubble wrote that reference
  itself (`0x1000d23f`): the object, node **−2** and the **sector** in the
  third place — which is how a hit "names no node of its own and carries its
  sector" ([below](#shields-a-generator-a-deflector-six-sectors--read-and-measured)).
- **Kind 3** hits the exploding object's own nodes, then **every object whose
  sphere meets the blast's**, through `ILifeSystem` slot 8
  ([What a blast reaches](#what-a-blast-reaches--read)).
- **Before anything, the hit tells its object who fired** (message `0x19`,
  `0x1000ebdf`), whatever it goes on to do
  ([31-packages.md](31-packages.md#a-hit-pulls-a-unit-in--read)).
- A hit does nothing if the object that fired it no longer exists, nothing to
  the nodes of **the object that fired it**
  ([Whose hit it is, and whom it spares](#whose-hit-it-is-and-whom-it-spares--read-and-measured)),
  and nothing to the nodes of a target that is **invulnerable** (property 162, `+0x5b0`
  — the `[CS] INVULNERABILITY` debug key sets it on the hero, `0x1005e487`,
  and a unit is invulnerable from its arrival at a building it upgrades until
  the new level stands, `0x1003356f`, [32-builder.md](32-builder.md#upgrading-a-building--read)).

**A blast falls off with overlap** (`0x10010030`). For a node whose bounding
sphere has radius *r* and centre at distance *d* from a blast of radius *R*:

| | damage |
|---|---|
| `d ≥ R + r`, or a node sphere of radius 0 | 0 |
| one sphere strictly inside the other, `d < R − r` or `d < r − R` | the whole blast |
| otherwise | `damage × ((R + r − d) / 2R)³` |

### Whose hit it is, and whom it spares — *read*, and *measured*

**A hit names one object as its firer**, an id at the hit's `+0x18`. The damage
stage fills it (`0x10011766`–`0x1001177b`) with the exploding object's `+0x54c`
when that is not 0, and with the exploding object's own id otherwise.

`+0x54c` is **property `0x7f`**. The constructor leaves it 0 (`0x100070c1`); the
property's setter writes it (`0x1000e9d2`, `ILifeSystem` slot 6 through the
thunk at `0x10008000`) and then calls the collision object's slot 6 (`+0x34`);
and one place sets the property — the gun, on the round it has just created,
with the id of the object its own control system belongs to
(`0x1002a398`–`0x1002a3ed`). *Measured*, as a search: `push 0x7f` stands at 12
sites over the 21 modules of the install, and `0x1002a3e8` is the only one in
front of a property call; the rest are three plain arguments of direct calls
in `iron3d.dll` and eight `push`/`pop` pairs of the C runtime, four in
`iron3d.dll` and four in `services.dll`. The scan is by the immediate, so a set
whose id sat in a register would not show; the one it does find is the one the
gun was already known to make.

| what explodes | the firer its hit names |
|---|---|
| a round | the object whose gun fired it: the whole robot, whose parts share one control system ([29-weapons.md](29-weapons.md#the-rounds-start)), or the building |
| anything else — a node of a unit, of a building, of a tree | the object itself |

**A hit whose firer is gone is worth nothing.** The stage looks the id up — the
queue's slot 11, `GetIGObject` by its own panic string
(`World3D.dll:0x10007860`, called at `Control.dll:0x10011783`) — and asks the
object slot 21 (`0x1001178d`), which answers the agent's `+0x6f0`, the byte
control message 7 sets for an object that is not simulated here
(`AniMesh.dll:0x10002fe0`, `0x10001492`,
[13-control.md](13-control.md#control-message-7-says-who-simulates-the-object--read)).
With no object, or a mirror, the damage is 0 (`0x100117b8`) in place of
`ratio × .exp damage + lost life`. The queue walk passes over a hit that is not
above 0 before it looks at its kind (`0x10012f0b`–`0x10012f19`), so such a hit
goes through neither the falloff nor slot 8: no node takes it, on this object or
on another, and nobody is told of it. Its effect is started all the same
(`0x100117d0`–`0x10011897`). A dead unit is deleted the
controller's `+92` ms after it dies
([above](#what-a-damaged-node-a-destroyed-part-and-a-dead-unit-draw--read-and-measured)),
through `KillGameObject` (`World3D.dll:0x100088a0`), so **a round still in the
air when the unit that fired it is deleted goes off for nothing**. A building
is not deleted, and its rounds keep their worth after it is a shell.

**The target's side, in order** (`ILifeSystem` slot 8, `0x1000ebc0`). A kind-3
or kind-4 hit runs:

1. message `0x19` to the target's object, with the firer's id (`0x1000ebdf`);
2. on a building, the door a struck node opens (`0x1000ebf7`–`0x1000ec7e`,
   [24-motion.md](24-motion.md#a-shot-opens-a-door--read-and-seen));
3. **the shield step**, on a copy of the hit (`0x1000ed26` → `0x1000ff00`),
   which takes what the sector stops off the copy's damage;
4. nothing left of it, and the hit ends (`0x1000ed2b`–`0x1000ed3a`);
5. the target invulnerable, and it ends (`0x1000ed40`–`0x1000ed48`);
6. **the target's id equal to the hit's firer, and it ends**
   (`0x1000ed4e`–`0x1000ed59`). The id is the one the world keeps the object
   under: the system's `IGameObject` at `+0x1c`, slot 9, the agent's `+0x120`
   (`AniMesh.dll:0x10017d60`), which a building passes on to its agent
   (`Terrain.dll:0x100573f0`);
7. every node through the falloff (`0x1000ed5f`–`0x1000ed81`), and then the
   push each destroyed node is given (`0x1000ed83`–`0x1000edb8`, slot 25,
   `0x100102a0`).

A kind-2 hit that names a node runs 1, 2, 5 and 6 and then that one node
(`0x1000edfe`, `0x1000ee0c`–`0x1000ee17`, `0x1000ee29`); one that carries a
bubble's sector runs 1 and the shield step alone (`0x1000ee97`–`0x1000ef3c`).

What follows from the order:

- **It is one id against another, and no clan is asked.** Neither slot 8, the
  queue walk, the shield step nor the falloff reads the owner word (`+0x550`). A
  round is given its gun's at its start (`0x1002a3cb`–`0x1002a3dc`, slot 10),
  and nothing on a hit's way looks at it. A blast hurts the firer's own side as
  it does any other, and so does a direct hit.
- **It spares every node of the firer.** The test stands before the first node,
  on the object, and the object is the whole unit.
- **The firer's shield still pays.** The shield step comes first, so a blast
  that crosses the firer's own bubble from outside is stopped by its sector for
  what the sector holds, and the firer is told of the hit, naming itself.
- **It applies to a direct hit as well**, where it has nothing left to do: the
  collision pass has already given the round no contact with its shooter
  ([The hit test](#the-hit-test--read-and-measured)).
- **The hero is an object like the rest.** Its guns are on its own control
  system, so a rocket at its feet costs it a shield sector and no node. At a
  tower's guns it is not the tower: the tower's rounds carry the tower's id,
  and the hero standing in the pod room is struck by them as anyone is.
- **The exploding object's own nodes are not on this path at all.** The queue
  walk hits them itself, before it turns to the others, through the falloff
  alone: no shield step, no invulnerability test, no firer test
  (`0x10012f3b`–`0x10012f59`). The falloff passes over the node that exploded
  (`0x10010066`–`0x1001006e`), a destroyed node and a node whose parent is
  destroyed (`0x10010052`, `0x10010074`–`0x1001008d`). The object is then
  passed over in the list of the others by its id (`0x10012f63`–`0x10012f85`).
  So a unit whose node explodes as a blast takes it on its other nodes, and is
  its own firer to everything else in reach: the firer test never matches there.

**What the data holds** (*measured*):

- **Rounds.** 35 of the 66 readable `BULL` rounds explode as a blast at node 0
  (the 67th, `bm_b_02`, names a `.ndp` and a `.ctl` that `weapon.rlb` does not
  hold), with radii of 2 to 60 m, and 34 of the 62 guns that name a round fire
  one. Four guns' blast is 15 m or more: `e_gun_bl_17` (`bm_b_04`, 60 m and
  100,000), `e_gun_bl_18` and `e_gun_ml_18` (`bm_m_04`, the winged SSM, 45 m and
  60,000) and `e_gun_fl_09` (`fm_h_01`, 15 m and 3,000). Nothing in the data
  keeps a firer clear of them. A gun's target gate has a far limit and none
  near ([29-weapons.md](29-weapons.md#firing-from-button-to-round--read-and-measured)),
  and the nearest thing to a minimum range is the AI's distance score
  ([29-weapons.md](29-weapons.md#how-the-ai-fires--read)): 22 of the 35 carry
  frame flag `0x10` or 8 and score the same wherever the target stands, and
  the other 13 lose their score only over the last 5 m, where 4 of them blast
  wider than that (`ba_a_02` and `ba_a_04` 10 m, `ba_a_05` 15 m, `br_b_03`
  6 m). So the firer test is what stands between such a round and the unit
  that fires it at a target close by.
- **Everything else.** The `objects.rlb` records name 504 distinct `.ndp`
  members, 12 of which their archives do not hold. The 427 that belong to no
  round hold 1,828 nodes: 1,699 name a kind-1 `.exp`, the effect and nothing
  more, 128 name none, and **one names a blast** — node 1 of `o_tur_la_06`, the
  table the turrets `e_tur_lb_06` and `e_tur_lt_06` share, 80 hit points and
  `explode_rbr_bomb`: 20,000 in 75 of the node's radii, which is 43.4 m on its
  0.579 sphere. **0 of the 458 shipped designs carries either turret.** So no
  placed unit's and no building's death is a blast: a tower that dies under a
  unit standing in it plays an effect and hurts nothing.

### What a blast reaches — *read*

**One sphere for the tick's blasts.** The queue walk (`0x10012ce0`) goes over
its hits of kind 3 and 4 first: their centres' mean weighted by their radii
(`0x10012d17`–`0x10012dda`), and about it the radius that holds every one of
them, the largest distance plus radius (`0x10012de0`–`0x10012e42`). One blast
gives its own sphere.

**The world gathers by class and by sphere.** The sphere goes to `CWorld` slot 3
(`Terrain.dll:0x10025f40`, called at `Control.dll:0x10012e82`) with a class mask
of **`0x61c`**: the `1 << class` words of 2, 3, 4, 9 and 10
(`0x10012e44`–`0x10012e6c`) — the `WPNS` objects, buildings, units, rounds and
`STAT` scenery, and not the landscape, which is 1. Slot 3 starts at the world's
root, which is the landscape (the queue's slot 12, `World3D.dll:0x100079d0`),
and walks the tree of objects (`Terrain.dll:0x10025d10`):

- an object is **taken** when its class's bit is in the mask
  (`0x10025d1b`–`0x10025d3a`, the table at `0x1009a5f0`), it answers interface
  `0x18`, and its sphere — slot 9 asked with 2, the agent's sphere in the world
  ([The hit test](#the-hit-test--read-and-measured)) — comes within the sum of
  the two radii of the blast's centre, `d² ≤ (r + R)²`
  (`0x10025d89`–`0x10025dff`);
- taken or not, it is **asked for its children** near the sphere (`IGameObject`
  slot 15, `0x10025ef4`), and each of those is walked the same way.

Slot 15 is one body on the landscape and on an agent (`Terrain.dll:0x1008b020`,
`AniMesh.dll:0x10017e00`), and a building hands the call to its agent
(`Terrain.dll:0x10056ae0`): the object's own interface `0x18` slot 8, with the
centre twice and the radius — a swept sphere that does not move.

- The landscape's (`Terrain.dll:0x1001cc70`) walks the cells of its object grid
  that the sphere covers (that interface's `+0x7a38`, 12 bytes a cell) and
  keeps each object in them whose sphere meets the blast's, once
  (`0x1001cfdd`–`0x1001d090`, `0x1001d0e1`).
- An agent's (`AniMesh.dll:0x100142d0`) goes over the list its mesh keeps of the
  children attached to it that answer `0x18`, and keeps each whose sphere meets
  the blast's (`0x10014388`–`0x100143fe`). The mesh adds a child to that list
  on event 2 of message 21, a child attached, and takes it out on event 3
  (`0x10007378`, `0x1000738f` → `0x1000b620`).

**Then each object's nodes, by their spheres.** Every object gathered but the
exploding one is handed the hit through slot 8 (`0x10012f9a`–`0x10012fb4`), and
each of its nodes takes the falloff above against the node's own sphere,
interface `0x20` slot 3 (`0x100100ea`).

**No line is drawn and nothing stands in the way** — *read*, as a search with a
control. The four routines a blast runs through are the queue walk
(`0x10012ce0`–`0x1001317b`), slot 8 (`0x1000ebc0`–`0x1000ef4b`), the shield step
(`0x1000ff00`–`0x1001001f`) and the falloff (`0x10010030`–`0x10010298`), and
between them they call: the gather; an object's id (`IGameObject` slot 9) and
the object an id stands for (`GetIGObject`); the life system of each object
gathered (interface `0x16`) and its slot 8; a node's sphere (interface `0x20`
slot 3); the shield's sphere, sector and strength (the device list's slots 12
to 15, `+0x38`); a building's door (interface `0x17` slot 14); the node update
(`0x10010f30`), a destroyed node's push and the two walks from a node
(`0x10011130`, `0x10011220`); and the queue's own memory. The world's
segment query is `CWorld` slot 7 (`Terrain.dll:0x10024fd0`), the mesh's is
interface `0x18` slot 6 (`AniMesh.dll:0x10013ef0`) and the landscape's
`GetFirstIntersectedFace` (`Terrain.dll:0x100205c0`), and none of the four
calls any of them; nor do the gather's own routines, which ask slot 9 for a
sphere and nothing that takes a face. Two of the callees were not read through
here: the node's sphere, a getter the gun's gate and a seeker use for an aim
point ([29-weapons.md](29-weapons.md#a-guided-gun-waits-for-a-lock--read-and-measured)),
and the node update, which is handed an amount already settled.

**The control** is the same enumeration over all of `Control.dll`: every call
through a vtable taken from a pointer that was itself loaded from a `+0x44`
field, which is where a control system keeps the world, slot by slot. It finds
slot 7 **once**, at the gun's sight ray (`0x1002a768`); slot 4, gravity, at the
integrator's read (`0x10015887`,
[24-motion.md](24-motion.md#gravity--read-and-measured)); and slot 3 at ten
sites, the queue walk's `0x10012e82` among them. A search that finds the
sight's line where it is known to be drawn, and none in a hit's routines, is
not blind to one there.

So **a blast is stopped by nothing but distance**:

- **A unit in a building's room is reached as one in the open is.** It is the
  building's child while it stands on a face of it
  ([24-motion.md](24-motion.md#walking-into-a-building--read-and-measured)), so
  the walk comes to it through the building's slot 15, and its sphere and its
  nodes' spheres are all that is asked of it: not the roof over it, not the
  walls, not the ground between. A blast of 45 m on a Small Bunker's roof
  gives every node of a hero in the pod room, 17 m from the burst, the whole of
  it, the node's sphere lying wholly inside the blast's.
- **The mask has the rounds' bit.** A round is an agent of class 9 with a life
  system and hit points on its nodes, so a round the walk comes to inside a
  blast takes it like any object, and one whose stage rises goes off where it
  is. Whether the walk comes to a round in the air — whether one hangs in the
  landscape's object grid — was not read.
- **Everything gathered is told who fired**, hurt or not: the message comes
  before the nodes, and the gather's sphere is the object's, not a node's.

### What nothing reads in an `.exp` — *read*, as a search

An `.exp` is 792 bytes on all 144 (*measured*), and every one of them is
accounted for: a 0x18-byte header — kind, damage, radius, **two floats**,
placement word — and twelve 64-byte name pairs, `0x18 + 12 × 64 = 792` exactly.
The two floats at `+0xc` and `+0x10` are **1.0 on all 144**, and **nothing in
the shipped modules reads them**. The search is exhaustive rather than merely
unlucky, and it runs like this:

1. **The bytes only ever reach one pointer.** `.exp` records live in one cache
   in `Control.dll`, the object at `0x10042750`. The index-to-record getter
   `0x1000a200` is called three times in the module, and the other two use the
   `.ndp` cache at `0x10042700` (`0x10008bc0`, `0x10008c56` — 0x4c-byte records,
   which is the `.ndp` stride). So `0x100113db` is the only fetch of an `.exp`,
   and no other module can reach one: `Control.dll` exports `CreateCollManager`,
   `CreateCollObject`, `InitializeSettings`, `LoadControlSystem` and
   `LoadPhysicalModel`, and nothing else, and no module holds the string `.exp`
   at all — the record is found by its `objects.rlb` slot name.
2. **What the fetcher reads.** Tracking the returned pointer through the damage
   stage (`0x100113e0`–`0x10011900`), it is read at `+0x14` (`0x100115d6`, the
   placement word), `+8` (`0x10011749`, `0x1001175c`, the radius), `+4`
   (`0x100117a7`, the damage) and `+0x18`/`+0x38` (`0x100117fa`–`0x10011803`,
   the name pairs) — and nowhere else. It is then stored **at offset 0 of the
   hit record** by the hit's constructor (`0x100129f0`).
3. **Where a hit goes.** A hit is 0x48 bytes and lives in the target's own queue
   at `+0x57c`, count `+0x580`. Every access to that queue in the module is one
   function, the queue walk `0x10012ce0` (`0x10012d17`, `0x10012de0`,
   `0x10012eb2`, `0x10013007`, `0x100130b8`, `0x100130d6`), plus the constructor
   at `0x1000710f` and the destructor at `0x100075f5`. The one interface entry
   that takes a hit is `ILifeSystem` slot 8, `0x1000ebc0` (vtable `0x1003b59c`).
4. **What they read of it.** The hit's `+0` is dereferenced in exactly four
   functions — the blast-centre average `0x10012d24`, the queue walk
   `0x10012f1f`, slot 8 `0x1000ec81` and the blast `0x10010093` / `0x100100ed` —
   and every one of them reads the record's **first dword only**, the kind. `0x1000ff00`, the sector pick, reads the hit's `+0xc` and `+0x10`
   (its object and its node) and never touches the `.exp` at all. The blast's
   `fadd [eax+4]` and `fmul [eax+8]` at `0x10010235`/`0x10010226` are the
   **armour's** linear and square factors, not the explosion's.

**The control**: the same enumeration finds every other field. It finds the
kind at the four hit sites, the damage, the radius and the placement word at the
addresses in step 2, and all twelve names — four of the six scalars and the
whole tail. A search that comes back with that and misses two adjacent floats
between the radius and the placement word is not blind to them; they are not
read.

**What they are for** is a *guess* the format supports and the code does not
settle. They sit between the two magnitudes a hit uses — the damage and the
radius — and the word that steers the effect, and 1.0 on every record is what an
editor writes for a factor nobody touched. A pair of unit multipliers on the
damage and the radius is the reading the layout suggests; a second, unshipped
pair of magnitudes is as consistent with it. Neither is established.

## The hit test — *read*, and *measured*

**One pass a frame.** The world's frame (`World3D.dll:0x10006bf0`) runs in four
steps, with no substeps:

1. It sends every object message 1, the tick; a round moves and spends its
   range here (`Control.dll:0x1000cbb0`).
2. It runs the collision pass once (`Control.dll:0x1001c040`).
3. It sends message `0x1c`.
4. It delivers what the pass posted: message `0x1b` to every object whose
   contact record was filled, which a round takes as its collision response
   (`0x1000d0c0`).

**Who takes part.** Every agent has a collision object: a kind, an owner, a
swept sphere — a start, an end and a radius — and the object's geometry. The
kind comes from the `objects.rlb` tag (`AniMesh.dll:0x1000317f`):

| tag | records | kind |
|---|---|---|
| `BULL` | 67 | 9, a round |
| `BTLU` | 63 | 4, a unit |
| `WPNS` | 5 | 2 |
| `STAT` | 81 | 10 |
| any other | — | the kind of the object it hangs on |

The parent comes first: an agent with a parent takes the parent's kind
whatever its own tag (`0x10003174`), and a building's agent hangs on
`Terrain.dll`'s `CBuilding`, which answers 3 (`Terrain.dll:0x10057da0`).

Only rounds and units carry a contact record.

**The sweep runs from where the object was to where its tick left it** —
*read*. A collision object sets its start and end to its agent's world
bounding-sphere centre on message 1 (`0x1001fec0`) and moves its end there on
message `0x1c` (`0x10020010`). The agent hands message 1 to its components in
a fixed order (`AniMesh.dll:0x10001370`): the behaviour, the wizard, **the
collision object** — start = end = the centre now — then **the control
system**, whose tick moves the object, and then, still inside message 1,
**message `0x1c` to the collision object** alone (`0x1000145a`), which moves
the end to the centre after the move. So when the world's pass runs, the
segment is this frame's motion of the sphere centre, and the world's own
`0x1c` broadcast afterwards only writes the same end again.

**The radius is the mesh header's sphere** — *read*. Each part of an agent
keeps its mesh's stream-2 header — box, sphere, cylinder
(`AniMesh.dll:0x1000a891`) — and the agent's sphere is recomputed from them at
the current pose: the parts' centres weighted by their radii, a radius reaching
the farthest part's sphere, then the centre times each scale and the radius
times the largest (`0x10009510`). That runs at the attach, as each part is
loaded and as one is removed, and at no frame's step
([24-motion.md](24-motion.md#finding-the-ground--read)). Interface `0x18` slot 9 hands that sphere
out, its centre moved to world space and its radius as it is
(`0x10014580`), and message 1 stores that radius. A round is one part, so its
radius is its header's radius — 0.103 to 1.41 on the 67 `BULL` rounds, 0.121
on the hero's cannon shell and laser bolt (*measured*).

**The pass** does three things:

- **A round against the ground.** A round's segment is clipped to the map box,
  whose top is doubled (`0x1001e1e0`), and run through the landscape's
  `GetFirstIntersectedFace` (`Terrain.dll:0x100205c0`). That walks the grid
  cells along the segment from its start, tests each cell's faces with a
  one-sided segment–plane test and a point-in-triangle test, and returns the
  nearest hit in the first cell that has one (`0x1001dbe0`). The cells are
  **the landscape's own**, not a constant
  ([below](#the-query-record-and-what-a-round-excludes--read-and-measured)).
- **Every pair, once.** A pair goes further only if one side has a contact
  record and the two swept spheres touch within the frame (`0x1001e9f0`).
- **A round against an object** (`0x1001d630`):
  - **its own shooter is skipped** — a unit whose id is the round's owner gives
    no contact at all (`0x1001d6af`). The owner is the whole robot that fired
    it, not the gun (`0x1002a3ed`,
    [29-weapons.md](29-weapons.md#the-rounds-start)), so a turret's round never
    strikes its own robot;
  - a unit's **bubble**, its bounding sphere, gives a contact, kept in order of
    distance;
  - then the round's segment is run through the object's mesh.

  No clan is consulted here, and none later: the one object a hit spares is the
  object that fired it, by its id
  ([above](#whose-hit-it-is-and-whom-it-spares--read-and-measured)).

**The mesh test** (`AniMesh.dll:0x10013ef0`) takes every node once, and for
each node the triangles of **level 0 of its current variant**, in the node's
frame (`0x10010a50`): a plane from the stream-7 face normal, one-sided, and a
point in the triangle (`0x10011090`). The nearest hit by squared distance from
the segment's start wins, and the record keeps **the object, the node, the
batch, the triangle and the world point** — on the ground, the face, with the
node −1.

A round's query **passes through triangles flagged 4 or 32**, through batches
flagged 8, and through batches flagged `0x200` unless the round's type carries
`0x4000000` (`Control.dll:0x1001d9fa`, and the record is laid out
[below](#the-query-record-and-what-a-round-excludes--read-and-measured)). Flags 2
and 16 are struck, and **no shipped batch carries `0x200`** — 0 of 15153, so
that exception never fires (*measured*). The fifth
mesh slot and the 28 cockpit nodes that have nothing else are never tested:
the fifth slot is what a unit's own first-person view draws, not collision
geometry ([07-objects.md](07-objects.md#the-fifth-slot-is-what-the-units-own-view-draws)).

|  | *measured* |
|---|---|
| cockpit nodes a round can strike | none — 0 of 28 have a level 0 in any variant |
| level-0 triangles a round passes through | 1306 of 129542, on 30 meshes: trees and the mines |
| `r_h_01` / `r_h_03` / hero `r_h_02` | 4 / 6 / 9 nodes to hit, 104 / 174 / 230 triangles, none passed |

### The triangle test, exactly — *read*, and *measured*

`0x10011090` is the whole per-triangle test, and it borrows both halves from
`Ngi32.dll`.

**The plane half is one-sided and has no epsilon.** It builds the plane from
the face's three vertices and its stream-7 normal (`0x100110d9`), then calls
`g_FastProc` slot `0xb4` — `0x1001d560` on an SSE machine, `0x10024410` on the
FPU path, four builds of the same test (`0x1000344f`, `0x100037e6`,
`0x10003cca`, `0x10004094`). With `v = p₁ − p₀` and `d(p) = n·p + k`, a
crossing needs all three of

    n·v < 0,    d(p₀) ≥ 0,    d(p₁) ≤ 0

each **against exactly 0.0** (`Ngi32.dll:0x10037220`, `0x10037224`; the FPU
path's ties fall the other way and nothing else differs). So a segment that
reaches a face from **behind** meets nothing, and the hit point is
`p₀ + v · d(p₀)/(−n·v)`, the SSE build reciprocating with `rcpss` and one
Newton step.

**A batch flagged 2 is two-sided.** Before the plane test, `0x1001110c` reads
the covering batch's flags dword — the walker keeps the stream-13 record at
its `+0xc` (`0x100081ca`, stride 20) — and where **bit 1** is set it runs the
test a second time with the segment's ends swapped (`0x10011136`). So those
faces are struck from either side. The push-out's face walk reads the same bit
the same way (`0x1000d865`, and see
[24-motion.md](24-motion.md#collision-between-objects--read)). It is set on
**1477 of the 15153 shipped batches** (*measured*), 946 of them in
`static.rlb` and 440 in `fortif.rlb` — the trees and the buildings, whose flat
cross-planes would otherwise be invisible from one side.

**The containment half is `mrnPointInPoly`** (`Ngi32.dll` ordinal 202,
`0x10001e60`), called with the plane, the three vertex pointers, a count of 3
and the hit point (`0x100111ac`). It is **not** barycentric:

- It drops one axis and works in the other two. The axis comes from **two**
  comparisons, not a maximum: `|n.x|` against `|n.y|`, then the loser's
  neighbour against `|n.z|`. So x is dropped when `|n.x| ≥ |n.y| ≥ |n.z|`, y
  when `|n.x| < |n.y| ≥ |n.z|`, and z otherwise — which is not always the
  largest component, but is never a zero one, so the projection never
  degenerates.
- For each of the three edges it computes the 2D cross product of the edge
  with the point less the edge's first vertex, **multiplies it by the normal's
  component along the dropped axis**, and requires the result `≥ 0`. All three
  must pass.
- The comparison is against **exactly 0.0** (`0x100311d4`) with **no epsilon**,
  and `≥` passes, so a point lying on an edge is inside.

Multiplying by the normal's component is the same thing as the full dot
product with the normal, up to a positive factor, for a point on the plane —
so the test is an edge test signed by **the face's own normal**, the one the
plane test used, and not by the cross product of its winding. Those differ on
the 8 mesh faces (of 241887) whose stream-7 normal disagrees with their
winding. `parkan_sim::hit::inside` takes the normal for that reason.

### The query record, and what a round excludes — *read*, and *measured*

Every one of these searches carries the same **eight-dword query**
(`Control.dll:0x1001bd50` fills all eight, `0x1001bca0` the first four):

| dword | what it is |
|---|---|
| +0x00 | the classes to visit, as `1 << class` (`Terrain.dll:0x1002510f`, the table of `1 << N` at `0x1009a5f0`) |
| +0x04, +0x08 | a required and an excluded mask on the object's own word |
| +0x0c | 1 skips the mesh walk's per-part pass (`AniMesh.dll:0x10010a8b`) |
| +0x10, +0x14 | a required and an excluded **flags** mask: mesh batches, or world face flags |
| +0x18, +0x1c | a required and an excluded **class** mask: mesh triangles, or landscape face classes |

`AniMesh` tests +0x10/+0x14 against the batch's flags dword and +0x18/+0x1c
against the face record's first word (`0x100081fa`, `0x1000824a`). The
landscape translates both pairs into **its own** 32-bit face mask — the file's
flags word in the low half, its surface word in the high half — inline in
`GetFirstIntersectedFace` (`Terrain.dll:0x100209a9` for the flags, `0x100208e3`
for the class; the same mapping as `0x10022da0`,
[24-motion.md](24-motion.md#finding-the-ground--read)).

**A round's ground query** (`0x1001d9d0`, built at `0x1001d9fa`–`0x1001da4f`)
is `[0x41e, 0, 0, 0, 0, 0x208, 0, 0x24]`: classes 1, 2, 3, 4 and 10; nothing
required; **excluded world flags `0x208` and excluded class `0x24`**. Against a
mesh those last two are the batch bits 8 and `0x200` and the triangle flags 4
and 32 — what the round passes through, as above. Against the landscape they
become the **flags word's `0x20` and `0x80`** and the **surface word's `0x02`
and `0x01`**.

- Surface `0x02` is the **water surface**, on exactly the 3630 water faces of
  the 33 maps. **So a round's ground test does not strike it**: a shot into a
  lake passes the sheet and splashes on the bed.
- The other three are on **no shipped face** — 0, 0 and 0 of 275882 — and the
  control is the same scan over the same two fields, which finds the 3630
  (*measured*, `openparkan verify`).
- The round's `0x208` is the walker's; only the class differs (the walker
  excludes class 8, the surface word's `0x04`).

**The landscape's own cell.** `CLandscape` finds the cell under a point by
`floor((x − x₀) × inv)` on x and y (`0x100205d5`), `x₀` being corner 0 of the
stream-2 header and `inv` the reciprocal of a size it computes once in its
constructor: the grid is **cells across from the square stream's second count
field in the NRes directory, and its element count divided by that**
(`0x100178e6`–`0x1001794f`), and the cell is **cell 0's own box**, which it
takes from the stream-2 records and inverts at `0x10017c3f`. Measured over the
33 maps: 16 × 16 on 28 and 8 × 8 on 5, agreeing with the distinct cell corners
on all 33; cell 0's box is the map's extent over that grid on all 33; and the
cell is **49.90 world units on map 41 to 311.28 on `SC_3`** — a per-map figure,
never a constant, and nowhere near the 16 the engine's index used to assume.
The engine now indexes the ground over the map's own grid
(`parkan_sim::ground`).

**What the round does** (`0x1000d0c0`):

- **Bubbles.** It walks its bubble contacts nearer than its face hit. Where its
  life exceeds the sector's strength
  ([below](#shields-a-generator-a-deflector-six-sectors--read-and-measured)) it
  passes through and loses that strength (`0x1000d1b4`). Otherwise it stops on
  the bubble, runs its hit action group (`+0x4e4`) and spends all its life.
- **A face or the map edge.** It moves to the recorded point and runs its
  `+0x4e4` group for a face (`0x1000d35b`) or `+0x4e8` for the edge
  (`0x1000d36e`).
- **What the groups do** (*measured* on all 66 rounds, the actions *read* in
  [13-control.md](13-control.md#the-section-5-record--read-and-measured)): the
  face group ends or replays its flight effect (19, 8 or 10) and **kills the
  round** — action 17 on 63, invulnerability off and `ILifeSystem` slot 7, so
  node 0's `.exp` plays and deals the hit; 15 on 3; the edge group always **removes** it (15), with no
  explosion. At the **end of its range** (`0x1000d069`, block entry 4,
  `0x1000d390`) 58 rounds explode node 0 with their own `*_end.exp` (action 27)
  — a puff in the air, or for a missile its full blast — and the other 8 are
  killed or removed.

**Beams are rounds.** A laser flies at 10,000 m/s, but every test is a segment
or a swept sphere over the frame, so nothing is sampled and nothing tunnels. 54
of 66 rounds move further than their own radius even in a 0.01 s tick
(*measured*).

### What a hidden node is left out of — *read*

**Flag 1 of a node's record hides it.** The records are `0x130` bytes, at the
object's `+0x1a8` (count `+0x1ac`), the flags word at `+0x14` and the parent
at `+0x18`. `IAnimation` slot 8 (`AniMesh.dll:0x10005500`) sets and clears the
flag, and with mode bit `0x200` does the same to every node whose parent is
that node, down the tree (`0x100055be`–`0x100055ea`). Slot 7 (`0x100054e0`)
hands the word out. The flag is set by two things:
- a node's last stage ([Hit points](#hit-points--read-and-measured));
- a building's action 1 while it goes up, which action 2 clears
  ([32-builder.md](32-builder.md#actions-1-and-2-hide-and-show-the-building--read)).

**The sweep.** Over `AniMesh.dll`'s `.text`, every `test` or `and` on a
`[… + 0x14]` operand with bit 0 in its mask, and every `bt` of bit 0 there,
finds **9 sites**. The control is the draw's two, `0x10014e57` and
`0x100150a2`, which it finds. A second sweep, for a load of `[reg + 0x14]`
whose bit 0 is tested within four instructions, finds none at `+0x14`; the
same sweep at any displacement finds 21.

The object's interfaces are set by its constructor (`0x10006b38`–`0x10006b5b`).
`CBuilding`'s errors name two of them: *"Could not obtain IMesh2"* and
*"IJointMesh"* (`Terrain.dll:0x10055ee1`, `0x10055fbd`).

| interface | at | vtable |
|---|---|---|
| `IAnimation` (`0xb`) | `+0` | `0x1002057c` |
| `IMesh2` (`0x18`) | `+4` | `0x1002053c` |
| `IJointMesh` (`0x20`) | `+8` | `0x1002050c` |
| `0x25` (unnamed) | `+0xc` | `0x100204fc` |
| `0x26` (unnamed) | `+0x10` | |

The nine sites:

| routine | where | what it is | a flagged node |
|---|---|---|---|
| `0x10014b30` (`0x10014e57`, `0x100150a2`) | `IMesh2` slot 11 | the mesh's draw | is not drawn |
| `0x100101d0` (`0x10010251`) | `IJointMesh` slot 6 | a subtree draw, which hands each node's batches and matrix to the render queue; `CBuilding` draws its parts through it (`Terrain.dll:0x10058b2c`, `0x10058b7a`) | is not drawn; its children are walked, and carry the flag too |
| `0x10010dc0` (`0x10010dcd`) | the node visitor of `IJointMesh` slot 10 (`0x10010a50`), vtable `0x10020a2c` | the **segment query**, which `IMesh2` slot 6 (`0x10013ef0`) runs per node, with the triangle test `0x10011090` | has no triangle tested: no hit |
| `0x100106d0` (`0x100106d9`) | the node visitor of `IJointMesh` slot 7 (`0x10010550`), vtable `0x10020a20` | a **point-inside test** by crossing parity: the triangle test counts crossings (`0x10010995`, `0x100109a5`), and slot 7 answers the low bit (`0x100106a9`–`0x100106bc`). `CBuilding`'s own query uses it (`Terrain.dll:0x1005ab4d`) | answers "not inside" |
| `0x1000ce90` (`0x1000ce99`) | the node visitor of interface `0x25` slot 2 (`0x1000ccb0`), vtable `0x10020954` | the walk-face query with its 0.5 margin ([24-motion.md](24-motion.md#finding-the-ground--read)) | is not stood on |
| `0x10015b60` (`0x10015b69`) | the node visitor of `IMesh2` slot 7 (`0x10013fe0`), vtable `0x10020b84` | the walk-face query along its axis, with no margin | is not stood on |
| `0x1000dfe0` (`0x1000dfe9`) | the node visitor of interface `0x25` slot 3 (`0x1000d410`), vtable `0x10020980` | the push-out ([24-motion.md](24-motion.md#collision-between-objects--read)) | pushes nothing out |
| `0x1001db51` | no vtable | not a node: `[ebp + 0x14]` is a flags argument of a statically linked CRT string-to-integer routine | — |

Earlier this page counted `0x100106d0` into the segment hit test; it is the
point-inside test, and the segment query's visitor is `0x10010dc0` alone.

**The visitors are asked first.** Two node walks call a visitor's slot 0 before
any batch or triangle, and skip the node when it answers 0:
- the flat walk `0x10007e90` (`0x10007f52`), which the push-out, the two
  walk-face queries and the load's `0x1000af10` use;
- the one-node walk `0x10008120` (`0x10008173`), which `IJointMesh` slots 7 and
  10 use.

The one visitor that takes every node, `0x1000b040` (`mov al, 1`), is the
load's. It runs from `IAnimation` slot 2 only (`0x100070c4`, `0x10007268`,
`0x1000728c`), not in play.

**The sphere reads no node.** `IMesh2` slot 9 (`0x10014580`) copies the sphere
that `0x10009510` worked out at `+0x110` and moves it into the world
(`0x100145b5`–`0x100145ec`). Neither sweep hits it or `0x10009510`. So a query
by sphere still meets an object whose nodes are all hidden:
- the command-mode object pick
  ([42-selection.md](42-selection.md#what-a-building-going-up-is-left-out-of--read));
- the world's sphere walk that a building's kill asks (`Terrain.dll:0x10025d10`,
  `0x10025d51`), where the mask takes its class.

**Outside `AniMesh.dll`.** Across `iron3d`, `Terrain`, `Control`, `World3D`,
`Effect`, `Behavior` and `Wizard`, slot 7's word has bit 0 tested at five
sites. One is `Effect.dll:0x100061ba`, an effect's hidden attach point
([11-effects.md](11-effects.md#a-beacon-lights-glow--read-and-measured)).
The other four are in `Behavior.dll`. Each takes a hall-way vertex, asks slot 7
about the vertex's joint, and passes the building over when the bit is set:
- the refit's dock pick (`0x10023d5e`, in `0x10023b60`);
- a factory's creation vertex (`0x10029a71`, in `0x100299a0`), which is
  [36-factory.md](36-factory.md)'s *"Plant creation node destroyed"*;
- the transport's mine, vertex flag 8 (`0x10032ad8`, in `0x10032a00`);
- the transport's storage, flag `0x10` (`0x10032d28`, in `0x10032c50`).

`IAnimation` slot 3's query `0xa`, which also answers the word (`0x10005221`),
is asked nowhere: a sweep of slot-3 calls pushing `0xa` finds none, and the
same sweep finds `Control.dll`'s `0xd` and `0xe` (`0x1000f97f`, `0x1000f9a1`).

**So a building going up**, hidden from its first plan to its code-0 state:
- **is not drawn**;
- **stops no ray.** The world's segment walk (`Terrain.dll:0x100250c0`) asks the
  object for `IMesh2` (`0x1002512f`) and calls slot 6 (`0x1002545e`), which
  reaches `0x10010dc0`. `CBuilding`'s own slot 6 (`0x1005a6c0`) either
  forwards to its mesh (`0x1005a724`) or asks its parts through `IJointMesh`
  slots 7 and 10. Either way a hidden building gives no face. That covers the
  cursor's ray ([42-selection.md](42-selection.md#what-stops-the-cursors-ray--read-and-measured)),
  the sight ray and the outer camera's line. **A round passes through it**
  the same way: its mesh test is `IMesh2` slot 6 (`Control.dll:0x1001da6b`).
  The broad phase still pairs their spheres, but the mesh gives no contact;
- **is not stood on, and pushes no walker out**;
- **is taken by the object pick**, whose kinds table then answers 2, and never
  by its own kill, whose mask has no class 3.

## Shields: a generator, a deflector, six sectors — *read*, and *measured*

A shield is **two parts**: the fight shield (class 9, `i_fsh`) holds six
sectors, and a **deflector** (class 21, `i_def`) decides how much of each stops
damage. The bubble exists only while **both are present, switched on and not
destroyed** (`0x1002c500`).

**Which sector.** A hit on the object that names no node of its own carries
its sector with it — what a round striking the bubble looks like; any other
hit is given one if its sphere crosses the bubble, the object's bounding
sphere, from outside: `r < d < r + R` (`0x1000ff00`). A blast that goes off
inside the bubble meets no shield. The sector is the hit's
dominant axis after the object's matrix is applied (`0x1002c590`):

| sector | 0 | 1 | 2 | 3 | 4 | 5 |
|---|---|---|---|---|---|---|
| axis | +y | −y | −x | +x | +z | −z |
| side | **front** | back | left | right | top | bottom |

**Sector 0 is the front** — *read*, and *measured*. The test first turns the
hit's offset from the bubble's centre into the object's own frame — the
transpose of its matrix, `Ngi32.dll`'s `g_FastProc` slot `0x8c`
(`Control.dll:0x1002c666`) — and ties go to the lower sector. In that frame a
machine moves along +y ([24-motion.md](24-motion.md#the-pieces--read)),
and the models agree: every running-gear node whose name puts `F` beside its
side letter (`LFdd`, `WFRa`, `TMFL` …) rests at y > 0 and every one with `B`
(`LBdd`, `TBR`, `WMLB` …) at y < 0 — 18 and 16 of them — while the left gear
sits at x < 0 ([07-objects.md](07-objects.md)). `openparkan.control.shield_sector`
gives the sector for a direction.

**How much it stops.** A sector's effective strength is

    deflector value i × condition × power level  ×  shield value 0 × the sector's fill

(`0x1002ca30`). A hit takes `min(damage, effective strength)` out of the
sector and **the sector loses that divided by the deflector's coefficient**
(`0x1002ca80`) — so a 0.5 deflector spends two points of shield per point it
stops. What is left of a **blast** goes on to the nodes; what is left of a
**round that struck the bubble** is spent (`0x1000ebc0`). An unpowered or
destroyed deflector stops nothing.

**Recharge** (`0x100257b0`, on the power tick): the charge left after the
shield's idle draw buys `charge / value 2` points, at most `value 1 ×
condition` a second and never more than the six sectors lack; the points go
to each sector **in proportion to what it lacks** (`0x10025a90`). There is no
delay after a hit — no timer in any of the class's methods. **Short of power
the shield drains**: below the idle draw the charge is negative and the six
sectors lose in proportion to what they hold, at up to `power / value 2` a
second.

**A spent sector still meets a round.** The contact is made, but a strength of
0 is below any live round's life, so the round passes and loses nothing
(`0x1000d1b4`).

**The numbers** (*measured*): every fight shield is three values and zeros —
a sector maximum, a recharge a second and the charge a point costs — and every
deflector is six equal coefficients.

| | sector max | recharge /s | charge /point | deflector |
|---|---|---|---|---|
| chassis (`bases.rlb`, 22) | 100–1,000 | 10–100 | 2 | only `r_l_06`, 0.5 |
| `o_fsh_t` / `o_def_t` | 350–1,200 | 4–10 | 0.03 | 0.7–1.0 |
| `o_fsh_l` / `o_def_l` | 1,500–1,950 | 11–17 | 0.04 | 0.7–1.0 |
| `o_fsh_m` / `o_def_m` | 2,500–3,000 | 30–36 | 0.05 | 0.85–1.0 |
| `o_fsh_b` / `o_def_b` | 3,350–3,800 | 50–80 | 0.06 | 0.8–1.0 |
| `o_fsh_f` | 10,000–14,000 | 80–200 | 0.0003–0.0006 | — |
| buildings (`fortif.rlb`, 19) | 8,000 | 80 | 0.00015 | none |
| turrets (`turrets.rlb`, 55) | — | — | — | 0.5 |
| building defences (`u_*_def`, 7) | — | — | — | 0.36 |

The chassis rows are the slots' defaults: a fitted shield generator, deflector
and armour replace them on every shipped robot but the Small Tower, which has no
armour fitted ([28-chassis.md](28-chassis.md#a-fitted-part-takes-over-its-slot--read-and-measured)).
No building controller carries a deflector: a building's shield absorbs only
once one is fitted (the `u_*_def` parts).

**Where the figures sit** (*read*, the component record's values at `+0x2c`): a
fight shield's value 0 is the sector maximum (times `+0x11c`, the level ratio),
value 1 the recharge a second (times its node's condition), value 2 the charge a
point, its power figure the idle draw a second, its resource the hit effect and its
node the node whose life is its condition. A deflector's values 0–5 are the six
sectors' coefficients, read as `values[i] ×` condition × level (id `0x300 + i`,
`0x10021d00`), and its power figure its draw. A fitted generator re-parses its slot
through slot 9 (`0x10025600`), taking the part's figures and effect names and
keeping the slot's node ([28-chassis.md](28-chassis.md#a-fitted-part-takes-over-its-slot--read-and-measured)).

**Who carries one** (*measured*): 61 of the 76 building assemblies carry a fight
shield in their `fortif.rlb` controller and a deflector in a `u_*_def` part of
their own (coefficient 0.36, power 0). The bridges, generators, the mast, the ruins
and `mtp_s_n1` have neither; `teleport` and `mtp_m_n1` a deflector alone.

| (*measured*) | sector max | recharge /s | charge /point | deflector | effect |
|---|---|---|---|---|---|
| `11tin1`, `11tin2` (C01 Mission 02) | 350 | 4 | 0.03 | 0.7 | `r_shield_r` |
| `11smal1` | 1,500 | 11 | 0.04 | 0.7 | `r_shield_r` |
| `hero11` | 1,850 | 15 | 0.04 | 0.9 | `r_shield_b` |
| `12tower` (C01 Mission 03) | 3,800 | 80 | 0.06 | 1.0 | `r_shield_y` |
| `l_bunk1` | 11,000 (`o_fsh_f_01`) | 120 | 0.0005 | 0.36 (`u_bun_def_l_01`) | `r_shield_g` |

### Power — *read*

The bubble's existence asks for no power (`0x1002c500`): both devices on
(state `+0x50` not 0) and their nodes' conditions not 0. **Neither can be switched
off**: no row of `hero.tbl`, `m1.tbl` or `m2.tbl` gives class 9 a state, `CICLS`
names no class 21, and both are built on (class 9 at `0x20`, the generic device
at 5) with no record overriding it (*measured*, 61 and 79 records).

The **level** in a sector's strength is the deflector's `+0x4c`: the share of what
its power channel wants that the channel is served, `min(1, supply ÷ want)`,
written on the power tick (`0x1002dca0`). Classes 9, 10, 21 and 27 share channel
5, served with channel 2 after the engines and channel 0 and before the weapons. A
battery supplies its output × fill × condition a second (`0x100229a0`). The
deflector draws its power × dt while on and whole (`0x10021860`); the generator
draws power × dt + value 2 × min(value 1 × condition × dt, value 0 × (6 − Σ
fills)) (`0x10025700`), and its charge is level × draw − power × dt, a point for
every value 2 of it (`0x100257b0`).

### A round meets a bubble — *read*

- **Which objects.** The pair resolve (`Control.dll:0x1001d630`) gives a bubble
  for a kind-4 object by its swept world bounding sphere and for a kind-3 one by
  its sphere; the round's owner gives none.
- **The touch** (`0x1001e9f0`), with *S* the bubble's radius plus the round's: a
  round whose start lies within *S* of the object's start makes **no contact**, so
  a shot from inside a bubble meets no shield; otherwise the first *t* ≤ 1 at which
  the two spheres touch, moving toward each other, marks contact flag 4.
- **The point** kept, sorted by distance from the round's start: the round's centre
  at *t* moved toward the object's by *r* ÷ (*R* + *r*), on the bubble's surface,
  then carried with the object's rest of the frame. No normal is kept; the sector
  pick works the direction from the centre again.
- **What the round does** (`0x1000d0c0`): a round with `+0x104` bit `0x2000000`
  skips bubbles. With more life than the sector's strength it passes, the object is
  told who fired ([31-packages.md](31-packages.md#a-hit-pulls-a-unit-in--read)), the
  sector is emptied (slot 15 with 0: raw strength becomes *E* ÷ (coefficient ×
  condition × level)) and the round's life drops by the strength. Otherwise it moves
  to the point, runs its hit group and dies, its hit carrying the sector.

### What a shield hit draws — *read*, and *measured*

**The effect is the generator's own resource** (`.ctl` record `+0x6c`/`+0x8c`), not
an action group's. Slot 1 (`0x100254e0`) loads it three times, the same names each
time, as instances `0x20000001`–`3` on the object's node 0.

- **When.** The sector pick (`0x1002c590`, called for every round contact at
  `0x1000d19b` and every blast crossing the bubble at `0x1000ffc1`) plays it when
  the sector's deflected strength is above 0 (`0x1002c83e`): never for a spent
  sector, an unpowered deflector or a unit with no bubble, and also for a round
  that passes through. A round the bubble stops does not flash again: its hit
  carries the sector.
- **Where** (`0x1002c844`–`0x1002c9ed`): a matrix whose first axis is *n*, the unit
  vector from the bubble's centre to the hit, its second (−*n*.y, *n*.x, 0) and its
  third their cross product, standing at the bubble's centre, placed in mode 2 and
  kept on node 0, **scaled by the bubble's radius** on all three axes (`0x10025ca0`,
  manager slots `0x28`, `0x20`), started in time mode 1 (`0x2c`). The instances are
  taken in turn, (index + 1) mod 3, so a fourth hit restarts the first.
- **Its colour** is the generator's (*measured*, the 61 class-9 records): `_df`
  parts name `r_shield_r`, `_01` `_g`, `_02` `_b`, `_03` `_y`; the chassis slots
  `_b` on `r_b_*` and `r_l_07`, `_r` on the other `r_l_*`, `r_h_02` and `r_t_*`,
  `_g` on `r_m_*`; `o_fsh_f_df` and the `fortif.rlb` slots `f_shield_r`. **It does
  not follow the sector's fill** (*read*): nothing edits the names, the instance is
  picked by the counter alone, no value is passed, and each shield material is one
  colour throughout. The HUD's sectors are what run from red to green
  ([35-hud.md](35-hud.md#shields-six-sectors--read)).
- **What `r_shield_<c>` draws** (0.5 s, flags 0): two type-9 emitters,
  `NE_shield_<c>` (fade 1) and the white `NE_shield_w` (fade 0.5), placed in the
  effect's frame (`+4` = 2) moving from 0.9 to 1.0 along *n* and sized
  (0.07, 0.4, 0.4) → (0.077, 0.44, 0.44): a **hemisphere** (`+200` = 0: 8 around by 3
  rings; 1: 16 × 6; 2: 24 × 9, `Terrain.dll:0x100273b0`) flat along the hit and 0.4
  of the bubble across, on its surface; a type-3 `ENV_wave_<C>` quad facing the
  camera at the centre, 2.4 → 2.5 of the radius, fading 0.5 → 0.1, the translucent
  sphere a recording shows (orange for `_R`, (255, 60, 0)); and `hit_shld.wav`,
  heard 10 to 100. `f_shield_*` draws the same at half the dome's size and a wave of
  2.15 → 2.3. Nothing draws while a shield is not hit.

## Armour — *read*, and *measured*

Class 27 (`i_arm`) is three numbers, kept when the part is created
(`0x1002d4b0`): a rating, a linear and a square factor. Every hit on a node
becomes (`0x10010030`)

    min(damage, linear × damage + square × damage²)

so armour takes most off small hits and nothing off a big one — a hit of
`(1 − linear) / square` or more goes through whole. The rating is handed out
as a property and plays no part in the sum; it is a weight per unit of area,
which the mass sum multiplies by every node's area (`0x1000fbac`,
[28-chassis.md](28-chassis.md#what-a-chassis-weighs--read-and-measured)). With more than one class-27 part
the last one created wins.

| (*measured*) | rating | keeps of a small hit | goes through whole from |
|---|---|---|---|
| chassis (22) | 0 | all | — |
| `o_arm_t` | 0.125–3.75 | 85–57% | 1,822–2,713 |
| `o_arm_l` | 2–7.5 | 74–45% | 2,373–2,892 |
| `o_arm_m` | 10–22.5 | 62–34% | 2,688–3,035 |
| `o_arm_b` | 17.5–30 | 51–22% | 2,464–2,790 |

## Repair: a unit's own repair unit, switched on and off — *read*, and *measured*

The **repair unit** a robot carries is its **repair system**, class 15
(`CICLS_REPAIRSYS`, the `i_rps` parts). The **repair mode** you toggle is that
system's switch. Both are about the unit's *own* hit points: nothing in the
code lets a repair unit mend another unit or a building (see "Nobody repairs
anybody else" below).

**It is a switch, and it ships off** — *read*, and *measured*.
- A component's state word is `+0x50`. The repair class's constructor sets it
  to 0 (`Control.dll:0x10022ae0`).
- The component record's `+0x18`, which [13-control.md](13-control.md) calls "an
  index", is really the **initial state**: the parser copies it over the
  constructor's value unless it is -1 (`0x10021d86`).
  - All 64 class-15 records leave it at -1, so **every repair system starts
    switched off**.
  - Other classes do use the field: 33 on the 4 class-24 turret records, 0 on
    the 24 class-25 building records, 9 on the 2 class-29 records.
- The state values are the input layer's own `CIS_` numbers:
  - 0 is `CIS_SWITCHOFF`, `0x20` is `CIS_SWITCHON`.
  - `0x40`, `CIS_SWITCH_INV`, toggles: the class turns `0x20` into 0 and
    anything else into `0x20` (`0x10022c90`).
- **The G key toggles it** — *measured*. `hero.tbl`, `m1.tbl` and `m2.tbl` each
  bind `SCAN_G` to `CICLS_REPAIRSYS MCMD_STATE CIS_SWITCH_INV`, which
  `Command.dsc` calls `CMD_REPAIRSYS_ON`, "Repair on/off".
- **The cockpit announces it.** `iron3d.dll` reads the player's unit's first
  class-15 component and counts `0x20` as on (`0x10076e10`). When that changes
  it plays `VOICE_REPAIR_SYS_ON` or `_OFF` (`0x100a5485`).

**What it does while on** — *read* (`0x10022b20`, `0x10022bb0`).
- **Off, or on a destroyed node:** it draws nothing and repairs nothing.
- **On:** each power tick it asks for its power figure (the *idle* draw) plus
  `value 1 ×` the hit points it would restore. It restores at most
  `value 0 ×` its node's condition a second, never more than the object lacks.
- **Short of power:** after the idle draw, the charge left buys
  `charge ÷ value 1` points (`0x10022bf1`); a charge below the idle draw buys
  none, and a repair with a value 1 of 0 restores what it asked for
  (`0x10022c1c`).
- **What the object lacks** (`0x10010b10`): on a building, its whole life's
  maximum less its life; on a unit, `max − life` over the nodes whose life is
  above 0, so a destroyed node asks for nothing.
- **Where the points go:** to the object's *own* nodes, in index order
  (`0x10010ba0` on the owner, `+0x3c`), each filled to its maximum before the
  next takes any (`0x10010cd4`–`0x10010d4d`).
  - A unit's repair skips destroyed parts; a kind-3 object's restores them
    (`0x10022b00`).
- **How a building brings a part back** — *read*.
  - The repair system's constructor sets `+0x94` when its owner's agent is of
    kind 3 (`0x10022b0e`). It passes that byte both to what the object lacks
    and to the node update (`0x10022c2e`, `0x10022c79`).
  - With it set, the gain loop gives every node its share, a destroyed one
    included (`0x10010d07`). Each node takes `min(its max − its life, what is
    left)` through `0x10010f30`. The loop takes that off what is left and stops
    once nothing is (`0x10010d47`–`0x10010d5c`).
  - A node whose life is above 0 afterwards loses its destroyed bit, `0x10`
    (`0x1001111e`).
  - The stage walk from node 0 follows at once (`0x10010d78`):
    - A node no longer destroyed that was flying, carried or brought down
      with its parent (`0x1c0`) has those bits cleared. It is handed back to
      its mesh (slot 8 with `0x101`, `0x100112e8`).
    - Its stage is recomputed from its life. A stage falling from the last,
      the one that hid it, shows it again (`0x100118d8`, `0x10011920` with 1).
  - So **a building rebuilds what it lost one node at a time, in index
    order**, each whole before the next gets a point. The parts a socket
    carries come after the socket, so a turret comes back before the guns on
    it.
- **On C02 M04's plateau Light Tower** — *measured*, then *derived*. `mtow02`'s
  repair system gives 110 a second. Its turret carries 3,506 hit points, and
  each of its two `e_gun_fc_01` cannons 6,000. With the turret shot off:
  - the turret is whole again in 32 s;
  - the first cannon at 86 s;
  - the second at 141 s.
  - This is how the player remembers it: "first the turret then one of the
    cannons, then another one" (reported from play, 2026-09-30).
  - The turret has life again within the first second, and the component
    test asks only that it has some
    ([27-ownership.md](27-ownership.md#what-0x10033e40-refuses-on-a-tower--read-and-measured)).
    So its pod opens the guns again almost at once.
- **Running cost:** left on at full health it costs only its idle figure.
  Being a class-15 part, it draws on power channel 0, served second.
- **No reach:** values 2–15 are zero on all 64 records (*measured*), so there
  is no range, no target and no radius. The catalogue agrees: all 16 `i_rps`
  entries in `objects.dlb` show a single **"Regeneration", in HP/s**.
- **What that row prints** — *read*, and *measured*. Its field, `regener`, is
  the stat panel's field 13 (`iron3d.dll:0x1006f070`), which asks the part's
  interface `0x202` slot 3 for id `0x77` on device 0 (`0x1006f62c`). That id
  is component query `0x1100` (`Control.dll:0x1002e887`): on a class-15 device
  **value 0 × its condition**, the points a second; on a class-9 fight shield
  value 1 × condition, the recharge a second; nothing on any other class
  (`0x1002c1b6`). Only the 16 `i_rps` entries use the field, and each
  `o_rps` controller's first device is its class-15 system (*measured*), so
  the row always shows the repair rate — the table below, for a new part.

**The numbers** — *measured*.

| part | Regeneration, HP/s by mark | charge a point | idle draw a second |
|---|---|---|---|
| `o_rps_l` small | 11 / 13 / 15 / 17 | 0.04 | 0.08–0.11 |
| `o_rps_m` medium | 30 / 32 / 34 / 36 | 0.05 | 0.18–0.21 |
| `o_rps_b` large | 50 / 60 / 70 / 80 | 0.06 | 0.27–0.31 |
| `o_rps_f` fortification | 80 / 110 / 150 / 200 | 0.0006 → 0.0003 | 0.01 |
| a chassis's own slot (`bases.rlb`, 22) | 1 | 1 | 1 |
| a building's own (`fortif.rlb`, 26) | 100 | 0.0002 | 0.01 |

- **Who carries one:** 372 robot assemblies, all but two, carry exactly one
  `i_rps` part. The two without are the target dummies `l_targ.dat` and
  `M_targ.dat`, on chassis `R_H_01` ("Hero target") and `R_H_03`. So do 70 building assemblies; the power mast, the four ruins and
  the small main teleport don't.
- **A unit uses the fitted part's values.** Every chassis's controller declares its
  class-15 slot under the part's label with 1, 1, 1, and the fitted repair unit is
  re-parsed into that slot, replacing them
  ([28-chassis.md](28-chassis.md#a-fitted-part-takes-over-its-slot--read-and-measured)).
  All 372 turreted robots fill it (*measured*).

What a full repair costs — *derived*, for a Small Wheel chassis (`r_l_03`, 2,621
hit points over 12 nodes) brought back from the edge with no part destroyed,
at full power and with the repair unit's own node intact:

| with | time | charge |
|---|---|---|
| a small MK1 repair unit | 238 s | 105 for the points + 19 idle ≈ 124, 4% of the small MK1 battery's 3,000 |
| a small MK4 | 154 s | 105 + 17 ≈ 122 |

**What the AI does with the switch** — *read*. The earlier note that nothing
reads `Decision_RepairOn`/`Off` was wrong.
- **Where the thresholds live.** A behaviour keeps its difficulty block at
  `+0x8d4` (bound at `Behavior.dll:0x1000a2f4` by `0x10019bd0`, read through
  `0x100146a0`); `Decision_RepairOn` is its `+0x10`, `Decision_RepairOff` its
  `+0x14`. **Every unit holds the block's defaults, 0.5 and 1**: no `diff_*`
  profile is ever loaded
  ([The difficulty block](#the-difficulty-block-every-behaviour-holds--read-and-measured)).
- **The decision** (`0x10017c70`) reads the object's life fraction and battery
  charge, then:
  - **switches repair on** when the unit *needs service* — life under 0.5, or
    under 0.9 for a building (`0x1001c700`) — *or* its life is under
    `Decision_RepairOn`, **provided its charge is over 30%**;
  - **switches it off** when it does not need service and its life is over
    `Decision_RepairOff`, **or whenever its charge falls under 10%**.
- **How the switch is applied.** It goes through the behaviour's device
  manager (`+0x3d0`). That manager collects every class-15 component into one
  list (`0x100186b0`) and sends each `CIS_SWITCHON` or `CIS_SWITCHOFF`
  (`0x10019a80`). Its sibling sends the detection shield's camouflage on and
  off the same way (`0x10019a10`).
- **Starting any task turns repair off**, and camouflage too
  (`0x10034930`). The next decision turns repair back on if it is needed. The
  manager remembers the last state it sent (`+0xaf`) and sends nothing when
  asked for the same again, so a switch the player flipped by key in between
  is not seen by it.
- **When the decision runs.**
  - A building runs it on its takt timer (`0x100054a0`).
  - A unit runs it on its takt (`0x10005110`) unless that tick sent it to a
    dock or into an attack (`0x10017d50`, `0x10017e70`).
  - The unit takt needs bit `0x10` of the behaviour's flags, which the
    behaviour's mode setter derives from its argument (`0x100067b0`). **The
    unit's `Wizard.dll` object sets it** (`0x10003890`), and taking a bot
    clears it unless its auto-driver level is 1 or 2 (`iron3d.dll:0x10074ff0`,
    [31-packages.md](31-packages.md#the-escape--read)). So **while the player
    drives a bot at level 0 the AI does not touch its repair switch**; at level
    1 or 2 it does. The player's hero has the bit forced off whether driven or
    let go.
  - Clans of type 3 skip the takt altogether (`0x10005070`).

| block | `Decision_RepairOn` | `Decision_RepairOff` | a unit switches on below | and off above |
|---|---|---|---|---|
| **every unit: the defaults** (*read*) | 0.5 | 1 | 50% | never, but for a flat battery |
| `diff_strong`, never loaded (*measured*) | 0.8 | 0.9 | 80% | 90% |
| `diff_normal`, `diff_slow`, `diff_stupid`, never loaded | 0.5 | 0.8 | 50% | 80% |
| `diff_weak`, never loaded | 0.1 | 0.3 | 50% (needing service overrides) | 50% |

The last two columns are *derived* from the rule above: no life is over 1, so a
unit's repair system, once on, stays on until its charge falls under 10% or a
task starts. A building needs service below 90%, so it repairs below 90% while
its charge allows.

**Nobody repairs anybody else** — *read*, as a search. Everything in the
shipped code that raises a node's life:
1. **The repair system**, on its own object only (`Control.dll:0x10022c87`).
2. **Setting the life fraction, property `0x31`**, through `ILifeSystem`
   slot 6 or the control system's main interface, slot 14 (`0x1000e980` →
   `0x1000e9c2`, which also
   restores destroyed parts). Across all twelve modules, the only callers
   that pass `0x31` are:
   - `Behavior.dll:0x1001816a`, the **dock**, on the units standing in it
     ([27-ownership.md](27-ownership.md));
   - `0x1001c69b`, which fills the behaviour's **own** object to full — every
     takt while `Behavior.ini`'s `DeterminMode` is set or a demo records or
     plays back (`0x10004e1f`), and on one internal message (`0x1000934c`);
   - `Control.dll:0x1000b382`, an object property handler whose ids 0 and 1
     set invulnerability and life (what calls it is not traced; mission
     properties, by the look of it — *guess*).
3. **The other callers of the node update** (`0x10010ba0`) all pass damage:
   collision (`0x1000d212`, `0x1000d2f9`), the ground (`0x10012a7e`) and the
   **wreck's burn** (`0x10012c37`) — an object whose total life has fallen
   below the mark at `+0x594` loses 0.1 of that mark, and the mark follows it
   down ([Vegetation and rock](#vegetation-and-rock-carry-node-life--read-and-measured),
   where the mark's 0.3 for scenery against 0.2 for everything else is what
   settles that a tree has a life system at all). The ground's loss is shared: every node gives
   up the same share of its own life, and none is destroyed until all of them
   are ([24-motion.md](24-motion.md#water-and-lava-beds-kill--read-and-measured)).
4. **A hit cannot heal**: the armoured damage is clamped at 0
   (`0x10010253`), and every one of the 144 `.exp` files does 0 or more
   (*measured*).

The orders say the same.
- **Orders 8 and 9 build the same task.** `ORDER_ROBOT_RELOAD` (8) and
  `ORDER_ROBOT_REPARE` (9) share one case in `MTaskStack::CreateTaskFromOrder`
  (`0x10033a80`, table `0x100344b0`): `M_Task_Reload`, a trip to a dock.
- **No repair-another order exists** among `varset.var`'s orders.
- **`Task_Repare` does nothing.** The behaviour-profile flag is bound with
  the other `Task_*` flags (`0x10022e20`, struct `+0x820`, `Task_Repare` at
  `+0x2c`), but nothing reads it. The struct has no direct reads, and all 17
  calls of its getter `0x10014680` read the ore and power fields at
  `+0x54`–`+0x68`.

What *can* restore someone else is a dock: a unit standing in one gains 10% of
its full hit points a second, destroyed parts included, plus 10% of its
shield, battery and ammunition (`Behavior.dll:0x10018100`, `0x10019372`,
`0x100181e0`).

## Not established

- Which of the effect frame's axes a type-9 dome's pole ends on: as read
  (`Effect.dll:0x1000d110`, a default direction (0, 0, 1)) its second, so the
  flash's dome would stand across the hit rather than bulge toward it; not checked
  against a recording. Where a round's `+0x104` bit `0x2000000`, which skips
  bubbles, is set; the height below which a building's own collision context drops
  a contact (`0x1001d846`); whether the deflector parts' own `deflector` and
  `tur_deflector*` load effects loop.
- ~~Which of ±x, ±y is a model's front.~~ Answered: +y, sector 0
  ([Shields](#shields-a-generator-a-deflector-six-sectors--read-and-measured)).
- ~~How a collision object's start and end differ when the pass runs.~~
  Answered: the agent's centre before and after its control system's tick
  ([The hit test](#the-hit-test--read-and-measured)).
- ~~How the struck object and node reach `ILifeSystem` slot 8's hit.~~
  Answered: the node stage copies the first three integers of the round's
  contact reference ([A hit](#a-hit-from-the-round-to-the-node--read)).
- ~~What reads a node's fifth slot, if a round's hit test does not.~~
  **Answered**: the mesh draw, for a view the turret's camera component
  registered with the unit's mesh — the first-person view draws every node's
  fifth slot (`AniMesh.dll:0x10014be5`). See
  [07-objects.md](07-objects.md#the-fifth-slot-is-what-the-units-own-view-draws).
- ~~Where a round's collision radius comes from.~~ Answered: its mesh's
  stream-2 header sphere, times the largest scale
  ([The hit test](#the-hit-test--read-and-measured)).
- ~~Who clears the behaviour flag `0x10` that lets a unit's takt switch its
  repair.~~ The unit's wizard, off while the player drives at auto-driver
  level 0 ([31-packages.md](31-packages.md#the-escape--read)).
- ~~What `IControl` component query `0x77` answers.~~ Answered: component
  query `0x1100`, a repair system's value 0 × condition
  ([Repair](#repair-a-units-own-repair-unit-switched-on-and-off--read-and-measured)).
- The two 1.0 floats of an `.exp` (`+0xc`, `+0x10`; 1.0 on all 144,
  *measured*). **Nothing reads them**, and the search is now finished rather
  than abandoned — see [What nothing reads in an `.exp`](#what-nothing-reads-in-an-exp--read-as-a-search).
- ~~What agent kind 3 is.~~ Answered: a building. What becomes of one whose
  node 0 is destroyed is narrowed: its behaviour stops for good and the object
  stays ([Hit points](#hit-points--read-and-measured)), and what `iron3d.dll`
  does with its `0xfffe` owner word is now read too: it skips it, everywhere
  ([27-ownership.md](27-ownership.md#the-37-compares-in-iron3ddll--read)).
- ~~What `iron3d.dll` does with an object whose owner word is `0xfffe`: 37
  compares with the value, one read (`0x1007dee0`, which skips it).~~
  Answered: all 37 are the same idiom — skip a dead object — spread over the
  lists and pickers, the HUD, the target list, the mode stack and the two
  component tests, and the value means *not a clan index*: 7 of the 37 stand
  within a dozen instructions of an `imul …, 0x68` into the clan table. Nothing
  authors it: 0 of the 864 placements of the 29 shipped missions carries it
  ([27-ownership.md](27-ownership.md#the-owner-word-and-0xfffe--read-and-measured)).
- ~~What a destroyed node draws, and what a dead unit leaves.~~ Answered: a
  node draws the block of its stage held below its stage count; at the last
  stage it is hidden from the draw, the ground, collisions and hits unless its
  mesh flags carry `0x100`; a destroyed part is first knocked off and flies
  for three seconds; a dead unit is deleted the controller's `+92` ms after it
  dies ([What a damaged node, a destroyed part and a dead unit draw](#what-a-damaged-node-a-destroyed-part-and-a-dead-unit-draw--read-and-measured)).
- Whether Behavior's hall-way searches also pass over a building whose sphere
  still runs. They pass over one whose place hangs on a hidden node: the
  refit's dock pick, a factory's creation vertex, the transport's mine and its
  storage ([What a hidden node is left out of](#what-a-hidden-node-is-left-out-of--read)).
  A new building is shown at 40 s and its sphere ends at 41 s, and whether
  those searches read its `0x20c` in that second was not read.
- Whether a blast reaches a hidden node. A kind-3 hit hurts every object
  whose bounds reach it and then each node by its sphere (`0x10010030`). No
  `AniMesh.dll` routine that tests node flag 1 is on that way, and whether the
  life system's own walk passes a node hidden by action 1, which is not a
  destroyed node, was not traced. Narrowed: the falloff's own tests are the
  life record's destroyed bit `0x10` on the node and on its parent and a sphere
  of radius 0 (`0x10010052`–`0x1001008d`, `0x10010101`), so what is left is what
  interface `0x20` slot 3 answers for a node hidden by action 1
  ([What a blast reaches](#what-a-blast-reaches--read)).
- ~~Where the test stands that makes a hit do nothing to "the target's own",
  and what it compares.~~ Answered: `Control.dll:0x1000ed4e` for a blast and
  `0x1000ee0c` for a direct hit, the target's object id against the id the hit
  carries at `+0x18`, the round's property `0x7f` or the exploding object's own
  id; after the shield step, before the first node, and with no clan in it
  ([Whose hit it is, and whom it spares](#whose-hit-it-is-and-whom-it-spares--read-and-measured)).
- ~~Whether a blast's reach is tested by anything but bounds: a line, a face,
  the building over a unit that stands in it.~~ Answered: by nothing else. The
  world gathers by class and sphere down the tree of objects, the nodes are
  tested by their spheres, and no segment query is on the way
  ([What a blast reaches](#what-a-blast-reaches--read)).
- What a unit does when message `0x19` names itself, as it does when its own
  blast reaches it: the handler is
  [31-packages.md](31-packages.md#a-hit-pulls-a-unit-in--read)'s, and its test
  of the firer's clan is what would let it pass. And whether a round in the air
  hangs in the landscape's object grid, so that a blast's walk comes to it: the
  mask has its class.
- How a knocked-off part flies: the push and spin it is given (`0x100102a0`,
  its subtree's box and mass from `0x10010760`),
  how its update integrates them, and what the world query at `0x100134c1`
  asks before it ends the flight.
- What `+0x104` bit `0x10000000` is, which spares an object the death timer,
  and what message `0x15` with 7 does at the object it is sent to.
- Who sends an agent message 6 with `0x16`, which deletes it through
  `KillGameObject` from `AniMesh.dll` (`0x10001602`).
- What the mesh node flag `0x10` means: it is on `ASd1`, `ASd3` and every part
  of `r_h_03`, and the life loader does not read it. It is a **broad** mark —
  861 of the 1845 mesh nodes, in all nine archives (*measured*) — where the
  vital `0x200` is narrow, so the two are not the same kind of thing; they
  coincide on 8 nodes, each of them the leaf of a vital chain.
- ~~Which node flag makes a node vital, and whether AniMesh query `0xe` is the
  mesh node's flags word.~~ Answered: query `0xe` is the mesh node's stream-1
  flags word, read at a 38-byte stride (`AniMesh.dll:0x10005242`), and the bit
  is `0x200` (`Control.dll:0x1000f9aa`), on 90 of the 1845 nodes
  ([What the loader takes from the mesh](#what-a-damaged-node-a-destroyed-part-and-a-dead-unit-draw--read-and-measured)).
  What is **not** established is why it falls on the hero chassis, the
  Transformer, the L-7f, the four animals and two turrets and on no other
  walker's legs.
- ~~Whether vegetation and rock carry node life.~~ Answered: they do, through
  the same control system as a robot, and the placement's scale cubes it
  ([Vegetation and rock](#vegetation-and-rock-carry-node-life--read-and-measured)).
  What the low-life mark at `+0x594` is worth — the effects `+0x4f4` and
  `+0x4f8` it starts and stops, and whether the burn is per tick or per second —
  is read only in outline.
