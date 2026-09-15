# The factory screen and production

What the player gets on a factory's control pod: the factory screen, and
through it a design built into a bot standing outside. This page reads the
screen's pieces, its controls, and the order they give the factory, and follows
that order through `Behavior.dll` until the bot leaves the building. It is
measured against the install's art and a recording of *The Constructor*
(`CAMPAIGN.00/Mission.02`).

The pod and the capture are [27-ownership.md](27-ownership.md)'s. The warbot
designer the screen opens is [37-designer.md](37-designer.md)'s, and what a
design is and costs is [38-designs.md](38-designs.md)'s. A build's ore, power
and time are [23-economy.md](23-economy.md#construction--read)'s, and the escape
a new bot runs is [31-packages.md](31-packages.md#the-escape--read)'s.

**Every claim is tagged**, as in [15-behaviour.md](15-behaviour.md):
- *measured* is re-derived by `openparkan verify`;
- *read* comes from the disassembly at the address given;
- *derived* follows from the two;
- *seen* is taken from the recording;
- *guess* fits and is not established.

Coordinates are the HUD's 640 × 480 layout
([35-hud.md](35-hud.md#everything-is-drawn-on-a-640--480-screen--read)).
The pen primitives, the compound-control skin (`ui/compaund.cfg`) and its
kinds are read there too.

## How the screen opens — *read*

**The pod opens the player's own factory.** The pod callback's same-clan
branch (`iron3d.dll:0x10062630`, [27-ownership.md](27-ownership.md#capture--read))
does four things for a building of Type `0x80000010`:
1. **It selects the building** (`0x1007db80`, on the level's `+0x71c` list).
   This plays `VOICE_SELECTED` if the building was not already selected, and
   hands the clan's factory record to the factory panel (`0x10097410`).
2. **It closes the wingman menu** if it is up: command 740 (`0x100626dd`).
3. **It pushes view mode 5 with the building** onto the interface's mode stack
   (`0x10062bc0`, from `0x1006270f`).
4. **It turns the commander panel to page 5** (`0x10084d80` with 5 and 1).
   Page 5 lists the clan's buildings of Type `0x80000010` and sets the page
   word `+0x5f0` to 5 (`0x10084de3`).

The same branch gives a research centre (`0x80000400`) the same mode 5 and
**page 4**, the research panel (`0x10062756`,
[41-commander.md](41-commander.md#the-research-page-4--read-and-seen)). A
generator, mine, storage or Outpost is only selected (`0x10062732`), a bunker
pushes mode 4 and a tower mode 6 while its turret lives (`0x100627b6`).

**The factory record** (`0x100875e0`) is made when a factory joins the player's
clan list. It is 0x1c bytes:

| field | what it holds |
|---|---|
| `+0x04` | the selected project, or −1 (the start) |
| `+0x08` | the file in production (empty at the start) |
| `+0x14` | the building's logic id |
| `+0x18` | *idle*: the factory's current order is not 12 |
| `+0x19` | *batch* |

The panel reads the factory's order through the building's `IAgent` slot 6
(`0x10086e80`). `Behavior.dll:0x10004bb0` answers that slot:
- **With no order**, it writes order 0 and returns 100.
- **Otherwise** it copies the current order and returns the running task's
  progress × 100, held to 0–100. The progress is `MBehaviour+0xa38`, which the
  task stack's takt stores (`0x1000522d`).

**Entering mode 5** (the transition handler `0x10064430`, table `0x10104b18`
entry 0 → 5):
- It finds the player's hero record.
- It hands the hero back from the player (`0x10074ff0` with 0, which clears the
  record's `+0xa2`; see [39-boarding.md](39-boarding.md)).
- It stores the building as the view's building (level `+0xaf0`, `0x100a5680`)
  and the hero as the view's unit (`+0xaec`, `0x100a5660`).
- It sets the level's state word back to 1 (`0x100a4f90`).

The world is not paused: nothing in the transition or the draw pauses it
(*derived*).

*Seen*, on Mission 02: the screen is up at 106.4 s. It appears on the same frame
as *"from: System / Building is captured"* (string 5039), and the cockpit's HUD
goes. How the capture and the opening fall on one frame is
[27-ownership.md](27-ownership.md)'s.

## What is drawn — *read*, and *seen*

**The screens' draw in mode 5** (`0x1008d444`, in `0x1008d200`) runs in this
order:
1. If the warbot designer is up (the interface's `+8` object, `+0x2c` set), it
   draws only the designer (`0x10055dc0`).
2. Otherwise, if the view's unit has its `+0xa2` set (a driven bot, not the
   hero), it draws no panel.
3. Otherwise it draws the commander panel with its icon column left out
   (`0x100836f0` with 0 and 1, pushed at `0x1008d518`).
4. Then, as in the cockpit, it draws:
   - the satellite map;
   - the message box, which in mode 5 moves to (374, 352), 266 wide
     ([35-hud.md](35-hud.md#the-message-box--read-and-measured));
   - the game's `+0x30` overlay;
   - the objectives screen.

**No cockpit HUD is drawn** (`0x10043b20` is not called on this path).

**The commander panel's draw** (`0x100836f0`) with the column left out draws three
things:
1. its resource rows (`+0x7c0`, `0x1006d510`), always, first;
2. the page header (`0x10083a20`);
3. for page 5 the factory panel (`+0x7bc`, `0x10097490`), with the pen at
   (51, 21) (`0x100838e2`).

With the column left out the list of the clan's factories is not drawn
(`0x100838eb`).

### The resource rows

Two rows, right to left from x 640 (`0x1006d6cc`): **Ore** (string 5092) at
y 0 and **Energy** (5093) at y 21 (`0x1006d7b0`). Each row, right to left:

| primitive | piece | x | |
|---|---|---|---|
| `0x10099a30`, variant 1 | `ending_text` | 634–640 | |
| `0x10099f60`, width 35 | `body_text` | 599–635 | the value, `"%d%%"` |
| `0x10099c90` | `separator_left_text` | 594–600 | |
| `0x10099d80` | `ray_emitter_off` | 584–595 | |
| `0x1009a380`, width 205 | `ray_body` | 379–585 | the label over the value's fill |
| `0x10099e70` | `ray_ending` | 373–380 | |

- **Colour.** The value is `#80ff80`, and `#ff3232` when it is 0
  (`0x1006d59c`: `0xff80ff80 + 0x7e32b2`).
- **A zero blinks.** The text is shown or hidden alternately every 0.5 s
  (`+0x4c`, `+0x48`).
- **What the values are.** They are [23-economy.md](23-economy.md#what-the-hud-shows--read)'s:
  held ore over 4500, and net power over the map's power. The displayed number
  steps toward them while the panel is drawn.

### The page header

The page header starts with the pen at (51, 0) (`0x10083aa8`):
- `ending_text` from 51 to 56;
- a `body_text` box 278 wide with the page's title centred in white `GAME_FONT`:
  string 1607 **Factory** for page 5 (`0x10083a6b`);
- an exit button (`0x1009a260`, variant 2) from 334 to 369, drawn with
  `exit_icon` (13 × 13) at (+15, +3).

The exit button's rectangle is (334, 0)–(369, 20) (`+0x5a4`–`+0x5b4`).

### The factory panel

**The box** is fixed by the panel's constructor at (51, 20)–(369, 148): `+0xc`
51, `+0x10` 20, `+0x18` 369, `+0x1c` 148 (`0x1009716c`). It is drawn with the
compound frame and filled in `0x80008000` (`0x1009afa0` from `0x100975f0`), as
the message box is. **The panel takes a click** anywhere in (51, 21)–(369, 185)
(`0x10098000`).

**Its text**, from the draw (`0x10097490`):

- **No project selected, none in production.** The panel reads two lines in
  `GAME_FONT` (`0x10097a7f`, `0x10097b16`), `#c0c0ff`:
  - 6240 *"There are no available projects."* at (71, 31);
  - 6241 *"Use warbot constructor to build bots."* one line step below. The step
    is `GAME_FONT`'s height + 2, over the vertical scale, rounded.
- **A project shown.** The project is the unit loaded from the view file (below):
  - its name in yellow `0xffffff00` `GAME_FONT` at (61, 31) (`0x10070700`);
  - its unit lines from the same pen (`0x1006fc00`, the designer's unit box,
    [38-designs.md](38-designs.md));
  - the unit turning in a 115 × 115 preview at (236, 22) (`0x1009e1b0`, drawn by
    `0x1009ee30`).
- **Recent projects exist but none is shown.** The panel selects project 0, or
  the one in `+4`, and shows it (`0x10097a06`).

**The icons in the box**, all on the page named in `ui/hq.cfg`:

| piece | rectangle | shown | colour | tooltip |
|---|---|---|---|---|
| `short_button_frame_off` / `_on`, then `project_icon` inset 2 | (59 + 23i, 122)–(82 + 23i, 143), icon (61 + 23i, 124)–(80 + 23i, 141) | for each recent project i (at most 5) | the icon white for the selected project, `#80ff80` otherwise | 3068 *Recent projects* |
| `long_button_frame_off` / `_on`, then `active_project` inset 2 | (184, 122)–(216, 143), icon (186, 124)–(214, 141) | while producing or in batch | white while no recent project is selected, `#80ff80` otherwise | 3069 *Constructed bot* |
| `free_bots_icon`, or `no_free_bots_icon` | (279, 126)–(309, 141) | always | `#80ff80` | 6248 *Ore not required, warbot components available*, or 6249 *Ore required* |
| `brain_icon`, then the free minds as a number at (344, 129) in `GAME_FONT` | (324, 126)–(339, 141) | always | `#80ff80`; `#c80000` and blinking every 0.5 s at 0 | 3067 *Available CPUs* |

- **Which free-bots icon.** It is chosen by the factory's property `0x803`,
  which answers `MBehaviour+0x9ec`, `FreeBotNum`
  (`Behavior.dll:0x1000a860`; `iron3d.dll:0x10086e20`).
- **The number of free minds** is the count of the clan's mind entries reading
  −1 (`0x10039330`, [23-economy.md](23-economy.md#the-bot-limit-is-the-clans-mind-count--read-and-measured)).
- **A frame shows `_on` for one frame after a click** (`+0x91c`, `+0x91d` + i).

**The bottom row** runs from the pen at (51, 150):

| x | primitive | piece | what |
|---|---|---|---|
| 51–56 | `0x10099a30`, variant 0 | `ending_stub` | |
| 56–106 | `0x1009a9e0`, variant 1 | long button, `constructor_icon` at (+10, +2) | **Warbot constructor** (1511) |
| 106–116 | `0x10099d80` | `ray_emitter_off` | |
| 116–252 | `0x1009a380`, width 136 | `ray_body` | the progress bar, `"%d%%"` of the progress; empty and 0 while idle; 0 in batch at 100 |
| 252–263 | `0x10099d80`, right to left | `ray_emitter_off`, mirrored | |
| 263–313 | `0x1009a9e0` | long button | **batch**: `batch_build_icon` while idle, 6238 *Start batch production*; `batch_stop_build_icon` otherwise, 6239 *Stop batch production* |
| 313–363 | `0x1009a9e0` | long button | **build**: `build_icon` while idle, 1553 *Start production*; `stop_build_icon` otherwise, 3070 *Stop production* |
| 362–368 | `0x10099a30`, right to left | `ending_stub`, mirrored | |

**The button primitive** (`0x1009a9e0`):
- It draws the long button's piece (50 × 19) for its variant.
- It draws the icon 30 × 15 at (+10, +2).
- The icon's colour is set by the variant: 0 grey `#808080`, 1 `#f0f0f0`,
  2 white.

**Idle and producing**, by the record's words
(`0x10086cf0` is 1 when idle and not batch):

- **Idle.** Both buttons show their start icons, enabled when a project is shown
  and a mind is free.
- **Producing.** Batch is enabled only in batch, and build only when not in batch.

*Measured*, the art: all 14 pieces sit on their pages with ink. The icons are
30 × 15 on `ui_menu` and `ui_menu3`, `project_icon` and `brain_icon` 15 × 15,
the short frames 21 × 22 and the long frames 36 × 22.

*Seen*, the recording at 960 × 720, halved to 640 × 480:
- the box is at about (52, 22)–(368, 147);
- the brain icon at 324–339 and the free-bots icon at 279–309;
- the bar from about 110 to 258;
- the header's exit button at 340–367.

### The view behind it — *seen*

**The green, doubled image behind the panel is the pod around the camera, not
the screen.**
- At 158.0 s the screen has closed and the HUD is back, and the view is still
  green and doubled.
- At 158.5–159 s, looking up, it shows the pod's ceiling.
- At 159.5 s, with the hero stepping off, it is gone.

Nothing in the mode-5 transition or draw tints the scene (*read*: the calls
listed above). The pod's look is [27-ownership.md](27-ownership.md)'s.

## What the controls do — *read*

A click on the page reaches the factory panel's handler (`0x10098040`) through
the commander panel's click (`0x100841a0`, page 5 at `0x10084ae2`):

| control | when | does |
|---|---|---|
| **Warbot constructor** | always | opens the warbot designer for this factory (`0x10055c70`; [37-designer.md](37-designer.md)) |
| **build** | a project shown | idle: start production (`0x100986e0` with 0); otherwise: stop (`0x10098720` with 0) |
| **batch** | a project shown | idle: start batch production (`0x100986e0` with 1); otherwise: stop (`0x10098720` with 1) |
| **active project** | | show the unit in production (`0x100985f0`): the selected project becomes −1, and the preview loads the file in production |
| **recent project i** | | select project i (`0x100982a0`) |
| **exit** (header) | | the page goes to 0, and the view mode is popped if it is 5 (`0x100846ac`) |
| **Esc** | | `CMD_ROLLBACK_STATE` (735, *"Leave warbot/HQ/..."*, bound to Esc in `ui_other.man`) pops the view mode (`0x10072238` → `0x10062ff0`) |

- **Start** does nothing without a free mind (`0x100986fb`).
- **Stop** acts only when its button's kind matches the record's batch word
  (`0x1009872a`). When it acts:
  - it aborts the factory's order 12 (`IAgent` slot 40, `0x10087260`);
  - it clears the file and the batch word;
  - if no project is selected, it selects project 0 (`0x10098750`).

**Leaving** (the pop's transition 5 → 0, `0x100644b0`):
- it hands the hero to the player again (`0x10074ff0` with 1, which sets
  `+0xa2`);
- it clears the view's building;
- it sets state 1 and restamps the interface's timer.

The hero is where it stood, in the pod (*seen*, 158 s). A pod fires again only
after its occupant has left and 5 s have passed
([27-ownership.md](27-ownership.md#capture--read)), so standing on does not
reopen the screen (*derived*).

**Esc in the character handler** (`0x10070db0`) does not close the screen itself.
In the view's state 1 it goes on, in turn, to:
- the objectives screen;
- hiding a shown message box (729);
- closing the wingman menu (740);
- command 748 in mode 0.

The screen closes on the bound command 735 (*read*; *seen*: at 158.2 s it
closes with the cursor still on the batch button).

## Projects — *read*, and *measured*

**The recent projects** are the warbot designer's (`+8` of the interface):
- five 0x7c-byte slots from `+0xb674`;
- the count is the filled ones (`0x100558da`), at `+0xb8e0`;
- `0x100557c0` gives slot i ([17-saves.md](17-saves.md), where they are
  saved).

**Accepting a design** in the designer adds it and selects project 0 on the
factory panel (`0x100517d2`). On failure the System says 2111 *"Cannot build
warbot!"* (`0x100517d9`). Accepting does not start production (*read*; *seen*:
at 155.6 s the panel shows *LFW-X Warrior* idle, until the batch button is
clicked at 157.4 s).

**Selecting project i** (`0x100982a0`):
1. **It refuses a project whose parts are not all researched.** For each part it
   asks the clan's technology tree (`0x10098780`, `0x10056ae0`).
2. **It writes the project to two files** in the game directory:
   - always to `UNITS\view_unit_<logic id>.dat`;
   - while the factory is idle and not in batch, also to
     `UNITS\bld_unit_<logic id>.dat`.

   Each file is `0xf0f1`, the unit's Type, and the assembly in the `.dat`'s own
   layout (`0x100569b0`, [07-objects.md](07-objects.md)).
3. **It loads the view file as a unit** of the player's clan for the preview
   (`0x100cd060`).

*Measured*: the install's `UNITS` holds 16 pairs and a `temp_unit.dat`.
- All 33 files start `0xf0f1` and load as assemblies.
- Of the 16 pairs, 14 are warriors (`0x1008000`) and 2 builders (`0x1004000`).
- 15 pairs are identical. In one, the build file (an M-42t) differs from the
  view file (an S-31): a project shown while another was in production.
- The pair for Tut_2's Large Factory (logic id −2147483647) is an **L-2f**
  flyer on `R_B_02`.

## Production — *read*

**Start** (`0x10086f90`, with the file and the batch word):
1. **It copies the file name** into the record's `+8` and the designer's last
   file, and sets `+0x19` to the batch word.
2. **It reads the file's `0xf0f1` and the Type.**
3. **It gives the factory building `ORDER_BUILDING_CONSTRUCT`** (12) through
   `IAgent` slot 3, replacing (3):
   - parameter: the Type;
   - target: `TARGET_BY_NAME` (`0x205`) with the file name (`0x10087150`–`0x10087181`).
4. **If the order is refused**, it clears the file and the batch word. Nothing is
   said.

**The factory's task** is `M_Task_Construct`.

Its start (`Behavior.dll:0x100299a0`):
1. **It finds the creation vertex.** It asks `IHallWay` for the first vertex
   with bit `0x800`, else `0x80`; without one, *"Bad HallWay in Plant"*.
2. **It checks the vertex's joint**, the second word of the vertex. If that node
   is destroyed, *"Plant creation node destroyed"*.
3. **It takes the target**, which must be by name (*"Invalid Construct
   parameter"* otherwise).
4. **It reserves a mind and sets its budgets**
   ([23-economy.md](23-economy.md#construction--read)):
   - a paid bot costs its design's ore and power, with 5 s of time;
   - a free bot (`FreeBotNum` > 0) costs no ore and 1 power, with time from the
     factory-by-chassis table.

**Each takt** the task's progress is min(time, ore, power) as fractions. The panel
shows it × 100 as the bar's `"%d%%"`, rounded to the nearest (`fistp`).

**The bot appears** (`0x1002a920`, when the three budgets are met):
1. **The reservation is dropped**, and `FreeBotNum` is decremented if above zero
   (`0x1002a7f9`).
2. **The unit is made from the file** (`CreateObjectFromScheme`, `0x1001d180`):
   - it is of the factory's clan;
   - its matrix is the identity (its heading 0, facing +x);
   - it stands at the factory's position, lifted to a ray's hit from above plus
     15 (`0x1002a9c4`).
3. **It is placed at the creation vertex.** Given the building and the vertex,
   the creator replaces the translation with the vertex's world position:
   *"Placing Robot inside Building … into Vertex …"*, `0x1001d4fe`.
   `IHallWay` slot 5 (`ArealMap.dll:0x1000a760`) gives that position: the
   vertex's point through its joint node's world matrix.
4. **It is given `ORDER_ROBOT_LEAVE`** (20), no target, replacing (`0x1002aa6e`).
   That is the escape: 400 tries at a random point within 150, and so on
   ([31-packages.md](31-packages.md#the-escape--read)). Its status line reads
   *escaping*.
5. **The log reads *"Construction Complete..."*.**

**What the player hears.** The world's object callback 2, registered by
`iron3d.dll` at `0x10033398`, is `0x10033490`. For a new object it:
- sends the object's clan SuperAI slot 4 event 1 with its id, which is how a unit
  joins its clan's list ([34-progression.md](34-progression.md#function-31-how-many-robots-a-clan-has--read));
- clears the designer's last file;
- for the player's clan, when not loading (`+0xe5`), queues `VOICE_UNIT_READY`
  (`vc_u_ready.wav`) through `0x10061ac0`.

That the construction is what raises the callback is *derived*: it is the only
robot the game adds at run time. No text line is made.

**Batch.** Every frame the clan's factory list is walked (`0x100874b0`, from
`0x1005edbe`):
- each record's idle word is refreshed;
- an idle factory in batch with a free mind is given its file again.

So batch production keeps building one design while minds are free, and waits
when there are none (*derived*).

*Measured*, the creation vertex:
- **Each factory model has exactly one vertex with bit `0x800`** (flags
  `0x20000800`), and no hall way in `fortif.rlb` has a vertex with `0x80`.
- **Its joint is node 4, `i01_0_m1o1`**:
  - `fr_b_plant`: vertex 1 (−0.30, −1.44, 7.58), which is (−0.30, 66.20, 2.30)
    in the model;
  - `fr_m_plant`: vertex 0, (−0.30, 26.21, 4.91);
  - `fr_l_plant`: vertex 0, (−0.30, −4.51, 4.91).
- **On Tut_2** the Large Factory is placed at (392.42, 788.73, 151.75), turned
  −0.0246. The bot appears at about (393.75, 854.91, 154.05).

## Mission 02 — *measured*, *derived* and *seen*

- **The build is a free bot of 60 s.** The Large Factory `lplant01.dat` is
  `fr_b_plant`, size 4, with `FreeBotNum` 100. The design is a large chassis,
  so the build takes no ore and 1 power, and 60 s
  ([23-economy.md](23-economy.md#construction--read)'s large × large).
- **One free mind.** Plr has 2 minds and the hero holds one, so the panel reads
  1, and 0 once the build starts ([34-progression.md](34-progression.md)).
- *Seen*:
  - 106.4 s: the screen opens. Energy counts up from 0; the CPU count reads 1;
    `free_bots_icon` shows.
  - 107.5 s: the designer.
  - 155.6 s: the panel again, with *LFW-X Warrior* and one recent project.
  - 157.4 s: batch is clicked. The CPU count goes red at 0, and the bar reads
    0% at 157.6 s and 1% at 157.8 s. At 1% of 60 s a second after the start
    would be 0.6 s, which agrees.
  - 158.2 s: the screen closes.
  - 232.5 s: the bot is outside, *"LFW-2 Warrior [escaping]"*; by 234.5 s it
    reads *"[no order]"*.

## For an engine

1. **Opening.** When the player's hero stands in the pod of the player's own
   factory and it fires:
   - select the factory (`VOICE_SELECTED` if it was not selected);
   - close the wingman menu;
   - hand the hero's control away (it stands);
   - push view mode 5;
   - hide the cockpit HUD;
   - draw the screen and the message box at (374, 352), 266 wide;
   - show the mouse cursor (*seen*).
2. **Draw, on 640 × 480:**
   - the Ore and Energy rows at the top right, as tabulated;
   - the header "Factory" (1607) from (51, 0) with the exit button at
     (334, 0)–(369, 20);
   - the box (51, 20)–(369, 148), framed and filled `0x80008000`;
   - its contents:

   | state | text | row |
   |---|---|---|
   | no project | 6240 / 6241 at (71, 31) in `#c0c0ff` | start icons, grey |
   | a project shown | yellow name at (61, 31), the unit lines, the 115 × 115 preview at (236, 22) | start icons |
   | producing | as shown, plus the active-project frame | stop icons, bar `"%d%%"` of progress × 100 |

   - the recent-project buttons at x 59 + 23i;
   - the free-bots icon at 279;
   - the brain icon and the free-mind count at 324 and 344;
   - the bottom row at y 150 as tabulated;
   - tooltips by rectangle.
3. **Controls:**

   | control | action |
   |---|---|
   | constructor | open the designer |
   | build and batch, idle | need a shown project and a free mind; start |
   | build and batch, running | stop, which aborts order 12 |
   | recent project i | select it (only if all its parts are researched); the view file always, the build file only while idle |
   | exit or Esc | pop mode 5 and give the hero back |

4. **Production:**
   - The factory runs order 12 with the design, by name. Mind, ore, power and
     time are docs/23's; a free bot is 1 power and the size-table time.
   - The progress is min of the three fractions.
   - On completion, create the design's unit for the factory's clan at the
     creation vertex (bit `0x800`, else `0x80`): the vertex's point through its
     joint node's world matrix, heading 0.
   - Give the unit `ORDER_ROBOT_LEAVE` with no target, and decrement
     `FreeBotNum`.
   - Add it to the clan's list: function 31 counts it, which completes *"Build
     a warbot"* at the next Mission run.
   - Queue `VOICE_UNIT_READY` for the player's clan.
   - In batch, restart while a mind is free.
5. **While up**, the mission runs on. The pod's green glass stays around the
   camera until the hero steps off.

## Not established

- The mechanism that puts the capture and the screen on one frame on Mission 02
  ([27-ownership.md](27-ownership.md)).
- The stat lines `0x1006fc00` draws in the box, and the preview's camera and
  turn rate. They are shared with the designer
  ([37-designer.md](37-designer.md), [38-designs.md](38-designs.md)).
- What handing the hero back does to it while the screen is up, beyond clearing
  `+0xa2` ([39-boarding.md](39-boarding.md)), and whether the player's keys
  still move it.
- Whether the designer pauses the world. It sets bit 8 of the level's flag word
  when it opens and clears it when it closes (`0x10055c9b`, `0x10055d27`), and
  what that bit does was not read.
- How the cursor is shown in mode 5 (`0x100a4fc0` in state 1 was not followed).
- The heading the escape leaves the new bot with, and whether a flyer climbs on
  its way out.
- What commander pages 1–4 and 6–8 show from first person; only page 5 is read
  here.
