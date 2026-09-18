# Designs — what the robot constructor offers, fits, rates and writes

The robot constructor is the full-screen editor a factory opens ("Use warbot
constructor to build bots"). Its screen — pages, rows, buttons, previews — is
[37-designer.md](37-designer.md). This page is the design itself: which parts
each page lists, what fitting a part does, every number the unit box and a
part's box print, the design's name, the file an accepted design becomes and
what building it costs.

`openparkan.designs` implements it; `openparkan verify` re-derives it
(`check_designs`), including every unit box the recording of Mission 02 shows.

**Every claim is tagged**, as in [23-economy.md](23-economy.md): *measured*
is re-derived by `openparkan verify`, *read* comes from the disassembly at the
address given, *derived* follows from both, *seen* is the recording
(`training mission 2 The constructor`, 960 × 720; the designer is up from
107.5 s to 155.5 s).

## Sockets carry the part prefix they take — *measured*, and *read*

A mesh's **stream 10** is one label a node: a `uint32` length, then that many
bytes and a NUL (none when the length is 0). [07-objects.md](07-objects.md)
had it as one zero `uint32` a sub-object, which is what an empty label looks
like: every `static.rlb` mesh has only empty ones.

- **549 of 549** stream-10 members in `objects.rlb`'s meshes parse exactly;
  548 have one label a node (`mtcheck` has one label and no node) (*measured*).
- `AniMesh.dll` hands them out: its main interface's slot 19 is the node
  count (`0x10007e70`, `+0x1ac`) and slot 6 node *i*'s label
  (`0x100054b0` → `0x10012530`, the node's mesh `+0x20` walked
  length-prefixed), and the constructor asks slot 6 of every node of a chosen
  chassis (`iron3d.dll:0x10051e5a`) (*read*).

The labels in use (*measured*):

| on | labels | what they name |
|---|---|---|
| a chassis's turret socket | `e_tur_tt`, `e_tur_tb`, `e_tur_lt`, `e_tur_lb`, `e_tur_mt`, `e_tur_mb`, `e_tur_bt`, `e_tur_bb`, `e_tur_ht` | the turrets of its size, `t` standing, `b` hung under a flyer |
| a turret's gun socket | `central_<s>c`, `universal_<s>l`, `universal_<s>r`, `universal_<s>s` (`<s>` t, l, m, b, f) | the guns `e_gun_<s>c`, `e_gun_<s>l`, both, or `e_gun_<s>s` |
| buildings | `e_bnt_*`, `e_tow_*`, `u_*_def_*`, `u_min_upg_*` | a building's turret, defence and upgrade |

**The shipped assemblies keep to them** (*measured*): 371 of 372 turrets start
with their chassis socket's label (the exception, `42_mons.dat`, sits on
`r_b_07`, whose socket has none), and **865 of 865** guns take the prefix
their turret socket's label gives.

**The label is the only statement of what a socket takes.** A turret mesh's
labelled nodes are exactly its `Base_*` nodes bar the mount, on all 55 turret
records, so the node names pick the same sockets — but the node name says only
where the socket sits, never its kind, and no DLL names a `Base_*` node at all.
The Large builder's module socket is `Base_LU_02`
labelled `universal_bs`, while its `Base_LU_01` is a `central_bc` cannon
socket; the small and medium builders have it the other way round
([30-turrets.md](30-turrets.md#gun-sockets-are-the-meshs-base_-nodes-and-their-kind-is-the-label--measured-and-read)).

## The catalogue — *read*, and *measured*

**A page lists every part whose id starts with its prefix and whose research
item is in the tree and researched** (`iron3d.dll:0x1008a780`):

- It walks the clan's research tree's `TRFB` part list in order
  ([16-research.md](16-research.md)), matching names by prefix at `0x1008a875`
  (`0x10090900`, a case-insensitive compare with `?` wildcards).
- `0x1008a879` tests a byte argument, and **clear it keeps the part with no
  further test**. All five call sites fill it with the inverse of `0x1008ac50`,
  which is `atoi` over `Iron_3D.ini`'s `[CS] FULL_RESEARCH_TREE`
  ([22-settings.md](22-settings.md)) — a key the shipped file does not carry.
- Otherwise `0x1008a884` calls `IResearch` slot 3 (`MisLoad.dll:0x10002aa0`),
  which unpacks the item's category byte into a record — `+0x00` bit 2, `+0x04`
  bit 1, `+0x08` bit 4, then the four cost dwords and the part-id pointer — and
  `0x1008a896` and `0x1008a89e` require `+0x00` and `+0x08`. So the page keeps
  a part whose item is **researched and in the tree**; the `AVAILABLE` bit slot
  3 also writes is read by nothing here, and none of the four costs is
  compared.
- Rows keep the `TRFB` order. A page with several prefixes still walks the
  tree once, testing each part against each prefix (`0x1008a9a0`). A page
  holds at most 64 rows (`0x1004878b`).
- `objects.dlb` is asked only whether a member exists, and a row reads
  "`name (code)`" from `TRF8` and `TRF7` ([19-descriptions.md](19-descriptions.md)).

**The prefixes** (the page builder `0x10048220`, given a row's name):

| page | prefix |
|---|---|
| chassis (the name `r`) | `r_t`; `r_t`, `r_l`; … up to `r_b` as the grade runs 1 to 4 (the switch at `0x10048b21`, its table at `0x10049398`). **`r_h` is no prefix at any grade** |
| turret | the chassis socket's label, `e_tur_bb`, passed through unchanged once the tab's name starts `e_tur_` (`0x10049126`) |
| weapons | `e_gun_` and the socket label's last two letters, found from the label's length; both `<s>c` and `<s>l` when the last is `r`, cannons listed first (`0x100482f2`–`0x100483c1`) |
| armour, internal systems, ammunition | the slot's label from the part's controller, `i_arm_b`, `i_eng_b`, `i_c15_b` |

**The grade is the factory's size.** The factory screen opens the constructor
through `0x10055c70`, which keeps its fifth argument at `+0x18`
(`0x10055ca6`). The factory passes its own unit record's `+0x30`
(`0x1009810a`), the record field [31-packages.md](31-packages.md) reads as 1 to
2 for a small unit and 4 to 5 for a big one. That it is the building's size
letter — small 2, medium 3, large 4 — is *derived*: it gives the rule of
[23-economy.md](23-economy.md) that a factory builds chassis up to its own
size.

**Mission 02's player is offered 32 parts** (`MISSIONS/SCRIPTS/tut2_pl.trf`,
*measured*):

- one chassis, `R_B_02` Large Flying Chs (L-2f);
- one turret, `e_tur_bb_01` / `e_tur_bt_01` Large Battle Turret (4L1);
- two guns, `e_gun_bl_15` Large Rocket Lr (LRL36S) and `e_gun_bc_06` Large
  Flame Thrower (LFT);
- their clips `i_c15_b_df`, `i_c06_b_df`, `_01`, `_02`;
- the large internal parts in marks 1 and 2: engine `i_eng_b`, battery
  `i_pws_b`, shield `i_fsh_b`, detection shield `i_dsh_b`, repair `i_rps_b`,
  radar `i_rdr_b`, deflector `i_def_b`, and armour `i_arm_b` in marks 1 to 3;
- six brain modules no page lists.

Every part the recording's designer shows is among them (*seen*, and
*measured*).

## Fitting — *read*, and *measured*

**What a chosen part does** is picked by its research record's kind bytes
(`0x100519e0`, `0x1008a690`):

| part | category | fit |
|---|---|---|
| chassis (kind 9, sub-kind 32) | 0 | `0x10051bb0` |
| turret (9, 33), a building's turret, defence or upgrade | 1 | `0x10052570` |
| gun (kind 12), a building's radar | 2 | `0x10052fb0` |
| device (kind 11) | 3 | `0x10052d10` |
| ammunition (kind 10) | 4 | `0x10053510` |
| armour (11, 68) | 7 | `0x100537b0` |
| a building, a brain (11, 69) | 5, 6 | nothing |

**Choosing a chassis starts the design** (`0x10051bb0`):

- It builds the preview object ([37-designer.md](37-designer.md)) and asks it
  for its animation interface (`0xb`) and item manager (`0x202`).
- Every node's label makes a row of the turret page (`0x10051e50`).
- Every labelled slot of its controller, in slot order, gets a row. A slot
  labelled `i_brn_` is skipped (`0x10052017`); an `i_arm_` slot goes on the
  armour page; the rest on the internal systems page.
- **Each such slot is filled with the part named `<label>_df`**, when that part
  is offered (`0x100520bc`, the same collector over the one name).

**Choosing a turret or a gun fills its slots the same way**: `_df` appears in
the turret fit (`0x10052a92`, radar and deflector) and the gun fit
(`0x100532ed`, its clip) (*read*).

- Every labelled slot of every shipped chassis, turret and gun, brains aside,
  has a `_df` part: 270 of 270 (*measured*).
- The recording's weights come out only with the defaults fitted: the L-2f
  alone reads 26 t, not its body's 10 t (*seen*, re-derived below).

**Size never has to be checked.** A slot takes parts by its label, which
carries the size, and a socket by its label, so a design cannot hold a part
of the wrong size or kind. Nothing else is compared (*derived*).

**Accepting needs a turret and spare payload** (*read*, `0x10050409`):

- The accept button is enabled when a design exists and its turret is fitted.
  The turret fit sets `+0xad7f` (`0x100525a4`).
- The unit box must also report spare payload above 0 (`0x1007068e`).
- So **a design at or over its payload cannot be accepted** — the red
  "66 / 0 t" of the recording — while "65 / 0 t", 0.47 t spare, can.
- The save button needs only a design (`0x100510a8`).

## The unit box — *read*, and *measured*

`iron3d.dll:0x1006fc00` prints five lines about the preview object: labels in
green `0xff00ff00` ("Weight: ", "Max. speed: ", "Defence: ", "Offence: ",
"Sensor range:", strings 5069–5073), figures in lavender `0xffb4b4ff`. Every
figure is a property of the live object, so the design is rated by the same
code that runs the unit.

| line | printed | property | value |
|---|---|---|---|
| Weight | `"%-.f / %-.f"` t, both red `0xffff0000` when the second is not above 0 | 124 × 0.001, 137 × 0.001 | the total mass, and the spare payload |
| Max. speed | `"%-.f"` kph (6176) | 145 × 3.6 | the live top speed |
| Defence | `"%d"` % | 177, normalised | below |
| Offence | `"%d"` % | 178, normalised | below |
| Sensor range | `"%-.f"` m | `0x50` | the fitted radar's value 3; 0 with none |

**Mass and spare payload** ([24-motion.md](24-motion.md#load--read-and-measured)):

- total = Σ nodes (density × level-0 volume + armour rating × area) + Σ device masses;
- spare = max(0, payload (+124) + the chassis's own nodes' density × volume − total).

**Top speed** ([24-motion.md](24-motion.md#what-sets-the-live-limits--read)):

- live = min(top, top × E × (1 + spare ÷ payload) ÷ 2);
- *top* is the chassis's authored forward speed (+48);
- *E* is the fitted engine's value 0 (0.7 on a mark 1, 0.8 on a mark 2);
- the ground factor is 1 off the ground.

**Defence** — `Control.dll:0x10013940`, the machine's property 177
(`0x1000e735`):

- the maximum life of the first node with any level-0 area, over the fitted
  armour's value 1 (the share of a small hit it keeps, [26-damage.md](26-damage.md#armour--read-and-measured));
- plus, when the unit has both a shield generator and a deflector, the
  deflector's first coefficient × the generator's sector maximum (device id 16,
  `0x1002b99d`).

**Offence** — property 178 (`0x1000e7c7`), device id 6 (`0x1002b62b`):

- Σ over every gun: its round's damage × 1000 ÷ max(1, its interval);
- the round and the interval are the clip's, once a clip replaces the gun's
  firing record.

**The two percentages** are normalised (`0x100700e3`, `0x100702f5`) when
`Iron_3D.ini`'s `[TEMP] NORMALIZE` is set, as shipped:

- 0 at or below MIN, 100 at or above MAX, else round(100 ÷ (MAX − MIN) × (v − MIN));
- `OFFENCE_MIN` 0, `OFFENCE_MAX` 6550, `DEFENCE_MIN` 0, `DEFENCE_MAX` 24400
  (read into `0x1010bf60`–`0x1010bf70` by `0x1006f6d0`);
- with `NORMALIZE` 0 the raw figure is printed.

**The recording, re-derived** (*measured* against *seen*). Every unit box the
designer shows is reproduced line for line from the parts on screen, 17 of 17
states, red included:

| state | parts | seen |
|---|---|---|
| 108.5 s | L-2f, its `_df` parts | 26 / 39 t, 66 kph, 12 %, 0 %, 0 m |
| 109 s | + 4L1 and its `_df` radar and deflector | 37 / 28 t, 58 kph, 23 %, 0 %, 350 m |
| 111 s | + two LRL36S, two LFT, their `_df` clips | 58 / 7 t, 43 kph, 23 %, 25 %, 350 m |
| 113.5 s | armour mark 3 | 62 / 3 t, 40 kph, 27 %, 25 %, 350 m |
| 115.5–117 s | engine, battery, shield, detection shield, repair mark 2 | 64 / 1 t, 45 kph … 66 / 0 t (red), 44 kph |
| 119.5–120 s | radar and deflector mark 2 | 66 / 0 t (red), 44 kph, 28 %, 25 %, 400 m |
| 126.5 s | one LFT off — the design accepted at 155.5 s | 59 / 6 t, 48 kph, 28 %, 17 %, 400 m |

For the first line: 10,000 kg of body and 15,950 kg of `_df` parts and armour
make 25.95 t; 1,500 hit points over mark 1 armour's 0.51 make 2,957, 12 %.

## A part's box — *read*

A part's box is drawn from the same object code, over a preview of the part
alone (`0x1006f300`). Its rows are the research item's stat template, `TRFA`
([19-descriptions.md](19-descriptions.md)). A row is
`@G@<label>@B,<field>,G,<unit>,<width>,<decimals>@`; a quoted field (`"2"`)
prints as written. The recording prints one decimal on every number, whatever
the template says (*seen*).

| field | value | seen on Mission 02 |
|---|---|---|
| `weight` | property 124 × 0.001 | L-2f 10.0 t, 4L1 5.0, LRL36S 2.8, LFT 3.3, LEng1 1.6 |
| `payload` | 136 × 0.001 | 55.0 t |
| `maxspeed` | 144 × 3.6 | 110.0 kmph |
| `wattage` | device 0's value `0x1300` (a gun's energy a shot), else its value `0x63` | LRL36S 0.2 MWt (0.24), LEng1 4.5 |
| `Frate` | device 0's `0x65`: 1000 ÷ max(1, interval) | LRL36S 1.3 1/s, LFT 0.7 |
| `range` | device 0's round controller +108 | 250.0 m, 200.0 m |
| `damage` | device 0's `0x68`, the round's damage | 225.0 HP, 790.0 HP |
| `blast` | device 0's `0x76` | — |
| `Epower` | device 0's `0x79` | LEng1 3.1 MWt (not traced) |
| `capacity` | 113 × 0.001 | — |
| `Adfactor` | device 0's `0x78` | — |
| `sensrange` | `0x50`, the radar's range | 400.0 m |
| `regener` | device 0's `0x77` | shield 60.0 HP/s, repair 60.0 |
| `throughput` | 164 | — |
| `shotnum` | device 0's `0x55`, an integer | — |
| `effic` | 134 × 100 | deflector 85.0 % |
| `Spower` | 132 | shield 3500.0 HP |
| `density` | 163, armour's rating | ARM 1 17.5 kg/m2, ARM 2 22.5 |
| `Peffic`, `Eeffic`, `fastness`, `Pincrease`, `experience` | 0 | — |

The keyword list is `0x1006f070`; the values `0x1006f300`, a 24-way switch
(`0x1006f654`).

## The name — *read*, and *seen*

The unit box's title is `"%s-%s %s"` (`0x100765e0`, called at `0x1004f37b`):

- **the three letters** of [35-hud.md](35-hud.md#name-and-status--read-and-seen)
  (`0x10076270`): size from property `0x201` (T, S, M, L), chassis from
  `0x207` (F, S, W, T, A, U), class from the Type (B, T, W, C, H), `?` for
  anything else;
- **`X`** in place of a number (the constructor passes 0);
- **the class word** by Type (`0x10076490`): 6200 Transport, 6201 Builder,
  6202 Warrior, 6203 Comm. Center, 6204 Human, else 6205 Unknown.

The Type is the turret's role ([16-research.md](16-research.md)), or an animal
for a chassis whose name starts `a`. Without a turret it is 0, so the L-2f
alone reads **"LF?-X Unknown"** and with the 4L1 **"LFW-X Warrior"** (*seen*,
*measured*).

**A built bot is numbered by its clan.** `0x10075d50` names every unit of a
clan `"%s-%d %s"` with one more than the clan's count, and raises the count
for every unit it names — the hero's "Human" and an animal's "Animal" too
(`0x10075eb2`). Mission 02's player clan names its hero first, so its first bot
is **LFW-2** (*derived*, and *seen*).

## The files — *read*, and *measured*

- **Accept** works out the Type (`0x100514e7`). For a robot it writes the
  design to `<game>\units\temp_unit.dat` (`0x100516b3`), with the design writer
  `0x100544b0`.
- **Save** writes `units/<name>.dat` with the same writer (`0x10050c28`).
- **Load** lists `units/*.dat` but for the files named `bld_unit_`,
  `view_unit_` and `temp_unit` (`0x100511b3`).
- **Cancel** discards the design and reopens the root page, `r_` for a robot
  (`0x100513bf`).
- **The factory** writes `%s\units\view_unit_%d.dat` and, when it can start
  one, `bld_unit_%d.dat`, `%d` its logic id (`0x100983e2`, `0x1009842f`; see
  [36-factory.md](36-factory.md)).

All are the `.dat` layout of [07-objects.md](07-objects.md#unitsdat--unit-and-building-assemblies):
`0xF0F1`, the Type, then 112-byte components depth first. Each component is
`objects.rlb`, the part id as the tree spells it, flags 1, the attachment,
the label "`name (code)`", the class and the child count. The classes are
0 chassis, 1 turret, 2 armour, 3 internal part, radar or deflector, 4 gun,
5 clip.

**Children are written slots first, in slot order, then sockets in node
order** (*measured*). All 33 of the install's `bld_unit_*`, `view_unit_*`
and `temp_unit.dat` re-write byte for byte from their trees in that order:

- the chassis's engine 0, battery 1, shield 2, detection shield 3, repair 4
  and armour 5, then the turret on its socket node;
- the turret's radar and deflector, then its guns by socket;
- under each gun its clip at slot 0.

Also *measured*:

- 15 of the 16 `bld_unit`/`view_unit` pairs are identical;
- `temp_unit.dat` is `bld_unit_-2147483625.dat` byte for byte;
- 8 of the 16 hand-made `UNITS/UNITS/PREBLD` designs keep the same order, and
  the other 8 put armour first.

## The price — *read*

A factory prices a design by its parts (`Behavior.dll:0x1002a1a8` calling
`0x10029810` over the design tree):

- **ore** is Σ `BuildOreCost` and **power** Σ `BuildEnergyCost`, over every
  part including the chassis, from the clan's research tree;
- a part not researched or not in the tree fails the build ("Technology to
  create …");
- the ore is then divided by the factory's efficiency when above 0, and the
  time is 5 s for a paid bot ([23-economy.md](23-economy.md#construction--read)).

**The design the recording accepts** — L-2f; engine, battery, shield,
detection shield and repair mark 2; armour mark 3; 4L1 with radar and
deflector mark 2; two LRL36S with rocket packs and one LFT with its clip —
**costs 411 ore and 226.5 power** from `tut2_pl.trf` (*measured*). Whether a
free bot spares it is [36-factory.md](36-factory.md)'s.

## For an engine

1. **Catalogue.**
   - For a page prefix set, walk the clan tree's `TRFB` part ids in order.
     Keep a part whose id starts with any prefix (case-insensitive) and whose
     item has `TRF1` bits 4 and 2 (everything with `FULL_RESEARCH_TREE`).
     Cap at 64 rows.
   - Chassis page: `r_t`, `r_l`, `r_m`, `r_b` up to the factory's size class
     (small 2, medium 3, large 4).
   - Turret page: the chassis node whose stream-10 label starts `e_tur_`, and
     that label as the prefix.
   - Gun page per turret node with a label: `e_gun_` + its last two letters,
     or `<s>c` then `<s>l` when the last is `r`.
   - Slot pages: each controller component's label, `i_arm_` slots on the
     armour page, `i_brn_` slots nowhere.
2. **Fitting.**
   - Choosing a chassis starts a new design and fills every labelled slot but
     brains with `<label>_df`, when offered.
   - A turret goes on the `e_tur_` node and fills its radar and deflector.
   - A gun goes on its socket and fills its clip.
   - A part chosen on a slot page replaces that slot's part.
   - Accept is allowed only with a turret and spare payload above 0.
3. **Numbers.**
   - Assemble the design exactly as a unit loads
     ([28-chassis.md](28-chassis.md)) and read mass, spare, top speed,
     defence, offence and radar range as above.
   - Print `"%-.f / %-.f t"` (red when spare ≤ 0), `"%-.f kph"`, `"%d %"`
     twice over `[TEMP]`, and `"%-.f m"`.
   - A part's box: its `TRFA` rows, values by the field table, one decimal.
4. **Name.** "letters-X word" in the constructor; the built bot takes its
   clan's running count + 1.
5. **File.**
   - On accept, write the design as a `.dat`: children slots first, then
     sockets, ascending; labels "name (code)"; classes 0–5; flags 1.
   - The factory copies it into `view_unit_<logic id>.dat` and
     `bld_unit_<logic id>.dat`.
6. **Price.** Σ build ore and Σ build energy over the tree's parts, all of
   which must be researched; ore ÷ efficiency.

## Not established

- **The grade's source.** That the factory record's `+0x30` holds the
  building's size class is derived from its other readers, not from its writer.
- **`Epower`.** The engine's "Power" figure, device value `0x79` (LEng1 3.1
  MWt), is not traced to a record value.
- **The part box's number format.** The recording prints one decimal on every
  template; the formatter that ignores the template's decimals field was not
  read.
- **The turret and gun fits.** They were read only as far as their `_df`
  lookups (`0x10052a92`, `0x100532ed`) and the turret flag. What they do to
  guns already on a replaced turret, or to clips when a gun is swapped, is not
  read. `openparkan.designs` drops the old turret's subtree and a socket's old
  gun.
- **Rounding.** `"%-.f"` and `fistp` both round half to even under the default
  FPU mode, which Python's formatting matches; a value exactly on a half was
  not met.
- **Other modes.** The constructor has two more: buildings, `fr_`, and
  animals, `a_` (`0x10051413`). Only the factory's robot mode has a caller.
