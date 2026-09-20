# The warbot designer — the screen

The factory's warbot constructor: a source panel of parts on the left, the
project in the middle, the project's own slots on the right, and a row of
buttons under the project. This page reads the screen: how it opens and closes,
where every piece is drawn, how a click moves a part, and how the models turn.
What the parts are, which are offered, the numbers the boxes show and what
accept writes are [38-designs.md](38-designs.md)'s. The page builder that fills
a tab's list by name prefix is [28-chassis.md](28-chassis.md#what-the-label-is-not--read-as-a-search)'s.

**Every claim is tagged**, as in [15-behaviour.md](15-behaviour.md):
- *measured* is re-derived by `openparkan verify` (`check_designer_screen`);
- *read* comes from the disassembly of `iron3d.dll` at the address given;
- *seen* is the recording of *The Constructor*, 960 × 720, where the designer is
  open from 107.67 s to 155.5 s;
- *guess* fits and is not established.

The drawing conventions are [35-hud.md](35-hud.md#everything-is-drawn-on-a-640--480-screen--read)'s:
every coordinate is on a 640 × 480 layout and scaled by the display's two
scales; a sprite is a cut of a `ui/ui.lib` page named in
`ui/game_resources.cfg` (`icons` is `icons.tex`, `page1` `ui_tex1.tex`, `page3`
`ui_tex3.tex`, `page4` `ui_tex4.tex`, `page5` `ui_tex5.tex`, `page7`
`ui_tex7.tex`); a cut is written (page x, y, w, h); a sprite quad is drawn with
diffuse white and specular black in blend mode 1 unless a colour is given;
lines, rectangles and text come from the GUI server's slots. Text is in
`GAME_FONT` (the game's `+0x10`) unless said otherwise.

## Opening and closing — *read*

**The designer is one object of the interface's screens**, their `+0x08`
([35-hud.md](35-hud.md#who-draws-it-and-what-it-hides--read)): a holder of 0x30
bytes (`0x10055910`, vtable `0x100e5f98`) around the designer proper, 0xbca4
bytes (`0x1004d300`, at the holder's `+0x08`). The holder keeps:

| field | what |
|---|---|
| `+0x04` | the opener's first argument, the building record's `+0x3c` |
| `+0x0c`–`+0x14` | the three scan bands ([below](#the-scan-bands--read-and-seen)) |
| `+0x18` | the opener's fifth argument, the record's `+0x30` |
| `+0x28` | the kind: 1 robots (prefix `r_`), 2 bases (`fr_`), 3 `a_` (`0x10051410`) |
| `+0x2c` | open |

**The factory opens it.** The factory screen's click handler
(`0x10098040`) tests its constructor button's rectangle (its `+0x818`–`+0x828`)
and calls the holder's open (`0x10055c70`, at `0x1009810a`) with the record,
kind 1, the string `"r"`, 0 and the record's `+0x30`. The factory screen itself
is [36-factory.md](36-factory.md)'s.

**Opening** (`0x10055c70`) does nothing while a kind is set. Otherwise it stores
the arguments, sets `+0x2c`, writes 9 to the driven unit's flags (its slot 13,
`+0x34`), shows the mouse cursor, and lays the designer out with the string
(`0x1004dff0`). The first letter of
that string, kept at the designer's `+0xb668`, is what the screen later tests:
`r` is the robot designer.

**It is drawn instead of the mode's own screen.** The screens' draw
(`0x1008d200`) switches on the `CState` mode. In modes 3 and 4 (`0x1008d51c`)
and 5 (`0x1008d444`) it shows the cursor and, while the holder's `+0x2c` is
set, draws the designer (`0x10055dc0`) and nothing else; otherwise the mode's
screen. The factory's screen is mode 5 ([27-ownership.md](27-ownership.md#capture--read)).

**Three things close it**, each clearing bit `0x8` of the unit's flags (read at
slot 20, `+0x50`, written at slot 13), deleting the
previews and the project, and zeroing the holder's `+0x2c` and `+0x28`, so the
mode's own screen comes back:
- **the exit button** (`0x10051436`); it also clears both panels (`0x1004a480`);
- **Esc** (the holder's key handler, `0x10055e80`), when neither the load list
  nor the name field is up; with one of them up, Esc closes that instead;
- **accept** (`0x1005144c`), below.

**Input** reaches the holder while it is open. Its mouse handler
(`0x10055ff0`) and its Esc act only while the game object's `+0x08` is 4 (not
read). A click goes to the designer's buttons first (`0x100504d0`), then the
source panel (`0x10049b80`), then the destination panel; the first to take it
ends the search. Its character handler (`0x10055fa0`) feeds the name field.

## The screen, piece by piece — *read*, and *seen*

**The draw** (`0x10055dc0`), in order:
1. a black `0xff000000` rectangle over (0, 0)–(640, 480): nothing of the world
   shows;
2. the source panel's frame and part box, then the destination's
   (`0x10049ef0`);
3. the source panel's tabs and list, then the destination's (vtable slot 2,
   `0x1004bb00`);
4. the project's column (`0x1004ec50`);
5. the three scan bands (`0x1009f750`).

Three columns share the screen: the **source panel** on x 0–183, the **project**
on 185–455 and the **destination panel** on 457–640.

### A panel — *read*

Both panels are one class (`0x10049820`, 0x4e4fc bytes), the source built by
`0x1004ac40` (vtable `0x100e5f4c`) and the destination by `0x1004bf70` (vtable
`0x100e5f60`). The destination is the source moved right by 457; write *X*₀
for 0 or 457 and *X*₁ = *X*₀ + 183.

| piece | where | art | notes |
|---|---|---|---|
| list ground | (*X*₀ + 11, 61)–(*X*₁ − 11, 228) | a filled rectangle `0x3c329632`, alpha kept | a faint green |
| header | (*X*₀, 0)–(*X*₁, 61) | page4 (0, 44, 183, 61) | the title bar and the tab well |
| separator, left | (*X*₀, 228)–(*X*₀ + 14, 244) | page4 (121, 144, 14, 16) | |
| separator, tiles | from *X*₀ + 14 by 13 to *X*₁ − 27, y 229–243 | page4 (136, 145, 12, 14) | the last one ends at *X*₁ − 14 |
| separator, right | (*X*₁ − 14, 228)–(*X*₁, 244) | page4 (150, 144, 14, 16) | |
| preview ground | (*X*₀ + 11, 244)–(*X*₁ − 11, 396) | filled `0x3c329632`, then page7 (128, 128, 128, 128) in `0xff006400` | the radial grid |
| part box, left | (*X*₀, 396)–(*X*₀ + 20, 480) | page4 (0, 144, 20, 84) | |
| part box, tiles | from *X*₀ + 20 by 32 to *X*₁ − 52, y 397–480 | page4 (21, 145, 31, 83) | the last one ends at *X*₁ − 20 |
| part box, right | (*X*₁ − 20, 396)–(*X*₁, 480) | page4 (54, 144, 20, 84) | |
| circuit | four tiles over the preview ground, two by two | page3 (172, 140, 64, 64) | colour `0xff00gg00`, gg = 150 + 200 × the seconds since a 0.25 s timer restarted: it pulses |

**The title** is 3049 *Source panel* or 3050 *Destination panel* at (*X*₀ + 10,
6) in yellow `0xffffff00` (`0x10047d20`). When the selected tab lists nothing it
is 1115 *NO ITEMS AVAILABLE* in `0xff009600`, at x 10 in either panel.

**The part box** (`0x10049ef0`) shows the selected row's part: its stat rows
from the research tree's templates ([19-descriptions.md](19-descriptions.md#what-the-game-takes-from-this-file--read),
[38-designs.md](38-designs.md)) from (*X*₀ + 15, 415), 135 wide (`0x1006f890`);
with no part, 5081 *No data available* in red `0xffff0000` at (*X*₀ + 15, 415).

*Seen* at 109.5 s: *Weight* at x 14.7 and y 418 in the source's part box, and
*No data available* in red in the destination's until a row there is picked.

### The tabs — *read*, and *seen*

Six tabs sit in each header, three on the left and three on the right. Each is
a control with a frame, an icon, a tooltip and a list of up to 64 rows. The
source's, by position (*X*₀ = 0; the destination's add 457):

| tab | tooltip | frame | icon, 24 × 24 | icon cut | icon colours: off, normal, selected |
|---|---|---|---|---|---|
| Chassis | 6216 *Chassis* | (8, 16)–(35, 53) | (9, 25) | icons (48, 72) | `0xff404064`, `0xff9696e6`, `0xffdcdcff` |
| Turrets | 6218 *Turrets* | (35, 16)–(61, 53) | (36, 25) | icons (72, 72) | the same |
| Weapons | 6219 *Weapons* | (61, 16)–(87, 53) | (62, 25) | icons (96, 72) | the same |
| Armour | 6217 *Armour (optional)* | (101, 16)–(128, 53) | (102, 25), 26 in the destination | icons (193, 97) | `0xff404b4b`, `0xff78c8c8`, `0xffd2e6e6` |
| Internal systems | 6221 *Internal systems (optional)* | (128, 16)–(154, 53) | (129, 25) | icons (120, 72) | the same as Armour |
| Ammo | 6220 *Ammo (optional)* | (154, 16)–(180, 53) | (155, 25) | icons (206, 192) | the same as Armour |

- **The frame** (`0x10047af0`) is one of page4 (0, 106, 26, 37), (27, 106, 26,
  37) and (54, 106, 26, 37), stretched over the frame rectangle. The first has a
  dark lamp at its top, the other two a green one (*measured*: each holds art).
  A disabled tab draws the first, a selected one the second, and one under the
  cursor alternates the second and third every 0.1 s; any other the first.
- **The icon** (`0x10047c10`) is drawn in the tab's off colour when disabled,
  its selected colour when selected, its normal colour otherwise. Under the
  cursor its specular steps through grey 20, 40, 60, 80, 100 a frame, and its
  tooltip is armed.
- **A click** on the icon's 24 × 24 square (not the frame) selects an enabled
  tab (`0x10035920`, `0x10035700`, which plays `BUTTON_CLICK` as a control turns
  on); the panel then turns every other selected tab off (`0x10049e50`). On the
  next takt the other panel selects the same tab if it has not
  (`0x100506ef`–`0x1005093e`), so **the two panels always show the same tab**.
- **Which tabs are enabled.** When a panel is built only Chassis is: the other
  five are cleared (`0x1004b022` and after). Fitting a chassis enables Turrets
  and Internal systems in both panels, and Armour when a condition holds that
  was not read (`0x10051f4f`, `0x10052478`, `0x100524ae`). Fitting a turret
  enables Weapons and Internal systems (`0x10052929`, `0x10052c24`). Fitting a
  weapon enables Ammo (`0x10053407`).

*Seen*: at 107.67 s only the source's walker icon is bright and its lamp lit;
the others are dim. Once the chassis is in (108.33 s), both panels show their
Turrets tab.

### The rows — *read*, and *seen*

A selected tab draws its rows (`0x10047d20`): at most 6, from its first shown
row (`+0xce90`).

- **Row k** covers (*X*₀, 65 + 23k)–(*X*₀ + 180, 86 + 23k).
- **Its bar** is page4 (0, 22, 180, 21) over the whole row (`0x10046b10`).
- **Its lamp** covers (*X*₀, 65 + 23k)–(*X*₀ + 17, 86 + 23k) in `0xffc8ffc8`:
  page1 (178, 141, 17, 21) dark, (195, 141, 17, 21) bright green, (117, 80, 17,
  21) green. A selected row lights the second; a row under the cursor alternates
  the second and third every 0.1 s; any other shows the first.
- **Its text**, the part's *name (code)*, is at (*X*₀ + 10, 72 + 23k), drawn with
  colour 0 after the font's slot 2 (`+0x08`) is given a set of colours:
  `0xffc8c8c8` for a selected row, `0xff323296` and `0xff9696fa` for the rest;
  how the font uses them was not read. *Seen*: every row's text is light grey.
- **The scroll bar**, when the tab holds more than 6 rows (`0x10047660`,
  built by `0x10047210`):
  - a lamp at (*X*₀, 203)–(*X*₀ + 17, 224) from page1's three lamps as a row's;
  - a bar page4 (0, 0, 130, 21) at (*X*₀ + 5, 203)–(*X*₀ + 147, 224);
  - up, frame (*X*₀ + 147, 203)–(*X*₀ + 162, 224) from page1 (216, 78), (216,
    99), (216, 120), 15 × 21, and an icons (154, 0, 11, 11) arrow at (*X*₀ +
    148, 207), 13 × 13;
  - down, frame (*X*₀ + 162, 203)–(*X*₀ + 180, 224) from page1 (231, 78), (231,
    99), (231, 120), 18 × 21, and icons (154, 11, 11, 11) at (*X*₀ + 162, 206);
  - each button's icon drawn in `0xff9b9bff` as the buttons below are. A click
    on up moves the first shown row back by one, on down forward by one while
    rows are left (`0x1004bc19`).

*Seen*: at 109.5 s the destination's first lamp is centred on y 75; at 116.5 s it
shows six rows, and the scroll bar at y 205–221 with its arrows at x 603–630.

**A click on a row** (`0x10047f10`): a row the cursor is on, or the first shown
row when the click fell in the panel's preview, (*X*₀ + 8, 244)–(*X*₁ − 8, 396)
(`0x10049cd6`, which also plays `BUTTON_CLICK` through the sound server), is
taken.
- **An unselected row** is selected: every other row of the tab is turned off.
  Its name, its side (`+0xc0`: 0 source, 1 destination) and its item go to the
  interface's picked record (`0x1010a1b0`), stamped with the time.
- **A selected row clicked again within 0.2 s** sets the picked record's
  `+0xb8`: **a double click**. The designer's takt acts on it (next section).

**The picked part's model** appears in the panel's preview. When a row turns on,
the panel's takt (`0x1004bb90` for the source) makes a model view of the part
(`0x1009df90`) on (*X*₀ + 12, 244) sized 160 × 151, carrying the old model's
angle on (`0x1004bcca`).

### A row's record — *read*, and *measured*

The rows a destination tab lists are kept in the designer's own memory as
**arrays of 64 records, `0x330` bytes each**, and there are four of them, at
`+0xd220`, `+0x1a0c4`, `+0x26f68` and `+0x33e0c` — `0xcea4` apart, so each
array is one tab's. Which tab each one is was not read.

**The reset** (`0x1004b615`) walks 64 records writing `+0x00 = −1`, `+0x04 =
−1`, an empty string at `+0x08` and at `+0x14`, `+0x20 = −1` and `+0x24 = −1`,
and zeroes the list's count (`+0x33ab8` for the `+0x26f68` array, `+0x26c14`
for the `+0x1a0c4` one).

**The unit writer reads three of the fields** (`0x100546e2`–`0x1005479e`). It
skips a record whose `+0x20` is −1 or whose `+0x00` is non-zero, and otherwise
emits one component of the `.dat` ([38-designs.md](38-designs.md#the-files--read-and-measured)),
stride `0x78`: the library `objects.rlb`, the part id, flags 1, **`+0x04` as
the attach node**, the string at `+0x08` as the label, and class 3. The part id
comes from `0x1008a480` over `+0x20`, a wrapper on `IResearch` slot 3 that
returns the out-record's `+0x1c`, which `MisLoad.dll:0x10002aa0` fills from
`TRFB[item.part_index]` — so **`+0x20` is a research item index the writer
turns into a part id**.

**`+0x04` is the node's own index in the host part's mesh**, and the fits' loop
writes it (*read*, *measured*). Adding a part opens a row for each of its sockets
(`0x10052809`–`0x1005290c` in the turret add; the weapon add and the rest repeat it):

- the catalogue interface at the designer's `+0xbc94` is asked how many nodes the
  part has (slot 15, `0x10052809`), and the loop counter `ebx` runs **1 up to that
  count** (`0x1005281a`, `0x10052906`–`0x1005290c`);
- for each it asks slot 6 for that node's socket label (`0x10052832`) and **passes
  over a node that answers none** (`0x10052839`), so a node with no stream-10 label
  opens no row and node 0, the part's own root, never does;
- what it keeps is the counter itself: `0x10052878` writes `ebx` into the record it
  is building, at the offset the list insert (`0x10049410`) copies to the row's
  `+0x04` — `+0xd4` of the 0x330-byte record, which is `+0x04` in this page's
  numbering. The label goes to `+0x08`, and the row is appended by `0x10049410`,
  which copies eight fields from a prototype record on the caller's stack.

So the row's node field is the **node's index**, not an ordinal over the sockets.
*Measured* over the 33 `.dat` files the designer itself writes (`bld_unit_*`,
`view_unit_*`, `temp_unit`): all **138** external attachments are node 1 or above,
all 138 name a node that carries a stream-10 label, and all 138 rise in node order
under their host — and only **60 of the 138** would also fit an ordinal over the
host's sockets, the values being real node indices, 1 and 4 to 12, where the
ordinals would be 1 to 6. Over all 458 shipped assemblies, 1414 of 1414 land on a
`Base_*` node ([07-objects.md](07-objects.md#how-parts-attach)).

**`+0x00` and `+0x24` are the same handle seen from the two ends** (*read*, in
part). The part add fills a three-dword request — the selected row's `+0x00`, its
`+0x04`, and −1 — and hands it to the project (`+0xbc90`'s `+0x74`, slot 13, query
6 with `0x80000020`, `0x100526c3`–`0x100526f7`), which fills the third word in
place. That word is then:

- written into the row the part went into, at **`+0x24`** (`0x10052736` for a
  turret, `0x10052ea8` and `0x100530cb` for the other kinds), beside the research
  item that goes to `+0x20`;
- handed to the catalogue as the part whose nodes to count (`0x100527f8`), and
  written into **`+0x00`** of every socket row the loop then opens
  (`0x1005284b`–`0x10052855`).

So `+0x24` names the part the row now holds and `+0x00` the part whose socket the
row is; what the handle itself is — the project's own index for a fitted part — was
not read. It is what the unit writer tests: it emits a row only where `+0x20` is not
−1 **and `+0x00` is zero** (`0x100546e2`–`0x100546f9`), and then writes `+0x04` as
the component's attachment (`0x1005476e`) and `+0x08` as its label (`0x10054785`).

### Moving a part — *read*, and *seen*

The designer's takt (`0x100506d0`) reads the picked record once its `+0xb8` is
set, clears it, and by the tab:

| picked from | tab | what happens |
|---|---|---|
| the source | Chassis | the part is added (`0x100519e0`) if the project has no chassis yet (the destination's chassis row holds −1) |
| the source | Turrets, Weapons | added if the destination's selected row is an empty slot (−1) |
| the source | Internal systems, Ammo, Armour | added |
| the destination | Chassis, Turrets, Weapons | removed (`0x10053a50`) if the row holds a part |
| the destination | Internal systems, Ammo, Armour | nothing |

`0x100519e0` builds the project from a chassis (`0x10051bb0`), and otherwise
adds by the part's kind (`0x10052570` a turret, `0x10052fb0` a weapon, and the
others through its table at `0x10051b88`). What each accepts is
[38-designs.md](38-designs.md)'s.

*Seen*: from 108.0 s the cursor sits on the source's chassis preview, and at
108.33 s the chassis is in the project and both panels are on Turrets: the
double click on the preview.

### The project — *read*, and *seen*

Two rectangles make the middle column: the **view** *V* = (185, 0)–(455, 300)
and the **box** *B* = (185, 300)–(455, 480), both set when the designer is
first laid out (`0x1004e39f`–`0x1004e3e0`).

**The view** (`0x1004ec50`):
- a filled rectangle `0x3c329632` over (195, 10)–(445, 301);
- the large radial grid page7 (0, 128, 128, 128) in `0xff009b00`, four times,
  one quarter in each quadrant of that rectangle, mirrored so that they meet at
  its centre (320, 155);
- a top edge: page4 (131, 0, 14, 11) at (185, 0)–(199, 11), page4 (146, 0, 8,
  10) tiled 8 wide at y 0–10 from 199, and page4 (155, 0, 14, 11) at (441,
  0)–(455, 11);
- the project's model, a model view on (195, 10) sized 250 × 290
  (`0x10051c85`);
- the circuit page3 (172, 140, 64, 64) tiled four across and three down over the
  filled rectangle, pulsing as the panels' does (`0x10050185`).

**The prompt** is centred on x 320 at y 15 in green `0xff00ff00`:
- with no project, 1107 *SELECT CHASSIS* for the robot designer or 1114 *SELECT
  BASE CONSTRUCTION* otherwise;
- with a project, 3054 *SELECT TURRET* while the destination's Turrets tab is
  selected, 3055 *SELECT WEAPON* while Weapons is, 6242 *TUNE UP OR ACCEPT TO
  PRODUCTION* while Armour, Internal systems or Ammo is; nothing on Chassis.

With no project and no load list up, **the hint** is 3044 *Use the mouse to
click on the item*, 3045 *in the source panel* and 3048 *to add the item to the
project...*, green, centred on x 325 from y 341, a font height plus one apart.

*Seen*: the prompt's top at 13.5; at 107.67 s the three hint lines at y 345,
352 and 359.

**The box** (`0x1004ec50`):
- a filled rectangle `0x3c329632` over (202, 310)–(438, 440);
- page4 (74, 144, 19, 16) at (185, 300)–(204, 316); page4 (96, 145, 8, 13)
  tiled 8 wide at y 301–314 from 204; page4 (105, 144, 15, 15) at (440,
  300)–(455, 315);
- page4 (234, 137, 9, 119) at (185, 315)–(194, 437), the left edge;
- along the bottom, page4 (75, 169, 8, 43) at (185, 437)–(194, 480), (154, 169,
  5, 43) at (263, 437)–(269, 480), (75, 213, 67, 43) at (338, 437)–(405, 480)
  and (182, 213, 9, 43) at (446, 437)–(455, 480);
- while neither the name field nor the load list is up, the black inner panel:
  page3 (41, 140, 32, 115) at (195, 320)–(227, 435), page3 (74, 140, 64, 115)
  tiled 64 wide from 227 with the last one ending at 440, and page3 (139, 140,
  32, 115) at (423, 320)–(455, 435).

**The project's numbers** (`0x1006fc00`, at (220, 330)), with a project and
neither field up:
- the design's name at (220, 331) in yellow `0xffffff00`;
- rows from a font height plus 3 below it, each a font height plus 2 apart:
  5069 *Weight:*, 5070 *Max. speed:*, 5071 *Defence:*, 5072 *Offence:*, 5073
  *Sensor range:*, in green at x 220;
- each value right-aligned to x 352 in `0xffb4b4ff`, or red `0xffff0000`; its unit
  (6206 *t*, 6176 *kph*, …) in green at x 355.
- What the values are, and when one is red, are [38-designs.md](38-designs.md)'s.
  The routine's answer enables accept (below).

*Seen* at 109.5 s: *LFW-X Warrior* in yellow with its top at 331; *Weight:* at x
219.3; the value *37 / 28* ending at x 353; rows 8 apart.

**The callout** (`0x1004f3ed`–`0x1004f8b0`): while the destination's Turrets tab
has a row selected, or Weapons or Ammo has one, the socket that row names is
projected through the view's camera to (*p*ₓ, *p*ᵧ), and in green `0xff00ff00`:
- a line from (457, *r*) to (427, *r*), *r* being the row's top plus 10;
- a line from (427, *r*) to (427, *p*ᵧ);
- a line from (*p*ₓ + 16, *p*ᵧ) to (427, *p*ᵧ);
- a square (*p*ₓ − 15, *p*ᵧ − 15)–(*p*ₓ + 15, *p*ᵧ + 15).

*Seen* at 109.5 s: the vertical at x 427.3, the upper horizontal at y 75.3
ending at x 445, at the view's filled edge: the lines are clipped to the view.

### The buttons — *read*, and *seen*

Five buttons stand on the box's bottom edge, y 437–480 (laid out by
`0x1004dff0`). Each is a frame of three states and a 24 × 24 icon; a click
must land on the icon.

| button | frame | frame art | icon | icon cut | tooltip | enabled |
|---|---|---|---|---|---|---|
| accept | (194, 437)–(229, 480) | page4 (84, 169), (159, 169), (184, 44), 35 × 43 | (201, 450) | page1 (176, 231) ✓ | 6222 *Accept to production* | once a turret is fitted (`0x100525a4`) and the numbers' routine answers yes |
| clear | (229, 437)–(264, 480) | page4 (119, 169), (194, 169), (219, 44), 35 × 43 | (231, 450) | page1 (200, 231) ✗ | 1554 *Clear project* | always |
| save | (269, 437)–(304, 480) | as accept's | (278, 450) | page1 (152, 231) | 1555 *Save project to file* | while there is a project |
| load | (304, 437)–(339, 480) | as clear's | (308, 450) | page1 (128, 231) | 1556 *Load project from file* | always |
| exit | (405, 437)–(446, 480) | page4 (141, 213), (192, 213), (195, 88), 41 × 43 | (413, 450) | icons (192, 72) | 1557 *Leave bot constructor* | always |

- **Drawing** (`0x10045aa0`): the frame as a tab's, then the icon in `0xff9b9bff`,
  at half that colour (`0xff4d4d7f`) when disabled (`0x10035a60`).
- **A click** turns the button on; the takt sees it a frame later, turns it off
  and acts (`0x1005107c`–`0x100514e7`).

*Seen*: at 107.67 s accept and save are dim and clear, load and exit bright;
at 155.5 s the cursor on accept shows *Accept to production* in black on a pale
yellow box beside it, and the accept frame's lamp is green.

**What each does:**
- **Accept** stores the design's Type in the designer's `+0x04`. For the robot
  designer (kind 1) it writes `\units\temp_unit.dat` under the game directory
  (`0x100544b0` with flag 1, [38-designs.md](38-designs.md)) and, if that
  worked, tells the factory screen (the screens' `+0x18` `+0x7bc`,
  `0x100982a0` with 0; [36-factory.md](36-factory.md)); if not, the System says
  2111 *Cannot build warbot!* ([35-hud.md](35-hud.md#the-message-box--read-and-measured)).
  Either way the designer closes.
- **Clear** deletes the project and lays the designer out again with the kind's
  prefix: `r_`, `fr_` or `a_` (`0x100513c5`).
- **Save**, with a project, opens **the name field**: 3056 *Type the name of the
  designed warbot...* centred on x 320 at y 326 in green, over a field on (270,
  410)–(370, 430) that takes up to 16 characters. A printable character (the C
  library's `isprint`, mask `0x157`) is added, backspace takes the last off,
  Enter ends it (`0x100458f0`), and the takt then
  writes `units/<name>.dat` (`0x10050c28`). Esc drops the field.
- **Load** lists the `.dat` files in the game's `units/` directory, leaving out
  names that begin `bld_unit_`, `view_unit_` or `temp_unit`
  (`0x100511b3`–`0x100511f5`). With any, **the load list** is up: rows at y 320 +
  22 × *i*, four shown at a time with a scroll control; a clicked row loads its
  design (`0x10050c86` onward, in outline). Esc drops the list.
- **Exit** closes the designer (above).

**Recent projects**: when the designer was opened for robots and the designer's
count at `+0xb8e0` is above 0, five icon buttons icons (216, 96) at (200 + 30*k*,
270), tooltip 1558 *Recent projects*, are drawn in `0xff80ff80`
(`0x10050436`); one clicked loads that project (`0x100518ae`,
`0x10055190`). Who fills the count was not read.

## The previews — *read*, and *seen*

**A model view** (`0x1009df90`, 0xc8 bytes) holds one part or the project, a
rectangle and a camera of the same kind as the HUD panels'
([35-hud.md](35-hud.md#the-unit-in-the-middle--read-and-seen)):

- **The model** is loaded from `objects.rlb` as an agent of kind 3 when the
  name begins `f` or `F`, else kind 4 (`0x1009e058`).
- **The camera** (`0x1009e7e0`) is made for the model's bounding sphere, centre
  *c* and radius *r*. With K = 1 ÷ sin 30° = 2 (`0x1009dc10`), it stands K·*r* from
  the centre with a field of 2 × π/6 = 60°, where the HUD's is 1.25 times that.
  If (K − 1)·*r* is short of the shade's `+4`, the distance is that value plus
  *r*; if (K + 1)·*r* reaches 50000, it is 50000 − *r*; either way the field is
  then acos(1 − 2(*r*/*d*)²) = 2·asin(*r*/*d*). The camera is created with 300 and
  0.5 (`0x1009ec61`), as the HUD's is.
- **The model's frame** is its centre moved to the origin, pitched by −0.5 rad
  and turned about z, (0, 0, 1) (`0x1009f300`).
- **It turns** at 0.00075 rad a millisecond, 0.75 rad a second, a whole turn in
  8.4 s: each draw adds the milliseconds since the last times that rate to its
  angle (`0x1009ef1c`), and the turn is built from `Ngi32.dll`'s `ngiGetSinCos`
  of half the angle, a quaternion's half. *Seen*: a rocket pod turns about half
  round in 3.5 to 4 s.
- **Two lights** are given the directions (−1, 0, −1) and (1, 0, −1); the draw
  turns the z test on with writes and `LESSEQUAL` (render states 7, 14 and 23),
  and draws the mesh with flags `0x5f0` (`0x1009ee30`).

## The scan bands — *read*, and *seen*

Three bands sweep down the views (`0x1009f560`, drawn by `0x1009f750`):

| band | rectangle | sweeps at |
|---|---|---|
| the project | (185, 10)–(455, 304) | 0–1 s of every 4 s |
| the source part | (1, 241)–(182, 396) | 2–3 s |
| the destination part | (458, 241)–(639, 396) | 2–3 s |

- **The cycle** restarts every 4 s (`0x1009f710`).
- **The band** is *h* × 5/57 tall, *h* the rectangle's height (`0x1009f6f0`). During
  its second, *t* from 0 to 1, it covers the rows from y₀ + (*h* + *b*)·*t* − *b* to
  y₀ + (*h* + *b*)·*t*, held inside the rectangle, across its width.
- **Its art** is page5 (0, 202), (0, 219) and (0, 236), 163 × 16 noisy strips,
  one after another, a different one each draw, in `0xff009b00` with a green
  specular (⅔*g*, *g*, ⅔*g*), *g* = 254 × (1 − 2|*t* − ½|), brightest mid-sweep.

*Seen* at 107.67–108.33 s: a bright noisy band crossing the empty project view
from top to bottom in about a second.

## For an engine

1. **Open** from the factory screen's constructor button with kind 1 and the
   string `r`; draw it in place of the factory's screen while open, with the
   cursor shown. Close on exit, Esc (no field up) or accept, back to the factory.
2. **Draw** on the 640 × 480 layout, in order: black; for each panel its list
   ground, header, separator, preview ground and grid, part box and circuit; its
   title, six tab frames and icons, and the selected tab's rows (6 shown, 23
   apart from y 65, 180 × 21), their lamps and the scroll bar past 6; the
   project's view, box, model, prompt or hint, numbers, callout, circuit and five
   buttons; the three scan bands. Every piece, rectangle and colour is in the
   tables above.
3. **Tabs**: start with Chassis only; enable Turrets, Internal systems (and,
   under a condition not established, Armour) with the chassis, Weapons and
   Internal systems with the turret, Ammo with a weapon. Keep both panels on the
   same tab.
4. **Rows**: a click selects; a second click on the selected row within 0.2 s is
   a double click; a click in the panel's preview counts as a click on its first
   shown row. A double click from the source adds the part to the project, into
   an empty slot for chassis, turrets and weapons; from the destination it
   removes a chassis, turret or weapon.
5. **Prompts**: *SELECT CHASSIS* with no project; then *SELECT TURRET*, *SELECT
   WEAPON* or *TUNE UP OR ACCEPT TO PRODUCTION* by the selected tab.
6. **Previews** at 2 × the model's radius, 60°, pitched −0.5 rad, turning at 0.75
   rad/s about z, lit from two sides.
7. **Buttons**: accept (enabled with a turret and a valid design) hands the
   design to the factory and closes; clear empties the project; save and load
   use `units/<name>.dat`; exit closes.

## Not established

- What the driven unit's property flag bit `0x8` gates while the designer is
  open, and what the game object's `+0x08` value 4 is.
- The condition under which fitting a chassis enables Armour (`0x10052491`).
- How the font's colour slot turns a row's colour set into its text.
- The load list's rows, scroll control and loading, beyond their outline.
- Who fills the recent projects' count (`+0xb8e0`), and the kinds 2 and 3
  (`fr_`, `a_`): which screen opens the designer for them.
- The tooltip's box and timing (`0x1009bbc0`, [35-hud.md](35-hud.md#who-draws-it-and-what-it-hides--read)).
- The destination panel's draw and takt (`0x1004ccb0`, `0x1004cd00`) beyond their
  sharing the source's routines.
- ~~What writes a row's attach node at `+0x04`~~ — **read**, and **measured**:
  the fits' loop over the part's nodes writes its own counter, which runs 1 up to
  the part's node count and skips the nodes with no socket label, so the field is
  the node's own index; all 138 attachments in the 33 `.dat` files the designer
  writes are node 1 or above and every one carries a stream-10 label
  ([A row's record](#a-rows-record--read-and-measured)).
  **`+0x00` and `+0x24`** are narrowed, not closed: both take the handle the
  project answers for a fitted part (`+0xbc90`'s slot 13, query 6), `+0x24` for the
  part in the row and `+0x00` for the part whose socket the row is, and the unit
  writer emits a row only where `+0x00` is zero. What the handle itself counts was
  not read. **Which tab each of the four `0x330`-byte row arrays holds** is open
  too: the turret add appends its socket rows to the fourth (`+0x33e0c`) and the
  unit writer reads the third (`+0x26f68`).
