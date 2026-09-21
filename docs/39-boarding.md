# Boarding and leaving a warbot

How the hero gets into one of the player's own large bots, drives it, and gets
out again — what *The Constructor* (`CAMPAIGN.00/Mission.02`) does with the
LFW-2 Warrior it builds. Capturing a neutral bot, which can end in the same
boarding, is [27-ownership.md](27-ownership.md#a-neutral-unit-is-taken-by-the-hero--read-and-measured)'s.

**Every claim is tagged**, as in [23-economy.md](23-economy.md): *measured* is
re-derived by `openparkan verify` (`check_boarding`), *read* comes from the
disassembly at the address given (all `iron3d.dll` unless named), *derived*
follows from those, *seen* is the recording
`training mission 2  The constructor [VUOXXTEuxeQ].mkv`, and *guess* fits the
evidence without being established.

## The game view keeps a stack of modes — *read*

The game object's `+0x20` is the interface's `CState`: a deque of 16-byte
records, its front the mode the player is in.

| offset | field |
|---|---|
| `+0x0` | the mode |
| `+0x4` | a unit record: set for modes 1–3 |
| `+0x8` | a building: set for modes 4–6 |
| `+0xc` | a byte the takt (`0x10062950`) sets when that building's owner word reads `0xfffe` |

- **Entering a mode** (`0x10062bc0`, mode, building, unit) looks up the handler
  for *front mode × 8 + new mode* in the table at `0x10104b18` and calls it.
  The handler may refuse by setting the `CState`'s byte `+0x30`; otherwise the
  record is pushed. Mode 3 is refused for a unit that fails `IsHQ`
  (`0x10062c2e`).
- **Rolling back** (`0x10062ff0`) does nothing with one record on the stack.
  Otherwise it calls the handler for *front × 8 + the record below*, pops the
  front unless the handler refused, and repeats while the new front's `+0xc`
  is set.
- **The modes, as far as read:**

  | mode | what | pushed by |
  |---:|---|---|
  | 0 | on foot, the hero | the bottom record |
  | 1 | driving a bot | Enter (below) |
  | 2 | telepresence: a unit driven from a command view | a unit page's buttons ([40-command-mode.md](40-command-mode.md#telepresence-mode-2--read)) |
  | 3 | an HQ's command view ([40-command-mode.md](40-command-mode.md#an-hqs-command-mode-mode-3--read-and-seen)) | Enter from a bot that passes `IsHQ` (`0x10072104`), Enter in telepresence aboard one, or a unit page's D button |
  | 4 | a bunker's command view ([40-command-mode.md](40-command-mode.md)) | the bunker's pod, or its page in mode 3 |
  | 5, 6 | a building's screen | the building's pod ([27-ownership.md](27-ownership.md#capture--read)) |
  | 7 | the game menu | `CMD_GAME_MENU` (748, `0x10072359`): it pushes 7, or rolls back when the menu is up (`0x100723a5`) |
- **These are not the names at `0x1005a5cc`.** That switch (`FREE_MODE`,
  `SELECT_ATTACK_TARGET_MODE`, … `SELECT_PLACE_MODE_FM`) reads the global
  `0x1010c388`, which the commander's order menu writes (`0x10079998`,
  `0x1007a1a2`, `0x1007a3ce`) and selecting a unit clears (`0x1007d187`). It is
  a separate cursor mode; the stack's mode 1 is a driven bot, and 7 is outside
  its range.
- **The level's view state word** (`+0x710`, set through `0x100a4f90`) is
  separate again. Boarding and leaving both set it to 1, the cockpit. State 3
  is the outer camera, which both turn off first (`0x10038ad0`); 5 is a
  briefing ([21-briefing.md](21-briefing.md)).

## Boarding — *read*

**The key** is Enter: `CMD_ENTER_STATE`, 730, *"Enter warbot/HQ"*, bound to
`SCAN_W_ENTER` in `addition.man` and `ui_other.man` (*measured*). Its case
(`0x10071f08`) boards when every test passes, in this order:

1. the view state word is not 4;
2. the stack's front is not mode 1 (from a bot, Enter asks for mode 3 instead,
   which only an HQ passes), and not mode 2 on an HQ;
3. the player's hero record (`0x10072be0` with Type mask `0x1020000`) exists, the
   view state word is 1 or 3, and the hero's `+0xa2` is set: the player is
   driving the hero;
4. the hero's current target (target list `+0x38`, `+4`) is a unit whose Type
   has no bit outside `0x103e000`;
5. it is **less than 20 m from the hero across the ground** (`0x10071fe7`,
   the float at `0x100e4bc4`). Height is not compared;
6. its record's `+0x30` is **4** and `0x10076d30` does not refuse it
   (`0x10071ff8`). `+0x30` is the size class, object property `0x201`: T 1,
   S 2, M 3, **L 4** ([35-hud.md](35-hud.md#name-and-status--read-and-seen)),
   the chassis's `b` letter. The test refuses a missing record, a removed
   object, a unit with no class-1 turret and one whose turret has no life left
   ([27-ownership.md](27-ownership.md)) — the life of the turret's **body, its
   node 1** ([below](#the-turrets-life-is-its-bodys-node-1--read-and-measured));
7. a neutral target is captured first (27-ownership). Otherwise the target's
   clan (`+0x24`) must be the player's (`+0xad0`, `0x100720d3`).

Then `0x10062bc0(1, 0, bot)` (`0x100720e8`). So **only a large bot can be
boarded, it need not land or stop, and nothing in the order or behaviour is
asked** (*derived*). *Measured:* 86 of the turreted robot assemblies in
`UNITS/UNITS` stand on a size-4 chassis; 10 of them fly.

### The turret's life is its body's, node 1 — *read*, and *measured*

`0x10076d30` asks the record's `+0x60`, the control system's interface `0x202`
(bound at `0x1007e4e6`; `+0x64` is `IDeviceManager`, `0x204`, bound at
`0x1007e510`), in two calls:

1. **Slot 9 with index 0 and class 1** (`0x1002ea80`, handed to
   `IDeviceManager` slot 17) gives the first class-1 device, the turret, or −1.
2. **Slot 3 with that device and property `0x52`** (`Control.dll:0x1002e580`,
   case `0x1002e68c`) is `IDeviceManager` slot 6 (`0x1002bb40`) with value
   `0x400`, whose case (`0x1002bc3c`) asks the device's life system (`+0x44`)
   slot 3 with id 1 for the node the device keeps at `+4`. `ILifeSystem` slot 3
   (`0x1000dc40`) answers id 1 with that node record's `+0x14`: **life over
   maximum** ([26-damage.md](26-damage.md#what-a-damaged-node-a-destroyed-part-and-a-dead-unit-draw--read-and-measured)).
   At 0 or below (the 0.0 at `0x100e50a8`) the test refuses.

**Which node that is** (*read*). The device's `+4` is its `.ctl` record's `+4`,
which the shared parser copies when the factory builds a device (`0x10021d72`,
flag 1 from `0x1002d79f`). The loader then rebases it for every part but the
chassis (`Control.dll:0x1000906a`–`0x10009095`): **node 0 becomes the node the
part hangs on**, the load message's `+0xc4`, and node *k* becomes the part's
first node less one plus *k*, the first node being what the mesh's slot 14
answers for the part (`0x10008b57`). The control points are rebased the same
way (`0x10008be8`), and the `.ndp` rows match by a fitted part skipping its
first (`0x10008c6a`,
[28-chassis.md](28-chassis.md#the-order-parts-load-in-and-what-a-slot-keeps--read-and-measured)).

**What the files give** (*measured*): the install carries 64 class-1
components.

- **All 58 in `turrets.rlb` name node 1**; the chassis `r_l_06`'s own names its
  node 1, `Turr`; the five animals' (`a_a_l1`…`a_a_l5`) name node 0, their root,
  since an animal is one part.
- **All 60 turret parts** (`e_tur_*`, `e_tow_*`, `e_bnt_*`) have a node 0 of 1
  hit point, `Base_TL` or `Base_TM` — the socket, whose place in the unit the
  host's node takes — and a node 1 that carries the turret: `BTmn` with 3,000
  on the L-2f's `e_tur_bb_01`, `LTmn` with 270 on most small turrets, `TMmn`
  with 720 on the medium ones, up to 80,000 on `e_tur_bb_12`. The one exception
  is the hero's `e_tur_ht_02`, whose node 1 is the 1-point pivot `Turn`.

So **a bot can be boarded, and taken over from command mode, until its turret's
body is shot to nothing**, whatever its socket or its other turret nodes.

**What the mode 0 → 1 handler does** (`0x100637c0`), in order:

1. turns the outer camera off (`0x10038ad0`);
2. **detaches the hero's object from its parent**: `IGameObject` slot 3 gives
   the parent (`AniMesh.dll:0x10017570`), whose slot 5 is
   `CGameObject::DetachChild` (`AniMesh.dll:0x10001f80` → `0x10017680`)
   (`0x100637ed`–`0x100637fa`);
3. in a network game (the game's `+0xe4`), sends game message `0x3f1` with the
   hero's id;
4. switches the hero's camera view off: its first class-4 component's view,
   flag bit 0 cleared (`0x10075160`);
5. hands the hero back (`0x10074ff0` with 0), selects the bot alone
   (`0x1007d0a0`), hands the hero back again, then **takes the bot**
   (`0x10074ff0` with 1) and switches its camera view on (`0x10075180`);
6. makes the bot the driven unit (`+0xaec`, `0x100a5660`) and clears `+0xaf0`;
7. sets the view state word to 1, and the `CState`'s `+0x28` to the bot;
8. in game mode 4 (a mission of campaign 0, the training campaign — *read*,
   [21-briefing.md](21-briefing.md#the-launch-mode-and-what-mode-4-is--read-and-measured)),
   if the bot flies
   (`0x10075f70`, `ChassisType` 1), plays message 100 (`0x100638a9`);
9. ends in `World3D.dll`'s `stdClearKeyboard` (`0x100638b2`), which removes
   every pending key and mouse message (`0x100`–`0x108`, `0x200`–`0x209`) and
   clears the key state (`World3D.dll:0x10011830`).

**Selecting the bot** (`0x1007d0a0`) plays `VOICE_SELECTED` unless the bot was
already selected (record `+0x80`) or the game is in parameter mode 3. The voice
takes the speaker's suffix (`0x10061b45`): `_S` for size 1–2, `_B` for size 4–5
or a building, none for 3 — so `VOICE_SELECTED_B`, `vr_sel_b.wav`, for a large
bot (*measured* binding in `ui/game_resources.cfg`).

**Taking the bot** follows its auto-driver level (`+0x9c`,
[31-packages.md](31-packages.md#the-escape--read)). A unit record bound to its
object starts at **0** in single play (`0x10074dcf`: 2 only in parameter mode
3), so the player gets its movement, turret and guns, the wizard goes to the
player's mode, and `+0xa2` is set.

**What becomes of the hero** (*derived*):
- detached from the world's object tree, it is no longer reached by the frame's
  broadcasts: not drawn, not ticked, and not met by the queries that walk the
  tree (the hit test's segment query, the ground search);
- it keeps its position and matrix until it is put out;
- its behaviour takt does not run, which would leave it reporting no route
  while aboard. **A recording contradicts that** (*seen*): on Mission 02 route
  4's message plays while the hero flies the warbot over the lake, 500 m from
  where it boarded ([34-progression.md](34-progression.md#seen-in-a-recording)).
  So the route is reported where the bot goes, by a path not read.
- **it leaves the collision manager** (*read*). `CGameObject::DetachChild`
  (`AniMesh.dll:0x10017680`, and the copy the landscape's `IGameObject` slot 5
  runs, `Terrain.dll:0x1008a8a0`; both name themselves in a panic string) tells
  the parent 3 and the child 7 (`0x100176f3`, `0x1001770e`; `0x1008a913`,
  `0x1008a92e`). The agent passes the 7 on as
  message 21 to its mesh, its control system and its collision object
  (`IGameObject` slot 20, `0x10001ba0`), and sub-code 7 takes the collision
  object out of its manager and puts it in none (`Control.dll:0x1001f579` →
  `0x1001fea0`). Putting the hero out sends 6, which also joins the first
  ancestor that answers `0x203` (`0x1001f585`–`0x1001f58e`): the world's
  manager, or a building's
  ([24-motion.md](24-motion.md#walking-into-a-building--read-and-measured)).
  So nothing collides with a boarded hero.
- **the areal map is not told** (*read*). The system areal map's attach and
  detach hook (slot 3, `ArealMap.dll:0x1001f660`) returns at once for anything
  whose kind is not 10, a tree or a stone (`0x1001f68e`). What it lists of a
  unit are the tactical areals, the routes, which only the unit's behaviour
  report changes
  ([34-progression.md](34-progression.md#who-stands-in-a-route--read)). That
  report reads the object's position from its kind-2 world matrix, elements 3,
  7 and 11 (`Behavior.dll:0x10014f90`, `IGameObject` slot 8), and a detached
  object keeps that matrix as it was: slot 8 composes it afresh only under a
  parent (`AniMesh.dll:0x10002470`). So the hero's own report would keep it on
  the routes about the place it boarded, and how the recording's route 4 comes
  to hold it is still not read.

## Driving — *read*, and *measured*

**The input table is the chassis's.** A robot chassis record in `objects.rlb`
names its `.tbl` as slot 6's member (the slot's library field is left blank).
*Measured*, over all 24 chassis records:

| table | chassis |
|---|---|
| `hero.tbl` | `r_h_02`, the hero, alone |
| `m2.tbl` | the 9 flying chassis: `r_t_02`, `r_l_02`, `r_l_05`, `r_l_06`, `r_l_07`, `r_m_02`, `r_b_02`, `r_b_07`, `r_b_08` |
| `m1.tbl` | the other 14 |

**`m2.tbl` is `m1.tbl` with four more rows** (*measured*): R sends `MCMD_UP` 1,
F sends `MCMD_DOWN` −1, and each release sends 0. `table_2.man` is
`table_1.man` plus `CMD_OBJ_MOVE_UP` on R and `CMD_OBJ_MOVE_DOWN` on F, and
`ui_bots.man` binds the same. What the rows do:

| input | row | effect |
|---|---|---|
| W / S | `MCMD_WALK_F` 1 / `MCMD_WALK_B` −1 | the command's y ([24-motion.md](24-motion.md#from-input-to-motion--read-and-measured)) |
| A / D | `MCMD_LEFT` / `RIGHT` | the strafe |
| R / F | `MCMD_UP` 1 / `MCMD_DOWN` −1 | **the command's z**: the handler (`World3D.dll:0x1001059b`) writes z into the command triple it read (property `0x20`, `0x1000fbb4`) and hands it to `SetTangAccel` |
| , / . | `MCMD_ROTATE_Z` ±0.7 | the hull's spin, 0.7 × its live yaw rate; replaced every step while the turret lock is on |
| mouse X | turret `ANGLE_X` 0.15, wrapping | the turret's yaw; the hull follows it ([30-turrets.md](30-turrets.md#the-hull-follows-the-turret--read-and-measured)) |
| keypad 5 | turret `MCMD_LOCK` 0.5, `TURRET_LOCK` | switches the turret lock, on when the bot is taken: whether the hull follows the turret |
| mouse Y | turret `ANGLE_Y` 0.25, clamped | the pitch, which never moves a flyer up or down ([24-motion.md](24-motion.md#a-flyers-height--read-and-measured)) |
| keypad `* + − /` | `MCMD_FORWARD` | the cruise |
| 0–8, left button, H, G, N | as on the hero | guns, camouflage, repair, infrared |

So a flyer climbs and sinks toward ±its top vertical speed while R or F is held,
and holds its height when neither is: a flyer's states have no contact points
and never fall ([24-motion.md](24-motion.md#holding-the-body-on-the-ground--read-and-measured))
(*derived*). *Measured* on the L-2f `r_b_02` controller: mode 0, authored top
speed (4, 30.56, 15) m/s — 110 km/h forward — acceleration (4, 22, 10) (live
twice that), turn rate 4.2 rad/s about z.

Mission 02's own help text for the moment disagrees with the tables: `T02_H06`
tells the player to use PageUp and PageDn for altitude (*measured*). Those keys
are bound to `CMD_JAMES_HQ_MOVE_UP` / `DOWN` in `ui_hq.man` and `addition.man`,
the HQ camera's.

**The view and the HUD** (*read*, elsewhere):
- the eye is the bot's turret camera: `CameraCenter` and `TargetDirect`
  ([30-turrets.md](30-turrets.md#aiming-and-the-camera--read-and-measured)),
  whose up is −z on a hung turret such as the L-2f's `e_tur_bb_01`;
- the view draws the bot's fifth slots, its cockpit
  ([07-objects.md](07-objects.md#the-fifth-slot-is-what-the-units-own-view-draws));
- everything the cockpit HUD reads from the driven unit (`+0xaec`) is now the
  bot's: the weapons list, the own panel with its name and order, the target
  panel from its target list, the radar
  ([35-hud.md](35-hud.md)).
- **The wingman list at the top left** is one line a friendly unit on the
  driven unit's target list (`0x100432f0`, `0x1006ddb0`)
  ([31-packages.md](31-packages.md#the-wingman-menu-from-first-person--read-and-measured)).
  That is the strip reading "LFW-2 Warrior" in the recording from 232.3 s,
  when the bot came onto the hero's list; aboard, the hero is off every list
  and the strip goes (*seen*, *derived*).

## Leaving — *read*

**The key** is Esc: `CMD_ROLLBACK_STATE`, 735, *"Leave warbot/HQ/..."*, bound
to `SCAN_ESC` in `addition.man` and `ui_other.man` (*measured*). Its case
(`0x10072238`) rolls the stack back, and from mode 1 that is the handler
`0x100638c0`.

How Esc gets there: a window message the interface's handlers do not take goes
to the key bindings (`0x100a0fe7` → `0x10071c10`), which turn a key-down into
its command (`0x10071ca1`). The game loop translates every message
(`TranslateMessage`, `0x1005e748`), so an Esc also queues a character for the
game view's character handler, whose Esc on foot opens the game menu
([34-progression.md](34-progression.md#after-the-outcome--read-and-measured)).
Leaving ends in `stdClearKeyboard`, which removes that pending character
(*derived*); none is seen in the recording.

**Where the hero goes** (`0x10063350`, called from `0x10063500`):

- **A bot that is broken or gone** — `0x10076d30` refuses it, or its owner word
  reads `0xfffe` — puts the hero at **(bot x − 1, bot y − 1)**, untested
  (`0x100634ad`).
- **Otherwise eight places are tried** at r = the bot's sphere radius + the
  hero's (the records' `+0x94`, the radius of the sphere interface `0x20`
  slot 3 answers to kind 1 and an all-zero request, `0x1007e5f2`–`0x1007e60e`:
  the object's node sphere,
  [24-motion.md](24-motion.md#finding-the-ground--read)), at angles i × π/4
  from +x, i = 0…7 (`0x100633ce`): (bot x + r cos, bot y + r sin). On the L-2f
  and the hero that is 11.84 + 1.59 = 13.43 (*measured*). The first that
  passes wins:
  - `IWorld` slot 8's vertical query for **landscape** faces (object class 1,
    mask 2: `Terrain.dll:0x10025ba0`, table `0x1009a5f0`) must find one there;
  - that face must not be a liquid surface (world face flag `0x200`,
    `0x1006346a`);
  - **for a flying bot, the bot must be less than 10 m above that point**
    (`0x1006347c`, the float at `0x100e5d6c`). A walker's height is not tested.
- **The height** is the highest landscape or building surface at that x, y
  (`0x100a14d0` with mask `0xa`, or 0 where there is none) **plus 8**
  (`0x100634e4`). The hero is dropped from there and falls under gravity
  ([24-motion.md](24-motion.md#holding-the-body-on-the-ground--read-and-measured))
  (*derived*).
- **No place passes**: the System line 6211 *"Risk area! Landing impossible."*,
  `VOICE_RISK_AREA` (`vc_002.wav`), indicator 4 red for 3 s
  ([35-hud.md](35-hud.md#the-indicators--read-and-seen)), and the handler
  refuses: the player stays aboard (`0x10063542`–`0x10063694`).

**The matrix the hero is given** (`0x10063699`–`0x1006379f`, *read*). With F
the unit vector from the place to the bot across the ground and R = (−F) ×
**z** = (−F.y, F.x, 0), it writes the rows (R, place x), (−F, place y), (**z**,
place z), (0, 0, 0, 1) over a copy of the identity at `0x1010b620`: the place
goes into elements 3, 7 and 11. **The matrix goes on as it is** (*read*):

- `IControl` slot 12 (`Control.dll:0x10004690`) copies the sixteen words
  unchanged into the hull's matrix `+0x264` (kind 3), `+0x2a4` (kind 1) or all
  of `+0x2e4`, `+0x2a4` and `+0x264` (kind 2). Where the control system moves
  all three by one offset (`0x1000cb03`–`0x1000cb5f`) it adds to their elements
  3, 7 and 11: the layout the leaving routine writes.
- `IGameObject` slot 7 with kind 2 (`AniMesh.dll:0x10001fb0`) compares it word
  by word with the object's world matrix and, where it differs, copies it there
  unchanged (`+0x778`, `0x1000212b`), working the local matrix out from the
  parent's. Slot 8 with kind 2 hands that world matrix back (`0x1000248e`),
  composed again from the parent's and the local one.
- **A unit's forward axis is that matrix's y column.** The own panel's camera
  takes slot 8's kind-2 matrix and reads its elements 1, 5 and 9 as the way the
  unit faces (`iron3d.dll:0x1004186f`–`0x10041884`,
  [35-hud.md](35-hud.md#the-unit-in-the-middle--read-and-seen)), as the motion
  takes y for forward
  ([24-motion.md](24-motion.md#the-hull-leans-and-rights-itself--read-and-measured)).

So **the hero faces (F.x, −F.y)**, F mirrored about x (*derived* from the
reads). From the places due +x and −x of the bot (0 and 4) it faces the bot;
from those due +y and −y (2 and 6) it faces straight away from it; from the
four between it stands side-on. The rotation is a proper one (its determinant
is F.x² + F.y² = 1), so the hero is turned, not mirrored. In the recording the
hero comes out looking at the bot (*seen*, 331.8 s), from the first place, due
+x, which passes there ([below](#against-the-recording--seen)).

**What the mode 1 → 0 handler does** (`0x100638c0`), in order:

1. finds the place, or refuses (above);
2. turns the outer camera off;
3. **re-attaches the hero's object** to the object new objects are added under
   (the level's `+0xae0`, the parent `AddNewObjectToGame` is given), its slot 4
   `CGameObject::AttachChild` (`AniMesh.dll:0x10017580`) (`0x1006391f`);
4. in a network game, sends game message `0x3f2`;
5. gives the hero's `IControl` the matrix through slot 12 with kinds 1, 2 and 3
   (`Control.dll:0x10004690`: kind 3 the matrix at `+0x264`, kind 1 `+0x2a4`,
   kind 2 all three), then slot 8 with 5, and its object slot 7 with kind 2;
6. deselects the bot (`+0x80` 0, `+0x84` −1) and clears the selection
   (`0x1007d270`);
7. makes the hero the driven unit; clears the `CState`'s `+0x28` and `+0x31`,
   stamping the time;
8. **takes the hero** (`0x10074ff0` with 1), **lets go of the bot** (with 0),
   switches the hero's camera view on;
9. sets the view state word to 1, and ends in `stdClearKeyboard`
   (`0x10063a17`).

**The bot afterwards.** Let go, it goes back to the AI with every override on
([31-packages.md](31-packages.md#the-escape--read)): it carries on with its
order, and one left standing on a building's grounds is given an escape. The
recording's bot hovers where it was left, "LFW-2 Warrior (no order)" (*seen*).

## When the driven bot is lost — *read*

When a unit record goes (`0x100751a0`) and it is the driven unit:

- the player's hero fails the mission
  ([34-progression.md](34-progression.md#after-the-outcome--read-and-measured));
- **any other unit rolls the stack back** when its front is mode 1, 2, 5 or 7
  (`0x100755a9`, table `0x1007563c`), and not in 3, 4 or 6. From mode 1 that
  is leaving, and a gone bot puts the hero at (bot x − 1, bot y − 1), 8 m above
  the highest surface there (*derived*).

## The other writers of `+0xa2` — *read*

The flag the Enter test reads is written, besides the drive switch:

| where | value | when |
|---|---:|---|
| `0x1007e2ad` | 0 | the unit record's constructor |
| `0x10074dbf` | 0 | binding a unit record to its object |
| `0x100a2a73` | 0 | a briefing starts, after the manual controller's slot 9 |
| `0x1005e7e8` | 1 | the briefing ends ([21-briefing.md](21-briefing.md)) |
| `0x1006788a` | 0 | the help screen opens (`0x10067820`), with slot 9 and the game paused |
| `0x1006792a` | 1 | it closes (`0x100678c0`), with slot 10 and the pause lifted |

`CMD_HELP` (749, F1) toggles the help screen (`0x100722dc`). An earlier line in
[27-ownership.md](27-ownership.md) read the last two as the hero's body shown
and hidden.

## Against the recording — *seen*

| time (s) | what |
|---|---|
| 232.3 | the hero's target becomes the LFW-2 Warrior and the wingman line shows it at the top left |
| 234–249 | the hero walks to the bot, which hovers off the Large Factory's end where its escape left it; the left panel reads "LFW-2 Warrior (no order)" (below) |
| 249.3 → 249.4 | a hard cut into the bot's cockpit, blue and yellow frame; weapons LRL36S, LRL36S, LFT with 36, 36 and 30 rounds, which the font draws as "LAL365"; the right panel shows the bot; the box turns to *"Manual control of warbots…"* from the Information assistant (message 100); the wingman line is gone |
| 255–322 | flying to the island, firing, setting down by the Outpost |
| 311.6 | a box about large warbots, which expires at 331.6 |
| 326 → 331.5 | setting down by the Outpost, sampled twice a second: the altitude figure falls 33, 29, 26, 23, 19, 16, 13, then 11 from 329.5 s, and holds 11 to 331.5 s while the speed figure falls from 24 to 6; the target panel reads the Small Outpost 31–33 m off |
| 331.7 → 331.8 | a hard cut to the hero standing beside the bot, looking at it; the hero's weapons return, and the wingman line with the bot |
| 332 → 335 | the hero's altitude figure 6, then 3 from 332.5 s as it stands |

Neither cut fades or moves the camera between the two views.

**The weapons' names** are the research tree's short codes of the bot's gun parts,
`e_gun_bl_15` and `e_gun_bc_06`
([35-hud.md](35-hud.md#the-weapons-list--read-and-measured)). The game's font draws
R much as it draws A: the hero's own list reads "PLASMA AIFLE LS" and "BATTLE
LASEA EA" for strings 3072 and 3073 in the same frames.

**How the bot came to the hero** (*seen*, against what is *read*). Nobody sent it.
The factory gives a new bot the escape, no target, replacing
(`Behavior.dll:0x1002aa6e`, [36-factory.md](36-factory.md#production--read)): a
random point within 150 m on usable ground. The panel reads "[escaping]" at 232.5 s
and "[no order]" by 234.5 s, once it has got there
([36-factory.md](36-factory.md#mission-02--measured-derived-and-seen)). Sampled
every second from 234 s, the Large Factory grows in the view while the bot hovers
off its end, its shadow on the ground under it: the hero walks to the bot, which
keeps its place, and at 248 s it hangs overhead. Its status reads "(no order)"
throughout, where a Follow me would have read "[following]".

**The altitude it was left at** (*derived*). The figure is the record's
`+0xc`, rounded, over the water at z 150, and the height test above reads the
same field. The descent stops at 11 from about 3.3 a half second, as the
ground contact's lift stops a sinking flyer, and the target panel puts the
Outpost 31–33 m off: the bot rests on the island's flat ground at 151.67, which
runs from the Outpost's walls to its shores. Resting on the agent's sphere's
radius about the node sphere's centre
([24-motion.md](24-motion.md#finding-the-ground--read)), the L-2f's origin
stands 9.67 over it: 161.34, which reads **11**, and every place 13.43 m out on
the same ground lies 9.67 below, so it passes. The hero comes out standing
with its origin 1.40 over the ground, 153.07, which reads **3** (*measured* on
openparkan's engine). On the agent's sphere alone the bot would read 12 there
and every place about it would be refused; on the chassis's own sphere it
would read 9.

## For an engine

1. **Keep a mode stack** per player: 0 on foot, 1 in a bot. Enter pushes,
   Esc pops; a pop from 1 to 0 may be refused.
2. **Board on Enter** when on foot and driving the hero, and the hero's current
   target is a unit of the player's clan, of size class 4 (a `b` chassis), with
   a turret whose body — the node its class-1 component names, node 1 on every
   turret part — has life left, less than 20 m away in x and y. No landing,
   speed or order test.
3. **On boarding:** take the hero out of the world (not drawn, simulated,
   struck, collided with or listed; position frozen); select the bot and play
   `VOICE_SELECTED_B`; give the player the bot at auto-driver level 0 (the bot's
   AI off); make it the driven unit for the eye, the cockpit (fifth slots) and
   the whole HUD; drop every held key; in the training campaign, if the bot
   flies, play the mission's message 100. Cut, no transition.
4. **Drive with the chassis's table** (`objects.rlb` slot 6): `m2.tbl` for a
   flyer, `m1.tbl` for the rest. R/F set the command's z to ±1 and release to 0;
   the turret leads the hull.
5. **Leave on Esc:** try eight places around the bot at r = both sphere radii
   summed, starting at +x and turning by π/4; a place needs landscape under it
   that is not water, and a flyer less than 10 m above it. Put the hero at the
   first, 8 m above the highest landscape or building surface, heading (F.x,
   −F.y) for F the unit vector from the place to the bot: towards the bot only
   from the places due ±x of it. With none, show 6211,
   play `VOICE_RISK_AREA`, light indicator 4 red for 3 s and stay aboard.
   Then put the hero back in the world, hand it to the player, hand the bot
   back to its AI (with an escape if it stands on a building), deselect it,
   drop held keys, cut back.
6. **If the driven bot dies,** leave at once, the hero at (bot x − 1, bot y − 1),
   8 m above the surface, untested.

## Not established

- ~~Mode 2 of the stack~~ — telepresence
  ([40-command-mode.md](40-command-mode.md#telepresence-mode-2--read)). When
  their unit is lost, mode 2 rolls back like mode 1 (the removal's table
  `0x1007563c`); what modes 3, 4 and 6 then do is not read — nothing in that
  path pops them.
- Whether the interface's own key-down handlers ever take Esc before the
  bindings do. This recording is consistent with leaving by Esc but does not
  show the key; Mission 04's shows Esc closing the satellite map and the
  commander's page before it leaves an HQ's command view
  ([40-command-mode.md](40-command-mode.md#leaving)).
- ~~The facing: the heading above follows the column convention the placement
  matrices use elsewhere. That the leaving matrix is read the same way is
  *derived*, not traced into `IControl` slot 8's mode 5 or `IGameObject` slot 7.~~
  **Read**: `IControl` slot 12 and `IGameObject` slot 7 keep the matrix word for
  word, and the kind-2 world matrix it becomes is the one whose y column the own
  panel takes for a unit's forward axis, so the hero faces (F.x, −F.y): towards
  the bot from the places due ±x, away from it from those due ±y
  ([Leaving](#leaving--read)). Slot 8's mode 5 hands the object at control
  `+0x34` its slot 3 (`Control.dll:0x1000477d`) and takes no matrix.
- ~~Whether a detached hero stays in the collision manager's or the areal map's
  lists.~~ **Read** for both: its collision object leaves its manager on the
  detach and joins the world's on the re-attach, and the areal map's attach hook
  passes over everything but trees and stones
  ([What becomes of the hero](#boarding--read)). Still open: how the hero's route
  follows the bot, when its own report would hold the world matrix it boarded
  with.
- ~~Why the recording's bot came to the hero between 236 and 249 s: an order the
  player gave, or its own behaviour after production.~~ **Neither**: the hero
  walks to it (*seen*). The bot hovers off the factory's end, where the escape
  the factory gives every new bot left it at 234.5 s
  ([Against the recording](#against-the-recording--seen)).
- The wingman line's own layout (`0x1009d970`).
- What game messages `0x3f1` and `0x3f2` do on other machines. **What they
  carry is read**: the hero's object id (its record's `+0x28`, `IGameObject`
  slot 9 as the record is bound, `0x1007e51c`), posted to −1 on `World3D.dll`'s
  queue (`GetQueue`, slot 28) — `0x3f1` as the hero is detached (`0x10063817`),
  `0x3f2` as it is put back (`0x10063949`; and `0x10060474`, where a case of the
  game's message callback attaches a player's parentless object under the
  level's `+0xae0`). `World3D.dll`
  posts `0x3f2` itself when `SetStateForGameObjects` finds a player's hero (Type
  `0x1020000`) with no parent and attaches it to the queue's root
  (`0x10005de5`–`0x10005e2a`), so the pair says which hero left the world and
  came back. The receiving side is not found: a sweep of `iron3d.dll`,
  `World3D.dll`, `Net.dll` and `Behavior.dll` for either code as an immediate
  finds those four sends and `Net.dll`'s dialog control ids (`0x10001ed4`), and
  no compare. The game's own message callback takes codes 0 to 14 alone
  (`iron3d.dll:0x1005fabb`). A dispatch through a table would escape the sweep.
