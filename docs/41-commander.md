# The commander panel — the icon column, the unit pages and the orders

What the player works with in command mode: the column of icons down the left
edge, the pages it opens, and the order rows a page offers for the selection.
This page reads the panel's pieces, what each control does and which rows are
offered, and measures them against the install's art and a recording of *The
Field Base* (`CAMPAIGN.00/Mission.03`), whose Small Bunker gives the player
command mode.

Entering and leaving command mode, its camera and the world drawn behind the
panel are [40-command-mode.md](40-command-mode.md)'s, and so is picking in the
world: what a click on the ground or a unit does, the pick modes a row enters and
the building placed with a build order. The panel's draw with the column left
out, its resource rows, its page header and page 5, the factory panel, are
[36-factory.md](36-factory.md)'s. The order table and the orders a row gives are
[31-packages.md](31-packages.md#the-commanders-menus--measured-and-read)'s. The
numbers a unit box prints are [38-designs.md](38-designs.md#the-unit-box--read-and-measured)'s.

**Every claim is tagged**, as in [15-behaviour.md](15-behaviour.md):
- *measured* is re-derived by `openparkan verify`;
- *read* comes from the disassembly at the address given;
- *derived* follows from the two;
- *seen* is taken from the recording;
- *guess* fits and is not established.

Coordinates are the HUD's 640 × 480 layout
([35-hud.md](35-hud.md#everything-is-drawn-on-a-640--480-screen--read)). The pen
primitives and the compound-control skin (`ui/compaund.cfg`) are read there too.
The recording is 960 × 720; its positions are divided by 1.5.

## Where it is drawn — *read*

**The panel is the screens object's `+0x18`.** The screens load
(`0x1008d5d0`) builds it (`0x10082550`); its members are the column's widgets
(`+0x04`–`+0x3c`), the unit box's sprites and tooltips, the order menu
(`+0x624`), the research panel (`+0x7b8`), the factory panel (`+0x7bc`) and the
resource rows (`+0x7c0`).

**Its draw** (`0x100836f0`, *hide*, *column left out*) is called three ways by
the screens draw (`0x1008d200`):

| when | arguments | what shows |
|---|---|---|
| `CState` modes 3 and 4 (`0x1008d51c`), no designer up | 0, 0 | everything: resource rows, the column, the page |
| mode 5 (`0x1008d444`), the view's unit not driven | 0, 1 | resource rows and the page, which is only ever 4 or 5 there ([36-factory.md](36-factory.md#what-is-drawn--read-and-seen)) |
| the cursor kind (`0x10104148`) is 8 (`0x1008d326`) | 1, 0 | the resource rows alone |

- **In modes 3 and 4** the screens draw first shows the cursor, then the unit
  markers of the level's unit list (`0x1007d5e0`) and its building list
  (`0x1007db00`), then the panel, and after it, as in the cockpit, the satellite
  map, the message box, the game's `+0x30` overlay and the objectives screen.
- **The cursor kind** is set through `0x100571a0`, which draws no cursor sprite
  for kind 8 (its case table sends 8 to the no-op `0x10057619`). *Seen*: at
  182 s, while the mine's red model follows the mouse, only the Ore and Energy
  rows are on screen — no column, no page. Which picks set kind 8 is
  [40-command-mode.md](40-command-mode.md)'s.

**The draw itself** (`0x100836f0`):
1. the resource rows (`+0x7c0`), always;
2. unless *hide*: the column (below), unless *column left out*;
3. the page, **only while the column is fully out** (its step count `+0x5ec`
   above 15), or always when the column is left out. Pages 1–3 draw the header,
   the unit page (`0x10085530`) and the order menu (`0x1007b1e0`); page 4 the
   header and the research panel (`0x100898a0`); page 5 the header, the factory
   panel at (51, 21) and, with the column, the factory rows; pages 6–8 the header
   and their building rows (jump table `0x100839f8`).

**Every frame** in modes 3–5 the screens takt (`0x1008d5f0`, called from the game
loop at `0x1005eb3f`) runs the column's update (`0x10083c60`, at `0x1008d665`).
**A mouse press** (`0x100714d0` → `0x1008d690`) in modes 3–5 inside (374, 0)–(640,
42), the resource rows, is swallowed. Anywhere else it
goes to the panel's click (`0x100841a0`) and, when that does not take it, to the
world (`0x1008fe80`, [40-command-mode.md](40-command-mode.md)). While the cursor
kind is 8 it goes straight to the world.

## The icon column — *read*, *measured* and *seen*

### Its pieces

Every piece is an `ui/hq.cfg` sprite on `ui_menu` (*measured*: all on their page
at their size, with ink). The widgets sit at x 0, one under another:

| member | y | widget (`0x1009c030`, 46 × 29) | icon (23 × 23) | tooltip | page it opens |
|---|---:|---|---|---|---:|
| `+0x04` | 0 | the head, `objpanel_head` (46 × 12) | | | |
| `+0x10` | 12 | button | `objpanel_icon_hero` | 1612 *Strategic control off* | — |
| `+0x08` | 41 | separator, `objpanel_separator` (46 × 24) | | | |
| `+0x14` | 65 | button | `objpanel_icon_warbots` | 1600 *Battle units* | 1 |
| `+0x1c` | 94 | button | `objpanel_icon_builder` | 1602 *Builders* | 3 |
| `+0x18` | 123 | button | `objpanel_icon_cargo` | 1601 *Transports* | 2 |
| `+0x08` | 152 | separator | | | |
| `+0x20` | 176 | button | `objpanel_icon_restree` | 5080 *Research Center* | 4 |
| `+0x08` | 205 | separator | | | |
| `+0x24` | 229 | button | `objpanel_icon_factory` | 1607 *Factory* | 5 |
| `+0x28` | 258 | button | `objpanel_icon_tower` | 1614 *Battle tower buildings* | 6 |
| `+0x2c` | 287 | button | `objpanel_icon_bunker` | 1608 *Bunker buildings* | 7 |
| `+0x30` | 316 | button | `objpanel_icon_build` | 1610 *Other buildings* | 8 |
| `+0x08` | 345 | separator | | | |
| `+0x34` | 369 | button | `objpanel_icon_chat` | 5038 *Chat* | — |
| `+0x38` | 399 | button | `objpanel_icon_map` | 1501 *Satellite map* | — |
| `+0x3c` | 428 | button | `objpanel_icon_system` | 1302 *Game menu* | — |
| `+0x0c` | 457, or 12 | the lock (`0x1009c420`, 46 × 24), `objpanel_lock_icon` (13 × 13) | | | |

- **The positions** are the constructor's (`0x10082550`, each button made by
  `0x1009c030` with x 0, its y and its tooltip id); one separator widget is
  moved to each of its four places as the column draws (`0x10083741`–`0x10083826`).
  The widget takes its size from its first sprite (`0x1009b730`).
- **Builders sit above Transports, but open page 3**, and Transports page 2
  (*read*: the click's page numbers and the header's jump table `0x10083c00`).
- *Seen*, at 180 s, the icons' middles at y 79, 110, 137, 247, 299, 330, 413,
  442 and the lock's at 472, against 79.5, 108.5, 137.5, 243.5, 301.5, 330.5,
  413.5, 442.5 and 470.5 as read — within 3, where the cursor and the red REC
  mark leave the measure rough.

### A button's look

A button (`0x1009bad0`'s three states, drawn by `0x1009bec0`) has three sprites,
`objpanel_button1`, `2` and `3`, and draws its icon inset by (16, 3):

| state | when | sprite | icon colour |
|---|---|---|---|
| 0 | disabled (`+0x21` clear) | `objpanel_button1` | `#646464` |
| 1 | enabled | `objpanel_button2` | `#80ff80` |
| 2 | on (`+0x1c8` set) | `objpanel_button3` | white |
| 1 and 2 by turns | enabled and under the mouse | every 0.1 s (`0x1009bf61`) | as its state |

The lock is the same widget with `objpanel_lock1`–`3` and its icon inset by
(18, 7). A hovered button hands the tooltip manager its text (`0x1009c3a0`).
*Seen* at 180 s: a disabled button's lamp is dark and its icon grey; the builder
icon under the cursor is white.

### What enables a button, and what lights it

The update (`0x10083c60`) sets them afresh every frame:

| button | enabled while | on while |
|---|---|---|
| hero | always | never |
| Battle units | the player's clan has a unit of Type within `0x1018000` (warriors and HQs) (`0x10072be0`) | page 1 |
| Transports | … within `0x1002000` | page 2 |
| Builders | … within `0x1004000` | page 3 |
| Research Center | the clan has a building of Type `0x80000400` not building itself (`0x1007dee0` with 1) | page 4 |
| Factory | … `0x80000010`, likewise | page 5 |
| Battle tower buildings | … within `0x80300000`, likewise | page 6 |
| Bunker buildings | … within `0x80070000`, likewise | page 7 |
| Other buildings | the clan has a building within `0x8000044e` (generator, mine, storage, Outpost, institute), even one building itself (`0x1007dee0` with 0) | page 8 |
| Chat | a network game (the game's `+0xe4`) | the game's `+0x30` overlay is shown (its `+0x24`) |
| Satellite map | always | the map is open (its `+0x261`) |
| Game menu | always | the game menu screen is shown (screens `+0x0c`, its `+0x20`) |

- **"Building itself"** is property `0x20c` non-zero (`0x100344c0`): the
  construction sphere is running ([32-builder.md](32-builder.md#upgrading-a-building--read)).
  The walker leaves out removed objects (owner word `0xfffe`).
- **A page whose button is disabled closes.** For pages 4–8 the update turns the
  page to 0 when the page's button reads disabled (`0x10084154`); pages 1–3 close
  when their unit list comes out empty (below).
- *Derived* against *seen*, Mission 03 at 180 s, just after the Small Bunker's
  capture: the player's clan holds the hero, `tut3_b` (a builder), `tut3_t` (a
  transport), the Large Factory, the Small Warehouse, and the Power Generator and
  Small Bunker it has taken. By the table, Battle units, Research Center, Battle
  tower buildings and Chat are disabled and the rest enabled — which is what the
  recording shows, lamp for lamp.

### The lock: the column slides in and out

- **Open**, as the constructor leaves it (`0x10083c20`): the step count `+0x5ec`
  is 16, `+0x614` is set and the lock stands at y 457.
- **The column draws one item more per step** (`0x10083733`–`0x10083879`): the
  head always, then the hero button at step 1, the first separator at 2, and so
  on down to the game menu at 15, the lock last and always. The page draws only at
  step 16.
- **Closing.** A click on the lock while the column is open and still
  (`0x10084297`) starts the slide: `+0x615` is set, the lock is hidden, and
  `BAR_OPEN` plays (`i_bar_op.wav`, *measured* binding in `ui/game_resources.cfg`).
- **Opening.** While the column is shut and still, **the mouse merely resting on
  the lock** starts the slide out the same way (`0x10083d41`, the lock's hit test
  in the update, not the click).
- **The slide** steps the count by one every 0.05 s (`0x10083e1d`), down while
  closing and up while opening, so it takes 0.8 s. At 0 the column is shut and
  the lock shows again at y 12; at 16 it is open and the lock shows at y 457.
- **Not in mode 5.** With the view's mode 5 at the front (`0x10083d29`,
  `0x1008427a`), the lock does nothing: that screen draws no column.

### What a click on the column does

The panel's click (`0x100841a0`) tries, in order:
1. **the lock**, as above;
2. **the satellite map's rectangle at its `+0x230`**, while the map is open:
   close the map (`0x10074100`) — what that rectangle is was not followed;
3. **with the column open and the `CState` mode 3 or 4**, the buttons, each only
   when enabled and hit:
   - **hero** — roll the `CState` back to mode 0 (`0x10062ce0` with 0, 0,
     `0x100843c6`): strategic control off, the hero's own view
     ([40-command-mode.md](40-command-mode.md));
   - **a page button** — page 0 if its page is already on, else its page
     (`0x10084d80`);
   - **Chat** — show or hide the game's `+0x30` overlay;
   - **Satellite map** — open or close the map (`0x100740f0`, `0x10074100`);
   - **Game menu** — push `CState` mode 7 (`0x10062bc0`, `0x10084674`);
4. **the page header's exit button** (334, 0)–(369, 20), while a page is up: page
   0, and pop mode 5 when it is the front ([36-factory.md](36-factory.md#what-the-controls-do--read));
5. **the page's own controls** (jump table `0x10084b5c`), below.

A click nothing takes returns 0 and falls through to the world.

## The pages — *read*

`0x10084d80` (*page*) stores the page at `+0x5f0` and selects for it
(`0x10084e60`):

| page | header (`0x10083a20`) | selects within | shows |
|---:|---|---|---|
| 0 | — | — | nothing: the column alone |
| 1 | 1600 *Battle units* | `0x1018000` | the unit page |
| 2 | 1601 *Transports* | `0x1002000` | the unit page |
| 3 | 1602 *Builders* | `0x1004000` | the unit page |
| 4 | 5080 *Research Center* | — (the research panel's refresh `0x10089fd0`) | the research panel |
| 5 | 1607 *Factory* | buildings `0x80000010` | the factory panel and the factory rows |
| 6 | 1614 *Battle tower buildings* | buildings `0x80300000` | building rows |
| 7 | 1608 *Bunker buildings* | buildings `0x80070000` | building rows |
| 8 | 1610 *Other buildings* | buildings `0x8000044e` | building rows |

Any other page's header reads 6205 *Unknown*.

**Selecting for a page** (`0x10084e60`):
- **units**: if exactly one unit is selected and its Type lies within the mask,
  it stays; otherwise the selection is cleared (`0x1007d270`) and the clan's first
  unit within the mask is selected (`0x1007d0a0`, with `VOICE_SELECTED`). The list
  is rebuilt, and the order menu is placed under it and opened for that one unit;
- **buildings**: likewise with the building selection (`0x1007db80`, which also
  clears the unit selection).

**The update keeps a page current** every frame (`0x10084045`): pages 1–3 turn
their button on, rebuild the unit list (`0x10085100`), place the order menu under
it (`0x1007b1a0`) and let it rebuild itself (`0x1007b4a0`); pages 5–8 turn their
button on and rebuild the building rows (`0x10085340`); pages 4–8 close when
their button is disabled.

## The unit pages, 1 to 3 — *read*, and *seen*

### The list

`0x10085100` (*mask*), every frame:
- takes the clan's units whose Type lies within the mask (`0x1007d7f0` on the
  level's unit list, `+0x720`), in the list's order;
- makes a row for each (a 20-byte record, kind 1, `0x10095b30`);
- **turns to page 0 when there are none** (`0x100852b4`);
- **selects the first when none of them is selected** (`0x10084e60`).

### The box

The unit page (`0x10085530`) frames (51, 20)–(369, 117) and fills it
`0x80008000` (`0x1009afa0`). By how many units are selected (`0x10076e70`, the
selected units, and none unless the game's `+0x08` is 4):

| selected | the box shows |
|---|---|
| none | 5076 *No bots selected*, green `0xff00ff00`, at (101, 30) |
| one | that unit (`0x10085890`), below |
| more | 5077 *%d bots selected*, green, at (101, 30) |

**One unit** (`0x10085890` at (51, 20)):
- **two icons** 15 × 15 at (60, 27) and (77, 27) (`0x10077120`, from `ui_menu`,
  *measured* with ink):
  - the first by Type: a builder (65, 110), a transport (81, 110), a warrior, HQ
    or hero (49, 110);
  - the second by the unit's property `0x207`, 1 to 4: (97, 94), (49, 94),
    (65, 94), (81, 94);
  - both tinted by the record's `+0x30`: 1 `0xffffe7ff`, 2 `0xffff8080`,
    3 `0xff80ff80`, 4 `0xff8080ff`. *Seen*: the Mission 03 builder's and warriors'
    icons are red.
- **its name and status**, `"%s [%s]"` — the unit's name and its head order's
  status line (`0x10076f90`, [31-packages.md](31-packages.md#the-orders--measured))
  — in yellow `0xffffff00` at (101, 30);
- **the unit lines** of [38-designs.md](38-designs.md#the-unit-box--read-and-measured)
  (`0x1006fc00`) with an empty name, pen at (122, 27);
- **the buttons along the bottom**, each a 21 × 21 `short_button_frame_off` with a
  15 × 15 icon inset by 3:

  | button | frame | icon | tooltip | lit (white; else `#808080`) when | a click (`0x1008470d`) |
  |---|---|---|---|---|---|
  | A | (60, 88) | `buildscreen_hq_icon`, a palm | 1502 *Manual* | the unit can be boarded (`0x10076d30`) and its head order is not 24, upgrade | take the unit at auto-driver level 0 (`+0x9c`): `CState` mode 2 with it, page 0 |
  | B | (84, 88) | `botscreen_autogunner_icon` | 1519 *Gunner* | as A | the same at level 1 |
  | C | (108, 88) | `botscreen_autodriver_icon` | 1518 *Automatic* | as A | the same at level 2 |
  | D, only for Type `0x1010000` | (138, 88) | `buildscreen_direct_icon` | 1517 *Strategic control* | `IsHQ` ([30-turrets.md](30-turrets.md#an-hq-unit-in-play--read)) | mode 3 with it, page 0 |
  | E | (324, 88), a 32 × 21 `long_button_frame_off`, `_on` while pending | `self_destruction_icon` (30 × 15) at (326, 90) | 6244 *Explode!* | nothing is pending (`+0x135` clear) | set `+0x135` and stamp the time (`0x10075fa0`); 0.6 s later the unit record's takt calls its object's slot 7 (`0x100756dc`, not followed) |

  A click hits a button's icon rectangle grown by 2 on each side. The levels are
  the auto-driver's ([35-hud.md](35-hud.md#the-indicators--read-and-seen),
  [39-boarding.md](39-boarding.md)); what mode 2, the driven unit, does from
  command mode is [40-command-mode.md](40-command-mode.md)'s.

*Seen* at 190 s, *SWB-2 Builder [building]*: the box at about (52, 20)–(368,
115); the icons at (60, 28) and (79, 28), red; the name from x 100 at y 30; the
lines from x 122, 8 apart, starting at y 38; A, B and C at 63, 84 and 108 and E
at 323–355, all lit.

### The rows

Row *i* stands at (51, 118 + 20*i*) (`0x100857fc`), drawn by `0x10095b80` with the
pen, left to right:
- `lamp_text_ending`, lit while the unit is selected (its `+0x80` reads 1);
- the unit's two icons, each in a small icon piece (`0x1009a7a0`), a `body_text`
  square as wide as the piece is high, 19, with the icon inset by 2; the pen moves
  19 for each;
- a separator and a `ray_emitter`, lit when selected;
- **a bar 248 wide** (`0x1009a380`) with `"%s [%s]"` centred — white when
  selected, `#808080` otherwise — over a fill of the unit's life percentage
  (`0x1007e980`, red under 20, olive under 80, green);
- `ray_ending`.

**A click on a row** (`0x100860a0`) selects that unit alone (`0x1007d0a0` with 1:
every other unit's `+0x80` cleared, `VOICE_SELECTED` if it was not selected),
places the order menu under the list and opens it for that one unit. *Seen* at
380 s: five rows *SSW-4* … *SSW-8 Warrior [patrolling]* with their tops at about
120, 140 … 200, only the last one white and lit, and the *Orders* strip under
them at about 218.

### The order menu

**Where.** At (100, 118 + 20*N*), *N* the list's rows (`0x100840a3`): right under
the last row. It shows at most (454 − *y*) ÷ 21 rows (`0x1007b1a0`); from the
first, since nothing moves its offset `+0x10` from the 0 its constructor
(`0x1007a4c0`) gives it.

**The strip** (`0x1007b1e0`, only for the commander's menu, not the wingman's):
from the pen at (100, *y*):
- `ending_text`;
- a text box 158 wide with 5084 *Orders*;
- two long buttons, `scroll_up_icon` then `scroll_down_icon` (30 × 15 each);
- a mirrored ending.

**The two arrows do nothing** (*read*, as a search): their rectangles are stored
(`+0x160`–`+0x18c`) and no code reads them, and the offset is never written again.
*Seen*: the menu never scrolls in the recording.

**The rows** stand 20 apart from *y* + 20, each 269 wide (`0x1007b0bf`), drawn by
`0x1009c830`: `ending_text`, a lit `ray_emitter`, a bar 247 wide with the row's
string centred in white, and `ray_ending`. **A pressed row fills its bar** for
1 s (`0x1009c89a`).

**A click on a row** (`0x1007b510`) presses it, plays `BUTTON_CLICK`
(`i_click1.wav`) and executes its command (`0x1007b740`): straight to the
dispatcher, or into a pick
([31-packages.md](31-packages.md#the-commanders-menus--measured-and-read),
[40-command-mode.md](40-command-mode.md)). The panel shows nothing more for a
pick: the rows stay as they are.

**It rebuilds itself every 2 s** (`0x1007b4a0`, then `0x1007aaa0`), unless a row
is still pressed, and at once whenever a unit is selected from the panel.

### Which rows it offers

**The rows** (`0x1007aaa0`, the commander's branch) come from the 22-row HQ table
(`0x10104f98`): for each selected unit and each row, a row whose mask holds the
unit's Type — `(mask & Type) == Type` — and that passes the row test below is
taken. **Every command appears once, in command order** (the list is sorted and
duplicates dropped, `0x1007aef9`–`0x1007af92`), whatever order the units come in.

**The row test** (`0x1007bbb0`, case table `0x1007bdec`):

| command | offered when |
|---|---|
| 0 Standby, 1 Route, 3 Seek and destroy, 6 Guard, 7 Refit, 8 Transport minerals | always |
| 2 Search and capture | the first selected unit's `+0x30` is 1 or 2 |
| 9 Search minerals | a lode of the map is not yet found (`0x10081b50`: the lode's found word is not 1) |
| 10–16 Build … | below |
| 17–23 Upgrade … | the first unit can build (`0x10076da0`) and the clan has a building of that Type that `0x10034230` accepts (below) |

**A build row** (Mine `0x80000004`, Warehouse `…08`, Factory `…10`, Outpost `…40`,
Res. Center `…400`, Light Tower `0x80100000`, Heavy Tower `0x80200000`) is
offered only when all of these hold:
1. the first selected unit can build: alive, with an intact beam
   (`0x10076da0`, [32-builder.md](32-builder.md#building-a-building--read));
2. **the menu has not marked the Type owned.** Each time the menu opens
   (`0x1007a8e0`) it looks at the seven Types (listed at `0x1007a902`) and marks
   one when the clan owns a building of it (`0x100729a0` on the level's building
   list, `+0x71c`); a mark, once made, stays (`0x1007a9a6`);
3. **the clan owns no building of that Type now** (`0x10034010` → `0x100729a0`);
4. **no builder of the clan is already on its way to build one**: none has order
   7 at the head of its list with that Type as its parameter (`0x10033ea0`);
5. **every part of the Type's first building is researched**: the research tree
   (`+0xae8`) gives the scheme's first `.dat`, and the parts still unresearched
   (`0x1008b130`) must be none.

So **a clan builds one building of each kind** from the menu. *Derived*: once a
builder has been sent to build a mine, the row goes, and once the mine stands, it
does not come back.

**What an Upgrade row asks of a building** (*read*). The row test walks the
clan's buildings of the row's Type (`0x1007bd4d`–`0x1007bd9a`): `0x10072a80`
gives each building record on the level's list (`+0x71c`) whose clan (`+0x24`)
is the player's (`+0xad0`) and whose Type (`+0x2c`) is the row's, in the list's
order. The row is offered at the first that `0x10034230` accepts, given the
record's logic id (`+0x34`) and a word for why it refused. The game's `+0xae8`
object, which condition 5 above takes the first `.dat` from, answers the logic
id with the building (its slot 21). `0x10034230` accepts the building when all
of these hold, and writes why at steps 1, 2, 6 and 7:

1. the `+0xae8` object is there, else 1;
2. **its Type is none of five** (`0x10034282`–`0x100342b5`), else 2: the
   generator `0x80000002`, the hangar `0x80000040`, the main teleport
   `0x80000200`, the bridge `0x80001000` and the ruin `0x80002000`;
3. its property `0x20c` reads 0 — no construction sphere running
   ([32-builder.md](32-builder.md#upgrading-a-building--read));
4. its owner word, `IGameObject` slot 17 (`0x100342d1`), is not `0xfffe`,
   destroyed;
5. **its level, property `0x209`, is below the count of its Type's scheme less
   one** (`0x100342e2`–`0x10034303`: the object's slot 59 gives the count);
6. the object's slot 58 names the scheme's `.dat` at level + 1 for the player's
   clan, else 1;
7. **every part of that `.dat` is researched** in the player's clan's tree
   (the clan record at `+0x724` + 0x68 × clan, its `+0x50`): the same
   unresearched-parts count as condition 5 of a Build row, `0x1008b130`, must
   be 0 (`0x100343b3`–`0x100343d6`), else 6.

So **an upgrade asks for the next building's parts as a build asks for the
first's**, which is what Mission 03 had been read to imply: its Small
Warehouse's next entry, the Medium Warehouse, has `fr_m_store` unresearched in
`tut3_pl.trf`, and no Upgrade Warehouse is offered (below). **Upgrade Outpost,
row 20, is never offered**: the hangar is refused at step 2 whatever its ladder,
and its scheme holds one building anyway
([32-builder.md](32-builder.md#what-gets-built)). The generator is refused too,
though its scheme has two entries, but no row names it.

**The click** (`0x1007bb14` → `0x10078b60` with command 32 and the Type,
dispatched per selected unit to `0x10078f60`) runs the same test over the
clan's buildings within the Type (`0x1007df50`) and sends each unit to **the
accepted building nearest it** across the ground (`0x10079078`–`0x100790b3`):
order 24, `ORDER_ROBOT_UPGRADE`, target `0x201` with that building's logic id.
A unit with no accepted building gets no order (`0x1007914a`). So two builders
selected together may go to two buildings.

**Mission 03** (*measured*, taking a part as researched when the item that
researches it is in the tree and researched, the constructor's rule of
[38-designs.md](38-designs.md)):
- the player's tree is `tut3_pl.trf`, and of the seven first buildings **only
  `smine01`'s six parts are all researched**; the Warehouse and the Factory are
  owned besides;
- the map's one lode starts found, so no Search minerals;
- so a builder, `tut3_b`, is offered Standby, Route, Search and capture, Seek and
  destroy, Guard, Refit and Build Mine — *seen* at about 181 s; *derived*, a
  transport, `tut3_t`, gets Transport minerals where the builder gets Build Mine,
  and Search and capture only if its own `+0x30` is 1 or 2;
- *seen* at 190 s, with the builder *[building]*, the same menu without Build
  Mine: condition 4.

**Mission 04** (*seen*, the recording of *Teleport*, 84–96 s, the Battle units
page):
- **The rows.** The HQ, *LWC-1 Comm. Center*, is offered Standby, Route, Seek
  and destroy, Guard and Refit. The helicopter, *TFW-2 Warrior*, is offered the
  same with Search and capture after Route: the row test's size class, 4 against
  1.
- **The unit box.** The HQ's box shows a fourth, ▶ button beside A, B and C,
  which the helicopter's does not.
- **The orders.** A click on the helicopter's Search and capture turns its line
  to *[searching]* at 86 s. The HQ's Standby turns its line to *[standing]* at
  89 s, and its Guard to *[patrolling]* at 96 s. What follows the search is
  [31-packages.md](31-packages.md#the-capture-tick-by-tick--read-measured-and-seen)'s.
- **The map.** The commander's map beside the page marks the other clans'
  buildings by their icons only while one of the player's units has them on
  radar ([35-hud.md](35-hud.md#the-panel-in-the-cockpit--read-and-seen)).

## The building pages, 5 to 8 — *read*, and *seen*

**The rows** (`0x10085340`, *mask*) are the clan's buildings within the mask
(`0x1007df50`), less any building itself (`0x100344c0`). Page 5 draws them under
the factory panel at (51, 171 + 20*i*), and only with the column
([36-factory.md](36-factory.md#what-is-drawn--read-and-seen)); pages 6–8 at
(51, 20 + 20*i*), with no box.

**A row** (`0x100961f0`), left to right from x 51:
- `lamp_text_ending`, lit while the building is selected;
- its icon (`0x100344e0`), 15 × 15 on `ui_menu`: storage (81, 110), mine
  (129, 126), generator (113, 126), plant (129, 94), Outpost (97, 126), institute
  (49, 126), any bunker (65, 126), either tower (81, 126); the main teleport,
  `0x80000200`, takes `ui_menu3` (148, 27). It is tinted by the record's `+0x30`: 1 white,
  2 `0xffff0000`, 3 `0xff00ff00`, 4 `0xff0000ff`;
- a separator;
- **for a bunker** (`0x80010000`, `…20000`, `…40000`): a 35-wide button with
  `buildscreen_direct_icon`, tooltip 1517 *Strategic control*;
- **for a bunker or a tower** (`0x80100000`, `0x80200000`): a 35-wide button with
  `buildscreen_hq_icon`, tooltip 1502 *Manual*, grey `#808080` and inert when the
  building's first class-1 component reads no life (`0x10033e40`);
- `ray_emitter`, and a bar from the pen to x 354 with the building's name centred,
  white when selected and grey otherwise, over its life;
- `ray_ending`.

**A click on a row** (`0x100860a0`, kind 0) selects the building (`0x1007db80`);
for a plant it hands the factory panel the building and turns to page 5. Then, by
the button under the mouse (`0x100965d0`):
- **Strategic control**, while the `CState` mode is 4 or 3: enter mode 4 with this
  bunker (`0x100862d6`) — command mode from that bunker;
- **Manual**: enter mode 6 with the building (`0x10086256`), unless its turret
  reads dead.

What modes 4 and 6 then show is [40-command-mode.md](40-command-mode.md)'s.

*Seen* at 385 s, *Bunker buildings*: one row at y about 21–38 — a lit lamp, a red
flag, a button whose icon the cursor covers, a palm, and *Small Bunker* on a green
bar ending at about 363.

## The research page, 4 — *read*, and *seen*

Page 4 draws **the research panel** (`+0x7b8`, `0x100898a0`) under its header,
titled 5080 *Research Center*, and turning to the page refreshes the panel
(`0x10089fd0`, from `0x10084dda`).
- **In command mode**, its button needs a research centre of the clan's (see
  above), so on Mission 03 it stays disabled.
- **From a pod.** Standing on the pod of one of the player's own research
  centres opens it too. The pod callback's same-clan branch pushes view mode 5
  with the building and turns the panel to page 4 (`0x10062756`), as it turns a
  factory's pod to page 5 ([36-factory.md](36-factory.md#how-the-screen-opens--read)).
  The column is left out, and the exit button or Esc pops the mode.

The research itself, what order 14 does at the centre, is
[16-research.md](16-research.md#researching--read)'s.

### The panel's parts

The constructor is `0x10089560`:

| field | what |
|---|---|
| `+0x04` | **the box**, 0x1d0 bytes (`0x10088980`): the preview (`+0`), the item it shows (`+4`), `long_button_frame_off` and `_on`, `batch_research_icon`, tooltip 6243 *Mark all items to research*, and a byte set for the frame after a click |
| `+0x1d4` | **50 rows** of 0x170 bytes each (`0x10088210`). A row holds its label (`+0x24`), its technology (`+0x30`), `resbutton_start` and `resbutton_stop`, and tooltips 6251 *Research item* and 6252 *Cancel research* |
| `+0x49b4`, `+0x49b8`, `+0x49bc` | the row count, **the selected technology**, the first row shown |
| `+0x49c0`, `+0x4a4c` | `scroll_down_icon`, `scroll_up_icon` |

**The sprites** are `ui/hq.cfg`'s:

| sprite | page | at | size |
|---|---|---|---|
| `resbutton_start` | `ui_menu` | (49, 126) | 15 × 15 |
| `resbutton_stop` | `ui_menu` | (238, 238) | 15 × 15 |
| `batch_research_icon` | `ui_menu3` | (177, 43) | 30 × 15 |
| `scroll_up_icon` | `ui_menu` | (145, 94) | 30 × 15 |
| `scroll_down_icon` | `ui_menu` | (176, 94) | 30 × 15 |

### Which rows — *read*

The refresh (`0x10089fd0`) walks every item of the player clan's tree in order.
It lists an item that is:
- in the tree;
- available;
- not researched;
- **priced**: a research energy or ore cost other than 0.

The rules:
- **At most 50 rows.**
- **The label** is `"%s (%s)"` of the item's name (slot 15) and short code
  (slot 13), or the name alone when the code is empty (`0x100887f0`).
- **A stale scroll resets.** If the first row shown is past the end, it becomes
  0.
- **The selection.** If the selected technology is not among the rows, the first
  row shown becomes selected.
- **Free items never show.** An item costing nothing is researched as soon as it
  opens ([16-research.md](16-research.md#completing--read)).

### What it draws — *read*

The pen primitives and pieces are
[35-hud.md](35-hud.md#everything-is-drawn-on-a-640--480-screen--read)'s.

**The box** (`0x10088c80`) is framed about (51, 20)–(369, 158) and filled
`0x80008000`. Its contents are clipped 5 inside.

- **A selected technology not yet researched** shows:
  - **its name** in `GAME_FONT` at (71, 30);
  - **the preview**: its part turning, 127 × 127 at (226, 18) (`0x10089050`
    makes it, `0x1009ee30` draws it);
  - **its descendants** (`0x100890e0`), when it unlocks anything:
    - 5079 *DESCENDANTS* at (71, 48) in `#c0c0ff`;
    - then up to four of the unlocked items as `"name (code)"`, at
      (86, 60 + *k* × line), where line is the font's height + 1;
    - `...` below the fourth when there are more.
    - The list is slot 10's unlocks, whether or not they are in the tree.
- **Otherwise** it shows 5078 *No item selected* at (71, 30).
- **The batch button.**
  - The frame `long_button_frame_off` sits at (170, 127)–(202, 148), and
    `_on` for the one frame after a click.
  - `batch_research_icon` sits at (172, 129)–(200, 146): white on that frame,
    `#80ff80` otherwise.
  - Its tooltip 6243 shows while the cursor is on the icon.

**The scroll row**, from the pen at (51, 159):

| x | piece | what |
|---|---|---|
| 51–56 | `ending_stub` | |
| 56–264 | `body_stub`, 208 wide (`0x1009b0e0`) | |
| 264–314 | long button, `scroll_up_icon` | variant 1 while the first row shown is above 0, else 0 |
| 314–364 | long button, `scroll_down_icon` | variant 1 while there are more than 12 rows and the first shown + 12 is short of their count |
| 364–369 | `ending_stub`, mirrored | |

**The rows.** At most 12 are shown, from the first shown; row *k* is at
(51, 179 + 20*k*) (`0x10089a09`). A row is drawn by `0x100885f0`:

| x | piece | what |
|---|---|---|
| 51–56 | `ending_stub` | |
| 56–91 | short button (`0x1009ab00`), variant 1 when selected | the icon 15 × 15 at (66, *y* + 2): `resbutton_stop` in `#ff8080` while the item is queued, else `resbutton_start` in white |
| 91–101 | `ray_emitter`, variant 1 when selected | |
| 101–363 | bar, 262 wide | the label centred, white when selected and `#c0c0c0` otherwise, over **a fill of the item's stored progress**: red under 20%, olive under 80%, green above |
| 363–369 | `ray_ending` | |

The button's tooltip is 6252 *Cancel research* while the item is queued, and
6251 *Research item* otherwise. **No row prints a cost or a percentage.**

**Queued** means an order 14 for that technology sits in any of the player's
research centres' order lists (`0x10087c00`, `0x10087d50`). The list of centres
(`0x1010c36c`) has two maintainers:
- the building record's setup files each of the player's own research centres
  in it (`0x10032d95`);
- the record's removal takes it out (`0x1007da7c`).

### What a click does — *read*

**The commander's click reaches the panel** on page 4 (`0x10084a90`). The
panel's area is x 51–369, from the top down to 177 + 20 × the row count
(`0x10089f90`, tested at `0x10084d45`). The panel's handler (`0x10089ce0`) then
takes, in order:

1. **The batch button**, when there are rows.
   - It flashes.
   - It **cancels every queued research at every centre** (`0x10087f10`: each
     order 14 is aborted through the centre's `IAgent` slot 40).
   - It **orders every row's technology**, in row order.
   - If nothing is selected, it selects the first row shown.
2. **Scroll up.** The first row shown goes down by 1, while it is above 0.
3. **Scroll down.** It goes up by 1, while the first shown + 12 is short of the
   count.
4. **A row.**
   - **On its button:** a queued technology's order is aborted, and one not
     queued is ordered.
   - **Anywhere on the row**, the row's technology becomes the selected one, and
     the preview is made again.

**Ordering** (`0x100880b0`, [31-packages.md](31-packages.md#the-orders--measured)):
- It skips a technology already researched or already queued.
- **Which centre.** The one that has not been destroyed (class word not
  `0xfffe`), whose property `0x20c` reads 0 (its construction sphere is not
  running), and which has the fewest orders.
- **The order.** 14, target `0x204`, the id as the parameter, to the end of the
  queue.
- **With no such centre**, nothing is ordered.

### When research completes — *read*

**The check runs every pass of the game loop** (`0x1005ea7a` → `0x10089a50`).
It walks every queued order 14 at every centre. For each whose technology now
reads researched, it:
1. **aborts that order**, so the research is reported once;
2. **posts a system message** (`0x1007eb60`): `"%s (%s)"` of 2010 *Research
   complete...* and the item's name;
3. **plays `VOICE_RSRCH_COMPLETE`**.

Afterwards:
- **The rows.** If any was reported, the panel is refreshed.
- **The box.** The refresh has already moved the selection to the first row
  shown, if any is left. If the box's technology still reads researched, the
  preview is dropped and the box reads *No item selected*. On Mission 04, with its
  one row gone, that is what the box shows (*derived*).

**A research that finishes after its order is gone is never reported.** The task
keeps its order through the takt after the one that completes
([16-research.md](16-research.md#the-takt)), so the check sees it as long as it
runs between two of the centre's takts (*derived*; how often a building's takt
runs was not read).

### Against the recording — *seen*

Mission 04, on the Enhanced Research Center's pod
([16-research.md](16-research.md#mission-04s-tree--measured-and-seen)), at
960 × 720 halved to 640 × 480:
- **The header and box.** The header *Research Center* spans 51–369. The box
  is framed from (53, 20) to (367, 157).
- **The name.** *Large Battle Turret* sits at about (75, 38): the text is drawn
  from its top at 30.
- **The descendants.** *DESCENDANTS* at about (75, 57), and its one descendant
  at about (91, 69), both light blue.
- **The preview** turns about (290, 92).
- **The batch button** at about (177–207, 133–150).
- **The scroll row** at y 160–177, the arrows at about 273–307 and 320–353.
- **The one row** at y 181–197: its button at 57–90, the label centred over
  101–363.
- **After the batch click**, the stop cross is pinkish-red and the fill runs from
  the bar's left.
- **The message.** 2010 reads *"Research complete... (Large Battle Turret)"*,
  from *System*.

## For an engine

1. **Draw** in command mode (modes 3 and 4), after the world and before the map,
   the message box and the objectives: the Ore and Energy rows, the column, and
   the page while the column is out. While a building's model follows the mouse,
   draw the rows alone.
2. **The column** at x 0, as tabulated: the head at 0, buttons 46 × 29 with their
   icons at (+16, +3), separators 46 × 24 at 41, 152, 205 and 345, the lock at
   457. Grey `#646464` on `objpanel_button1` when disabled, `#80ff80` on `2` when
   enabled, white on `3` when on; a hovered enabled button flips between 2 and 3
   every 0.1 s. Tooltips as tabulated.
3. **Enable and light** each button every frame by the table: units within
   `0x1018000`, `0x1002000`, `0x1004000`; buildings not building themselves of
   `0x80000400`, `0x80000010`, within `0x80300000`, `0x80070000`; any building
   within `0x8000044e`; chat only in a network game; map and game menu always.
4. **The lock**: resting the mouse on it while shut slides the column out;
   clicking it while out slides it in. One item a 0.05 s step, `BAR_OPEN` as it
   starts; the lock hidden while sliding, at y 12 when shut; no page while not
   fully out.
5. **Clicks**, in order: lock; the map's close; hero (back to the hero's view);
   page buttons (toggle the page); chat; map; game menu (mode 7); the header's
   exit (page 0); then the page. Swallow clicks on the resource rows; pass the
   rest to the world.
6. **Pages**: 1 Battle units `0x1018000`, 2 Transports `0x1002000`, 3 Builders
   `0x1004000`, 4 research, 5 Factory, 6 towers, 7 bunkers, 8 other buildings.
   Turning to a page keeps a single selected unit of its kind or selects the
   clan's first; a unit page with no units closes; a building page whose button
   is disabled closes.
7. **A unit page**: the box (51, 20)–(369, 117); for one selected unit its icons,
   yellow `"name [status]"` at (101, 30), the unit lines at (122, 27), the A, B, C
   (D for an HQ) and E buttons as tabulated; the rows at (51, 118 + 20*i*), a click
   selecting that unit alone with `VOICE_SELECTED`.
8. **The order menu** at (100, 118 + 20*N*): the *Orders* strip with two inert
   arrows, rows 269 × 20 from 20 below it, as many as fit above 454; each command
   once, in command order, as the row test allows; a click presses the row for
   1 s, plays `BUTTON_CLICK` and executes. Rebuild every 2 s.
9. **Build rows**: offer one only to a living builder with its beam, while the
   clan owns no building of the Type (and has owned none since the menu last
   opened), no builder of the clan is heading off to build one, and the scheme's
   first building's parts are all researched. On Mission 03 that is the mine
   alone. **Upgrade rows**: offer one to a living builder with its beam while
   the clan has a building of the Type, not a generator, hangar, main teleport,
   bridge or ruin, not in its construction sphere and alive, with an entry
   above its own in its scheme whose parts are all researched. A click sends
   each selected unit to the nearest such building, and a unit with none stays.
10. **Building rows** on pages 5–8: lamp, icon, the Strategic control button for a
    bunker, the Manual button for a bunker or tower, the name over its life;
    Strategic control moves command mode to that bunker, Manual takes its turret.
11. **The research page, 4.** It also opens in view mode 5 from a player's
    research centre's pod.
    - **Rows.** The tree's items in the tree, available, unresearched and
      priced, at most 50. Each is labelled `"name (code)"`, 12 shown at
      (51, 179 + 20*k*), with a start or stop button and a bar filled by the
      item's progress.
    - **The box** (51, 20)–(369, 158): the selected item's name, its
      descendants and its preview; the batch button at (170, 127).
    - **The scroll row** at y 159.
    - **Clicks.** A row's button orders or cancels. The batch button cancels
      everything, then orders every row. Order 14 goes to the end of the living,
      unsphered centre with the fewest orders.
    - **Every frame**, report each queued item now researched: abort its order,
      post *"Research complete... (name)"* and play `VOICE_RSRCH_COMPLETE`.
12. **The commander's map** (374, 63)–(640, 329) marks buildings with their
    `icons` cells, 20 × 20 about their place.
    - **Colour.** The clan rule's colour, white for a selected building of
      the player's.
    - **Which.** The player's own, and others' only while one of the player's
      units has them among its radar contacts.
    - **The rest.** Units as crosses and squares, and the camera
      ([35-hud.md](35-hud.md#the-panel-in-the-cockpit--read-and-seen)).

## Not established

- What the satellite map's `+0x230` rectangle is, which the column's click tests
  before the buttons.
- The record words the icons are tinted and picked by: `+0x30` (1 to 4 for a unit,
  1 to 5 for a building) and a unit's property `0x207`. The stand-in reading of
  `+0x30` as the chassis's size class is [31-packages.md](31-packages.md#not-established)'s;
  the recording's player units and bunker all read red, which is 2.
- What slot 7 of a unit's object does 0.6 s after *Explode!*.
- ~~What `0x10034230` accepts for an upgrade row, and what `0x80000200`, the Type
  given its own building icon, is.~~ — **read**: a building of the clan, not of
  five refused Types, not in its sphere, alive, below the top of its scheme,
  whose next `.dat` is researched whole (`0x10034282`–`0x100343d6`), in
  [Which rows it offers](#which-rows-it-offers); the research requirement is
  read, no longer derived from Mission 03. `0x80000200` is
  `BUILDING_MAINTELEPORT` (`MISSIONS/SCRIPTS/varset.var`, line 169), the main
  teleport, which the test refuses outright.
- ~~The research panel's contents and controls~~ — **read**, above.
- **The research panel's text colours.** The box's name is drawn in `GAME_FONT`'s
  current colour, set by whatever drew before it. It reads white in the
  recording.
- ~~What the wingman menu's rows look like beside the commander's.~~ Answered:
  with a number, the row draw (`0x1009c8e6`) puts `ending_text` and a 12-wide
  number box before a 150-wide bar
  ([31-packages.md](31-packages.md#the-wingman-menu-from-first-person--read-and-measured)).
  Without one, it starts with `ending_stub` (variant 0, `0x1009c8b2`), not
  `ending_text`. The strip's two ends are `ending_stub` as well (`0x1007b276`,
  `0x1007b32d`).
- Whether a click on a disabled order row can happen at all: the commander's
  rows are only the offered ones, and the click tests no row flag.
