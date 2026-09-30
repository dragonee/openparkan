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
- It clears the keyboard: `World3D.dll`'s `stdClearKeyboard` (`0x10064452`),
  which drops every pending key and mouse message and the key state
  ([39-boarding.md](39-boarding.md#boarding--read)).
- It turns the outer camera off (`0x10038ad0`) and clears the unit selection
  (`0x1007d270`).
- It hands the hero back from the player (`0x10074ff0` with 0, which clears the
  record's `+0xa2`; see [below](#what-the-hero-does-while-the-screen-is-up--read)),
  then gives the hero's own word to the player (its `Wizard.dll` object's slot 9
  with mask 1 and 3, `0x10064480`–`0x10064485`).
- It stores the building as the view's building (level `+0xaf0`, `0x100a5680`)
  and the hero as the view's unit (`+0xaec`, `0x100a5660`).
- It sets the level's state word back to 1 (`0x100a4f90`).

The world is not paused: nothing on a building's screen or in the designer
writes the pause byte, and the `Mission` handler runs on
([below](#the-designer-does-not-pause-the-world--read)).

*Seen*, on Mission 02: the screen is up at 106.4 s. It appears on the same frame
as *"from: System / Building is captured"* (string 5039), and the cockpit's HUD
goes.

**The capture and the opening are one call** (*read*). The building's computer
calls its callback once, from its tick, when the pod has opened on the hero
([27-ownership.md](27-ownership.md#capture--read)). For another clan's building
the callback (`0x10061050`) changes the owner and calls the ownership change
(`0x100611b0` → `0x100a48a0`), and that routine, on every path:
1. shows 5039 as a System line (`0x1007eb60`, at `0x100a49d9`, `0x100a4aa6` or
   `0x100a4b73`, one for each voice) and queues the voice;
2. for a plant, makes the clan's factory record (`0x100875e0`, `0x100a4c02`),
   which the opening hands the factory panel;
3. ends in the opening itself (`0x10062630`, `0x100a4e2d`), which every branch
   joins at `0x100a4e1e`.

For the player's own building the callback calls the opening straight away
(`0x100611cc`). So the line, the record and mode 5 all come out of one building
tick, and the frame the capture is shown on is the frame the screen opens on.

### What the hero does while the screen is up — *read*

**It stands, and the player's keys do not move it.**
- **Handing back** (`0x10074ff0` with 0, `0x1007510c`) sends the hero's agent
  the object message (6, 7, 0). Its `Wizard.dll` object takes the 0 as its mode:
  **0, the AI's** (`Wizard.dll:0x10001ced`). It then sets every one of the
  Wizard's words to 2, neither side's (mask `0xfff`), and the word for mask
  `0x10` to 1, the AI's (`0x10075122`–`0x10075145`). It clears `+0xa2`.
- **The keys' rows run only in the player's mode.** The Wizard hands the frame's
  update to the manual manager that runs the key rows only while its mode is 1
  (`Wizard.dll:0x10001ca2`,
  [14-controls.md](14-controls.md#a-row-that-stays-down--read)). With mode 0 no
  row runs, so no key, mouse movement or button reaches the hero.
- **Its AI does not move it either.** The transition's word 3 for mask 1
  (`0x10064485`) is the unit's own word, `+0x200`. With it at 3 the Wizard does
  not follow its points (the Wizard's takt,
  [24-motion.md](24-motion.md#how-the-ai-drives-a-machine--read-and-measured)), and
  the behaviour's movement and fight flags, which need a word of 1 or a 0 in
  the AI's mode (`Wizard.dll:0x10003890`), are all off.
- **What its AI keeps** is mask `0x10`'s word, `+0x208`: the group whose
  components the permission loop finds in class group 5, the shields and armour
  ([29-weapons.md](29-weapons.md#who-may-drive-a-units-guns--read)).
- **Leaving** hands it back to the player (`0x10074ff0` with 1: every word 3 and
  (6, 7, 1)), and sets `+0xa2` again.

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

**From first person only pages 4 and 5 are ever up** (*read*). Mode 5 draws
whatever page the panel is on, but only the pods turn it there: 5 for a plant,
4 for a research centre (`0x10062723`, `0x1006276a`). The other ways to a page
are closed in mode 5:
- the column, which opens pages 1 to 8, is not drawn, and the panel's click
  reaches the column's buttons only in modes 3 and 4 (`0x1008427a` sends mode 5
  to `0x10084389`, which tests for 3 and 4 at `0x1008439b`);
- the letter keys that turn the page act only in mode 3 (`0x10071782`);
- a click in the world turns to a unit's page only for the unit it picks
  (`0x10090640`–`0x10090679`), and the pick answers nothing in view state 1
  (`0x1008daa4`, [42-selection.md](42-selection.md#the-mouses-way-in--read)).

The page setter (`0x10084d80`) has 42 calls, all in `iron3d.dll` (a raw scan of
its `call`s). Besides the pods, the column, the keys and the world click, they
pass 0, the page already up, or 5 (a factory row, `0x100861fe`), or sit in the
transitions into modes 3 and 4
(`0x10063a20`, `0x10063ca0`) and in a click handler that works on command mode's
selection (`0x1008fb00`); none was found turning a page in mode 5.

So what pages 1–3 and 6–8 show is command mode's, and is
[41-commander.md](41-commander.md#the-pages--read)'s.

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
- **The fill is the bar's own** (*read*). Each row hands the bar primitive its
  displayed value (`+0x24` for Ore at `0x1006d770`, `+0x28` for Energy at
  `0x1006d860`), the value's colour for the label, and nothing for the fill.
  The primitive picks the fill's colour from the percentage itself
  (`0x1009a530`–`0x1009a54c`, and again for the other direction at
  `0x1009a665`–`0x1009a685`): `0x80800000` under 20, `0x80808000` under 80,
  `0x80008000` from 80, and no fill at 0 or below. These are the weapons list's
  colours ([35-hud.md](35-hud.md#the-weapons-list--read-and-measured)) because
  it is the same primitive; all 12 of its calls in `iron3d.dll` get them.
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
| 116–252 | `0x1009a380`, width 136 | `ray_body` | the progress bar, `"%d%%"` of the progress; empty and 0 while idle; 0 in batch at 100; filled to that percentage |
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

## The cursor in mode 5 — *read*

**Mode 5 shows the cursor the way command mode does**, through the same flag and
the same draw ([42-selection.md](42-selection.md#the-cursor-shows-a-state--read-and-measured)).

- **The screens' draw turns it on.** In mode 5 with the hero as the view's unit
  it sets the cursor-shown byte `0x1010b5c8` (`0x1008d4f1`) and, when the
  display's slot 12 answers, calls `USER32`'s `ShowCursor(TRUE)` until the
  system's count is positive (`0x1008d508`–`0x1008d516`; the import is
  `0x100e41bc`). Over the designer it does the same (`0x1008d457`), and so does
  the designer's own opening (`0x10055cb2`). The case for mode 0 turns it off
  again: cursor state 1, the byte cleared, `ShowCursor(FALSE)` until the count
  is negative (`0x1008d3b8`–`0x1008d3ee`), and so does mode 5 with a driven bot
  as the view's unit (`0x1008d4a9`).
- **The frame draws it.** While the byte is set the game frame runs the chooser
  and the draw (`0x10058710`, called at `0x10060c95` unless the cursor is in
  state 8).
- **It is `ARROW`.** The chooser takes the state from the pick's kind
  (`0x10058740`), and the pick answers kind 0 in view states 1, 3, 4 and 6 and
  while the designer is up (`0x1008daa4`–`0x1008dae5`, the early exit
  `0x1008e1b1` returns 0). Mode 5 runs in view state 1, so the state is 1, and
  its draw is `ARROW`'s object `0x1010b2d8` (`0x100586c6`).
- **Software or system.** The software strip is drawn only when slot 12 does not
  answer (`0x10057060`), as in command mode; which it is on a given display is
  not read (42-selection's *Not established*).

`0x100a4fc0`, which the transition's state setter ends in and which this page
once left unfollowed, is not the cursor: it hands the mouse filter the zoom of
the view state's unit ([30-turrets.md](30-turrets.md#the-zoom--read-and-measured)).

## The designer does not pause the world — *read*

**The flag word it sets is the view's, and its flag 8 is "draw no world".**
- **Whose word.** The designer's opening passes 9 to slot 13 of the level's `+0`
  (`0x10055c90`–`0x10055c9b`). That is the view the world is drawn from: the
  state setter stores there what `0x100a1c30` picks for the view state
  (`0x100a4fb2`), and in view state 1 that is the view's unit's camera view
  (`0x1007e6a0`, its record's `+0x4c`). On a building's screen that unit is the
  hero; in command mode the view is the command camera's.
- **What the word is.** It is `Terrain.dll`'s `CCamera` flags (vtable
  `0x1009c620`), slot 13 storing it whole at `+0x164` (`0x10084a79`) and slot 20
  reading it (`0x10084ad6`). The unit camera's getter turns flag 1 on
  (`0x1007e707`), and `0x20` is the infrared
  ([35-hud.md](35-hud.md#the-indicators--read-and-seen)).
- **What 8 does.** `CCamera::Render` (`0x100844c0`,
  [03-terrain.md](03-terrain.md#when-it-draws--read)) tests it twice. With it
  set it skips the world's `0x800` pass (`0x100845e5`) and the world and its
  `0x1000` pass (`0x100846fb`). Of the 18 instructions in `Terrain.dll` that
  address a `+0x164`, those two are the only tests of 8.
- **Closing** clears 8 alone (`0x10055d27`, and the exit, accept and Esc paths at
  `0x100514a0`, `0x10051843`, `0x10055f04`). A search of eight modules
  (`iron3d`, `Control`, `World3D`, `Terrain`, `Behavior`, `AniMesh`, `Effect`,
  `ai`) for a slot-20 call followed by a mask of 8, or a slot-13 call handed 8
  or 9, finds these five sites; its three other hits call other interfaces with
  three or four arguments (`iron3d.dll:0x1005abb7`, `Behavior.dll:0x10032a9d`,
  `0x10032b93`).

So **behind the designer the world is not drawn, and it goes on running.** The
opening's 9 replaces the whole word, so on a building's screen **the hero's
night sight is off after the designer** (*derived*).

**Nothing on these screens pauses the game.** The pause byte `+0xe8` has two
writers in `iron3d.dll`: the game's constructor (`0x1005c512`) and the setter
`0x1005f620`, which alone calls `World3D`'s `PauseGameTime` and
`ResumeGameTime`. The setter's five calls are the interface's pause and resume
(`0x1008d830`, `0x1008d850`), which the key-down handler's code `0x13`
toggles while playing (`0x10071094`–`0x100710e9`) and Esc lifts
(`0x10070df2`); the help screen opening and closing
(`0x10067895`, `0x10067935`) and a widget's show slot, which pauses while it
is shown (`0x100656d3`, slot 11 of the vtable at `0x100e61ec`). That widget
is the game menu's (*guess*: the transition into mode 7 calls slot 11 of the
interface's `+0xc` with 1, `0x10064616`). Neither the transitions into and out
of mode 5 (`0x10064430`, `0x100644b0`) nor the designer's open and close call
it. The search is the one that finds the setter's own store at `0x1005f630`,
the thing known to be there.

**So the `Mission` handler runs** while a building's screen or the designer is
up: its gates are the pause byte, the game's state word at 3 and the level's
view state at 5 (`0x1005ed5d`–`0x1005ed7f`,
[34-progression.md](34-progression.md#when-the-mission-handler-runs--read)), and
mode 5 sets the view state to 1.

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

**Esc in the key-down handler** (`0x10070db0`) does not close the screen itself.
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
- **One free mind** (*read*, and *measured*). Plr has 2 minds, and the hero,
  its one placed robot, took one as the mission placed it
  ([23-economy.md](23-economy.md#the-bot-limit-is-the-clans-mind-count--read-and-measured)),
  so the panel reads 1, and 0 once the build starts.
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
   - hand the hero's control away: no key reaches it and its AI does not move
     it, so it stands;
   - push view mode 5;
   - hide the cockpit HUD;
   - draw the screen and the message box at (374, 352), 266 wide;
   - show the cursor, `ARROW`, as command mode shows its own;
   - capturing a plant and opening it are one step: its System line and its
     screen come on the same frame.
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
   - the bottom row at y 150 as tabulated, the progress bar filled from 2 in
     from its left edge to its percentage of 136, in the weapons list's colours;
   - the resource rows' fills in the same colours;
   - tooltips by rectangle.
3. **Controls:**

   | control | action |
   |---|---|
   | constructor | open the designer |
   | build and batch, idle | need a shown project and a free mind; start |
   | build and batch, running | stop, which aborts order 12 |
   | recent project i | select it (only if all its parts are researched); the view file always, the build file only while idle |
   | exit or Esc | pop mode 5 and give the hero back |

   So a player looks through the recent projects while a build runs on, and the
   active project's button goes back to it. *The engine's*: the panel opens on
   the unit in production, its button lit, each time it comes up (from the pod,
   or on the commander's page 5 or another plant's panel); what `+0x04` holds
   across a closing and a reopening is not read.

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
   - A plant that changes owner drops its build (*seen*: C02 Mission 01's
     enemy Factory, taken from its pod while building, opens on the mission's
     prebuilt *SWW-X Warrior* with nothing in production — "Let's Play - Parkan:
     Iron Strategy, Part 3", 9:36). How the capture ends order 12 is not read.
5. **While up**, the mission runs on, the `Mission` handler included; neither the
   screen nor the designer pauses anything. The designer draws no world behind
   it, and puts the hero's night sight out. The pod's green glass stays around
   the camera until the hero steps off.

## Not established

- ~~The mechanism that puts the capture and the screen on one frame on Mission 02~~
  — **read**, and already in [27-ownership.md](27-ownership.md#capture--read)
  before this line was queued: the ownership change ends in the opening
  (`0x100a4e2d`), so one pod firing both takes the plant and opens it
  ([How the screen opens](#how-the-screen-opens--read)).
- ~~The stat lines `0x1006fc00` draws in the box, and the preview's camera and
  turn rate~~ — **read**, in the designer's docs: the lines are the unit box
  ([38-designs.md](38-designs.md#the-unit-box--read-and-measured)), and the camera
  stands on −x looking along +x, its 60° across the view's width
  ([37-designer.md](37-designer.md#the-previews--read-and-seen)). What scene colour
  a preview's materials take while the screen is up is still open there.
- ~~What handing the hero back does to it while the screen is up, beyond clearing
  `+0xa2`, and whether the player's keys still move it~~ — **read**: its Wizard
  goes to the AI's mode, in which no key row runs, and with its own word at 3
  and every other at 2 neither the player nor its AI moves it; only its shields
  and armour stay the AI's
  ([What the hero does](#what-the-hero-does-while-the-screen-is-up--read)).
- ~~Whether the designer pauses the world (bit 8 of the level's flag word,
  `0x10055c9b`, `0x10055d27`)~~ — **read**: it does not. The word is the view's
  camera flags, and 8 stops the world being drawn behind the designer; nothing
  on a building's screen or in the designer writes the pause byte, so the
  `Mission` handler runs
  ([The designer does not pause the world](#the-designer-does-not-pause-the-world--read)).
- ~~How the cursor is shown in mode 5 (`0x100a4fc0` in state 1 was not
  followed)~~ — **read**: as in command mode, the screens' draw turning it on and
  the frame drawing it, always `ARROW` because the pick answers nothing in view
  state 1; `0x100a4fc0` is the mouse filter's zoom
  ([The cursor in mode 5](#the-cursor-in-mode-5--read)). Whether it is the
  system's cursor or the software strip is the display's slot 12, which
  [42-selection.md](42-selection.md#not-established) has not read.
- ~~The heading the escape leaves the new bot with, and whether a flyer climbs on
  its way out.~~ — **read** on 2026-09-22. The escape sets no heading of its
  own: the bot is made facing +x and ends facing along the Wizard's heading
  curve, a Hermite cubic from the hull's forward axis to the point's heading
  (`Wizard.dll:0x10003d80`,
  [24-motion.md](24-motion.md#the-heading-curve--read)). A flyer climbs: the
  walker cuts its leg into a point every 20 across the ground and stands each
  15 over the ground under it, or 100 over a building's, a tree's or a stone's
  top (`Behavior.dll:0x10040f20`, `0x100146b0`,
  [24-motion.md](24-motion.md#a-flyers-walk-points--read-and-measured)), so
  Mission 02's escape point over the Large Factory's hall stands 115 m over its
  roof. `Movement_FlyHeight` and `Movement_FlyNearLandHeight` reach no point:
  their one reader (`0x100153a0`) is called by nothing.
- ~~What commander pages 1–4 and 6–8 show from first person; only page 5 is read
  here~~ — **read**: from first person only pages 4 and 5 are ever up. Mode 5
  reaches no other page: the column works only in modes 3 and 4 and the page
  keys only in mode 3 ([What is drawn](#what-is-drawn--read-and-seen)). What
  the other pages show is command mode's, in
  [41-commander.md](41-commander.md#the-pages--read).
