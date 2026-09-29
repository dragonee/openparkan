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

**Nothing opens the other two kinds** — *read*. That call is the only one to the
open in `iron3d.dll`: its address is the target of no other `call`, and it sits
in no table — the four-byte value `0x10055c70` occurs nowhere in the file, where
the same search finds the holder's constructor `0x10055910` called once, at
`0x1008cf33`. The kind at `+0x28` has four writers — the constructor's zero
(`0x100559a6`), the open (`0x10055c8a`), and the two closes' zeroes (`0x10055dad`,
`0x10055f88`) — so it is only ever 1. The `fr_` base designer and the `a_`
designer that clear lays out by the kind (`0x100513c5`) are left over in the
shipped game.

**Opening** (`0x10055c70`) does nothing while a kind is set. Otherwise it stores
the arguments, sets `+0x2c`, writes 9 to the game camera's flag word, shows the
mouse cursor, and lays the designer out with the string (`0x1004dff0`). The
first letter of that string, kept at the designer's `+0xb668`, is what the
screen later tests: `r` is the robot designer.

**The flag word is the game camera's, and bit 8 stops it drawing the world** —
*read*. What the open writes through is the level's first field (the game's
`+0x1c`, then `[+0]`), its slot 13 (`+0x34`) with 9 (`0x10055c9b`); its slot 20
(`+0x50`) reads the word back. That object is a `Terrain.dll` `CCamera`: the
vtable's slot 13 (`0x10084a70`) writes `+0x164` and slot 20 (`0x10084ad0`)
returns it, the same pair the outer camera carries the infrared bit `0x20`
through ([30-turrets.md](30-turrets.md#the-outer-camera--read-and-measured)).
The camera's frame render, its slot 5 (`0x100844c0`), tests **bit 8** twice and,
with it set, skips the scene's draw through the landscape (`0x100845e5`) and the
attached object's pass after it (`0x100846fb`). Those are the only two reads of
bit 8 among the camera field's 18 accesses in `Terrain.dll`; the same sweep
finds the infrared `0x20` (`0x10084524`, through slot 20), `0x10`
(`0x10084a88`) and `0x200` (`0x10084790`). So while the designer is up **the
game camera draws no world** — the designer's black rectangle covers the screen
anyway — and nothing here pauses it. The 9 also overwrites the rest of the word,
so a night sight the player had on (`0x20`) is off when the designer closes.

**Bit 1 is the view's "the eye places me"**, and the 9 keeps it — *read*. The unit
camera's getter sets it (`0x1007e707`), the hand-over to a boarded bot clears it on
the hero's view and sets it on the bot's (`0x10075174`, `0x10075180`;
[39-boarding.md](39-boarding.md)), and its one reader is `Control.dll`'s
first-person eye: the per-tick routine that sets the view's frame from the turret's
`CameraCenter` and `TargetDirect` (`0x100234c0`,
[30-turrets.md](30-turrets.md)) asks the view's slot 20 and skips everything unless
bit 1 is set (`0x100234e9`–`0x100234ee`). Sweeping all fourteen modules for a
slot-20 call followed, within eight instructions and after any shift, by a test or
mask of `eax` finds that site alone testing 1 on the answer (three more hits, in
`AniMesh`, `MisLoad` and `Terrain`, test a byte loaded from a global after the
call's loop ends); the same sweep finds the tests
of `0x20` (`iron3d.dll:0x10035c2d`, `0x10038973`; `Terrain.dll:0x10084527`) and
`0x10` (`iron3d.dll:0x10035c8d`), the four clears of 8, and six clears of 1
(`iron3d.dll:0x10038772`, `0x100388ed`, `0x10038b05`, `0x10075174`, `0x10075605`,
`0x1007595e`). So behind the designer the hero's eye goes on placing the view it
does not draw, as it did before.

**It is drawn instead of the mode's own screen.** The screens' draw
(`0x1008d200`) switches on the `CState` mode. In modes 3 and 4 (`0x1008d51c`)
and 5 (`0x1008d444`) it shows the cursor and, while the holder's `+0x2c` is
set, draws the designer (`0x10055dc0`) and nothing else; otherwise the mode's
screen. The factory's screen is mode 5 ([27-ownership.md](27-ownership.md#capture--read)).

**Three things close it**, each clearing bit `0x8` of the camera's flags (read at
slot 20, `+0x50`, written at slot 13, `0x10055d1d`–`0x10055d27`), so the world is
drawn again, deleting the
previews and the project, and zeroing the holder's `+0x2c` and `+0x28`, so the
mode's own screen comes back:
- **the exit button** (`0x10051436`); it also clears both panels (`0x1004a480`);
- **Esc** (the holder's key handler, `0x10055e80`), when neither the load list
  nor the name field is up; with one of them up, Esc closes that instead;
- **accept** (`0x1005144c`), below.

**Input** reaches the holder while it is open. Its mouse handler
(`0x10055ff0`) and its Esc act only while the game object's `+0x08` is 4
(`0x10056004`, `0x10055ea6`): that is the game's state word, and 4 is *playing*
([34-progression.md](34-progression.md#after-the-outcome--read-and-measured)).
Once the mission is won or lost the designer takes neither, and they pass on — Esc
to the one that leaves the mission; the interface pass draws the outcome panel in
place of the screens then, the designer's among them. A click goes to the
designer's buttons first (`0x100504d0`), then the
source panel (`0x10049b80`), then the destination panel; the first to take it
ends the search. While the name field or the load list is up the buttons are not
asked ([The buttons](#the-buttons--read-and-seen)). Its character handler
(`0x10055fa0`) feeds the name field.

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

**A row of the box** (`0x1006ea50`), with *x* = *X*₀ + 15 and the rows a font
height plus 2 apart (`0x1006fa0e`):
- the label at *x* in green `0xff00ff00`;
- the value, `"%6.1f"` whatever the template asks
  ([38-designs.md](38-designs.md#a-parts-box--read)), right-aligned so that it
  ends at *x* + 125: it starts at *x* + 90 + round(35 − its width ÷ the
  horizontal scale) (`0x1006ebd7`–`0x1006ebe5`), in `0xffb4b4ff`;
- the unit at *x* + 128 in green (`0x1006ec01`).

*Seen* at 116.5 s, 1.5 screen pixels to the layout's one: the source box's
figures end at x 208 (138.7), a glyph cell's blank last column short of
*x* + 125 = 140, and its units start at x 215 (143.3), where *x* + 128 = 143.

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
  five are cleared (`0x1004b022` and after). Fitting a chassis enables Turrets,
  Internal systems and Armour in both panels, **each only when the chassis gives
  it a row** (`0x10051f3a`, `0x1005241a`, `0x10052491`); fitting a turret enables
  Internal systems and, when it has a gun socket, Weapons (`0x10052912`,
  `0x10052c24`); fitting a weapon enables Ammo (`0x10053407`). Taking parts off
  turns tabs off again ([below](#which-tab-and-row-a-fit-leaves--read)).

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
- **Its text**, the part's *name (code)*, is at (*X*₀ + 10, 72 + 23k), drawn
  after the font's slot 2 (`+0x08`) is given six diffuse colours and six
  speculars (`0x10046c5e`–`0x10046cad`). The font puts colour *i* on vertex *i*
  of each glyph, and a glyph is two stacked quads, its vertices row by row —
  0 and 1 at the top, 2 and 3 halfway down, 4 and 5 at the foot
  (`Ngi32.dll:0x100111bd`–`0x100112d4`,
  [12-rsli.md](12-rsli.md#how-the-text-is-coloured--read)) — so **the colours are
  a gradient down the glyph**, times its white body:
  - a selected row: `0xffc8c8c8` at the top, white halfway, `0xffc8c8c8` at the
    foot;
  - any other: `0xff323296`, dark blue, at the top, white halfway, `0xff9696fa`,
    a pale blue, at the foot;
  - every specular `0xff000000`, black.

  *Seen*: the text reads light grey, brightest halfway down. At 116.5 s an
  unselected row's glyph rows average (148, 146, 154) at the top, (196, 193,
  204) halfway and (143, 139, 162) at the foot, its blue above its red at the
  foot by 19 to 25, against 15 on the selected row; the recording's colour is
  too subsampled to show more of the blue than that.
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

**The destination's takt** (`0x1004cd00`) — *read* — is the source's with two
things more:
- **scrolling** as the source's: the scroll bar's up and down move the selected
  tab's first shown row by one (`0x1004cd97`–`0x1004ce0b`);
- **an empty slot shows nothing**: for a tab that has just been turned on, when its
  selected row holds no part (the row's `+0x20` is −1), the preview's model is
  deleted (`0x1004ce12`–`0x1004cfcc`). The Armour tab's test reads the Ammo tab's
  selected row (`+0x4d7fc`, `0x1004cf9d`) where every other tab reads its own; an
  armour row always holds a part once a chassis is in — its `_df` defaults — so the
  slip shows nowhere;
- **a picked row sets the source's offer**: the first tab, in the order Chassis,
  Turrets, Internal systems, Weapons, Ammo, Armour, whose control answers a row that
  has just turned on (`0x100481a0`) has the source panel's same tab rebuilt by the
  page builder (`0x10048220`,
  [28-chassis.md](28-chassis.md#what-the-label-is-not--read-as-a-search)) from that
  row's socket label, Weapons with its flag 1; Chassis hands it the destination's
  own prefix (`+0x4dff0`);
- then, when the row holds a part, the preview is made again as the source's is,
  its angle carried on (`0x1004d0e9`–`0x1004d264`); a picked row with none leaves
  the model that was up (`0x1004d0dd`). The fits write the panels' previews too
  (`+0x4dffc`), which is not traced here.

### A row's record — *read*, and *measured*

The rows a destination tab lists are kept in the **destination panel** (the
designer's `+0xbca0`) as **arrays of 64 records, `0x330` bytes each**, one a
tab, `0xcea4` apart: tab *k*'s from `+0x37c` + *k* × `0xcea4`. The addresses
below are the panel's. The panel numbers its tabs **0 Chassis, 1 Turrets,
2 Armour, 3 Internal systems, 4 Weapons, 5 Ammo** (*read*): the chassis fit puts
the `i_arm_` rows on tab 2 and the other slots on tab 3 (`0x10052094`,
`0x10052265`), the turret fit its gun sockets on tab 4 (`0x10052897`), the gun
fit its clip on tab 5 (`0x100532bd`), and the takt wants an empty row on tabs 1
and 4 alone (`0x100509ba`, `0x10050a0e`). So the four arrays the fits and the
writer name are Turrets (`+0xd220`), Armour (`+0x1a0c4`), Internal systems
(`+0x26f68`) and Weapons (`+0x33e0c`); Chassis's is at `+0x37c` and Ammo's at
`+0x40cb0`. Each tab also keeps, from its control at `+0x34` + *k* × `0xcea4`:

| field, tab 0 | what |
|---|---|
| `+0x54` | enabled (the control's `+0x20`) |
| `+0xec` | on: the tab selected (the control's `+0xb8`) |
| `+0xcec8` | **its own selected row** |
| `+0xcecc` | how many rows it holds |

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

### Which tab and row a fit leaves — *read*

**Each tab keeps its own selected row** (`+0xcec8` + *k* × `0xcea4`), and
turning a tab on touches no row: the click's exclusive select (`0x10049e50`)
only turns the other tabs off and this one on, and the takt's copy of it to the
other panel (`0x100506ef`–`0x1005093e`) does the same. So a tab turned on shows
the row it was left on. What moves the rows is the fits and the removals:

| after | the tab it was made on | the tabs it opens | the panels turn to |
|---|---|---|---|
| a chassis (`0x10051bb0`) | — | Turrets, Internal systems, Armour, each at row 0, when the chassis gives it rows (`0x10051f6d`, `0x1005242b`, the armour defaults' own step) | Turrets, when it opened (`0x10051fc2`, `0x10051fd4`) |
| a turret (`0x10052570`) | Turrets steps to its next row (`0x10052754`) | Internal systems at row 0 (`0x10052c42`); Weapons at row 0 with a gun socket (`0x10052947`) | Weapons, when it opened (`0x1005299a`, `0x100529ac`) |
| a gun (`0x10052fb0`) | Weapons steps (`0x100530e9`) | Ammo at row 0 (`0x10053425`) | — |
| a system, armour or a clip | its tab steps (`0x10052ec6`, `0x10053966`, `0x100536c6`) | — | — |
| a turret off (`0x10053df0`) | Turrets steps (`0x10054170`) | Internal systems, Weapons and Ammo back to row 0, or off in both panels when left with no rows | — |
| a gun off (`0x10054210`) | Weapons steps (`0x10054414`) | Ammo back to row 0, or off | — |

A step is to the next row, and from the last back to the first. So a design's
sockets, systems and clips fill one after another, the source panel offering
the next slot's parts each time, and a chassis's own `_df` defaults, each fitted
with its row selected first (`0x10052194`, `0x10052381`), leave their tabs back
at row 0. *Measured*: 20 of the install's 24 chassis give all three tabs rows;
`r_b_07` and `r_l_06` no turret socket, so fitting one leaves the panels on
Chassis; the hero's `r_h_01` and `r_h_03` neither a socket nor armour. 7 of the
55 turrets have no gun socket (`e_tur_bb_11`, `_12`, `e_tur_bt_11`, `_12`,
`e_tur_ht_02`, `e_tur_lb_07`, `e_tur_lt_07`), and their fit leaves the panels on
Turrets. *Seen*: a chassis turns the panels to Turrets and a turret to Weapons,
each with its first row lit.

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
- **Save**, with a project, opens **the name field** (below). Enter, or a click
  on the field, ends the typing, and the next takt writes `units/<name>.dat`
  (`0x10050c28`, the writer accept uses) and drops the field. Esc drops it unsaved.
- **Load** lists the saved designs (below). With any, **the load list** is up; a
  clicked row loads its design. Esc drops the list.
- **Exit** closes the designer (above).

**The name field** — *read*. Save empties the field at the designer's `+0x1444`
and sets it typing (`0x100510b4`–`0x1005112d`); the designer's `+0xad7e` says it is
up. It is a text field on (270, 410)–(370, 430), laid out at `0x1004eaad`, that
holds up to 16 characters.
- **The draw** (`0x10050202`–`0x10050349`): 3056 *Type the name of the designed
  warbot...* centred on x 320 at the view's foot plus 25, y 326, in green; an empty
  load row at (200, 410) (the `+0x146c` control, below); and the field's text,
  centred in the field (`0x100454e0`, handed 0, so with no frame). Typing, all six
  of its glyph colours are magenta `0xffff00ff`; idle they are `0xff323264` at the
  top and foot and `0xffc8c8ff` halfway, and with the cursor on the field
  `0xff9696c8` and white. While it types, a caret — a filled `0xff9600ff` bar 5 wide,
  from the text's end plus 1, as tall as a glyph — shows for the first half of
  every second (`0x10045721`–`0x10045812`).
- **A character** (`0x10055fa0` → `0x100458f0`): the holder passes the C library's
  printable ones, backspace and Enter. Backspace takes the last off; Enter stops the
  typing (`+0x14` clear); any other is added while fewer than 16 are there and the
  text so far is narrower than the field less 30, 70 across (`0x100459a4`–
  `0x100459af`).
- **A click** strictly inside the field turns the typing over and plays
  `BUTTON_CLICK` (`0x10045820`); a click elsewhere is not the buttons', and goes on
  to the panels (`0x100504d0`).
- **The takt** saves when the field is up and not typing (`0x10050b93`–`0x10050c33`),
  so an Enter or a click that stops the typing saves at once. The path is `units/`,
  the text and `.dat`; an empty field writes `units/.dat`.

**The load list** — *read*. Load (`0x10051153`) empties the list and asks
`World3D.dll`'s `stdGetValidRobots` for the designs under `units/`
(`0x10014ce0`). That opens each `units/*.dat` and keeps one that reads as a design
— the `0xf0f1` word, the type, and every component (`0x10014e5d`–`0x10014f07`) —
whose chassis is no bigger than the factory builds, its size letter against the
factory's property `0x201` (`0x10014f19`–`0x10014fc2`), and whose every part is in
the tree and researched, the two flags [38-designs.md](38-designs.md#the-price--read)'s
price reads (`0x1001505b`–`0x1001511c`). The designer then passes over any name
**holding** `bld_unit_`, `view_unit_` or `temp_unit` — the C library's `strstr`
(`0x100b47f0`), not a test of the start — and makes a row of each other, its label
the name between `units/` and `.dat` (`0x100511b3`–`0x1005130b`). With none the
list does not come up.
- **A row** is a control of `0x300` bytes from `+0x176c` (`0x10045bd0`): 242 × 21 at
  x 200, row *i* at y 320 + 22 × (*i* − the first shown), four shown at a time
  (`+0xad6c` the first shown, `+0xad70` the count). It draws page1 (14, 57, 242, 21)
  across, a lamp of the panel rows' three at its left — dark in white, the bright
  one in `0xffc8ffc8` when on, the bright and the green alternating every 0.1 s in
  `0xffffc8c8` under the cursor — and its label centred on x 320 and in the row's
  height, a gradient from `0xff326496` through `0xff9696c8` to `0xff96c8fa`
  (`0x10045e60`).
- **The scroll control** at `+0xc60`, laid out at y 410 (`0x10046080`): a lamp at
  (230, 410), page4 (0, 0, 130, 21) from 235 to 377, and two buttons — up, frames
  page1 (216, 78 + 21*k*, 15, 21) on (377, 410)–(392, 431) and icons (154, 0, 11,
  11) at (378, 413), 13 × 13; down, frames page1 (231, 78 + 21*k*, 18, 21) on (392,
  410)–(410, 431) and icons (154, 11, 11, 11) at (392, 413) — drawn as the
  designer's buttons are (`0x100464c0`). Up takes the first shown back one, down on
  one while more than four are left (`0x10050f53`–`0x10051077`); a click must land
  on the icon (`0x10035920`).
- **A clicked row** (`0x10050cd0`–`0x10050efc`) drops the list and both panels'
  previews, lays both panels out again with the kind's prefix, deletes the project,
  and loads `units/<label>.dat` (`0x10055190`). That reads the file and fits its
  parts again in the file's order, each through the add a double click uses
  (`0x100519e0`), with the destination's row set first: the chassis; a turret in the
  Turrets row of its socket; the chassis's armour in the next Armour row and each
  internal system, the chassis's and the turret's, in the next Internal systems row,
  counted over the file; a gun in the Weapons row of its socket; a clip in the first
  Ammo row of its gun from the last one used. A part not in the tree is passed over
  (`0x100552c8`). The loader then prices the design (`0x100555a5` onward). Both panels
  turn to Chassis, and the recent projects' buttons go off (`0x10050ecb`–`0x10050efc`).
- **The box** while the field or the list is up draws neither the black inner panel
  nor the numbers (`0x1004f1db`, `0x1004f2ea`), and the hint not while the list is
  up (`0x1004faba`); with a project, accept and save are then drawn bright, the
  numbers' routine that would dim them not being run (`0x1004f2f0`, `0x10050409`).

**Recent projects**: when the designer was opened for robots and the designer's
count at `+0xb8e0` is above 0, five icon buttons icons (216, 96) at (200 + 30*k*,
270), tooltip 1558 *Recent projects*, are drawn in `0xff80ff80`
(`0x10050436`); one clicked loads that project (`0x100518ae`,
`0x10055190`). The count is of the filled slots among the five, counted again
whenever a project is pushed to the front: by an accepted design
([36-factory.md](36-factory.md#projects--read-and-measured), through
`0x100557f0` at `0x1005510e`) and by the mission's `prebuild`
([23-economy.md](23-economy.md#what-prebuild-does--read-and-seen),
`0x1004df79`–`0x1004df94`); `0x100557f0` has one more caller, at `0x100a33c8`.

**A tooltip** (`0x1009bc20`, the timer the screens run; `0x1009b970`, the draw)
shows once the cursor has held within 2 pixels of where it was, in each axis,
**for more than 250 ms** by `timeGetTime`; a move of more than 2 restarts the
wait. The box is the text's width plus 8 by the font's height plus 8, in the
game's `+0x18` font, `TOOL_FONT` (`0x1005f9f2`; entry 7 of `ui/font.lib` at
640 × 480, `ui/menu_resources.cfg`), at the cursor and 16 pixels × the vertical
scale below it (12 while the display's slot 12 answers yes,
[40-command-mode.md](40-command-mode.md#the-displays-slot-12-the-system-cursor--read-and-measured)),
turned to the cursor's left or above it where it would cross the screen's right
or bottom edge. It is filled pale yellow `0xfff5f596`, outlined black
`0xff000000`, and the text is black at (+5, +4).

**Who hands it the text** (*read*). The manager (`0x1009bbc0`, a singleton at
`0x1010c3e4`) keeps one text. The game frame clears it before the interface
pass (`0x10060aac`), each widget whose rectangle holds the cursor as it is
drawn hands it its own (23 of the manager's 26 callers, *measured* by
`check_command_views` as a call scan: the commander column's buttons,
enabled or not, `0x1009c401`; the unit box's buttons; a building row's; the
map's exit; the factory panel's, whose texts its constructor makes,
`0x10096c5e`–`0x10096fce`; the tabs and buttons here), and the timer, after the
pass, draws the last handed; with none handed it stamps nothing. It does not run
while the objectives screen is up (`0x10060c7e`). *Seen* at 155.5 s: *Accept to production*, black on pale yellow, above
and to the right of the cursor on accept, near the screen's foot; 0.3 s after
the cursor settled.

## The previews — *read*, and *seen*

**A model view** (`0x1009df90`, 0xc8 bytes) holds one part or the project, a
rectangle and a camera of the same kind as the HUD panels'
([35-hud.md](35-hud.md#the-unit-in-the-middle--read-and-seen)):

- **The model** is loaded from `objects.rlb` as an agent of kind 3 when the
  name begins `f` or `F`, else kind 4 (`0x1009e058`).
- **The camera** (`0x1009e7e0`) is made for the model's bounding sphere, centre
  *c* and radius *r*, the sphere its interface `0x20` slot 3 answers. With
  K = 1 ÷ sin 30° = 2 (`0x1009dc10`), it stands K·*r* from the centre with a
  field of 2 × π/6 = 60°, where the HUD's is 1.25 times that. If (K − 1)·*r* is
  short of the shade's `+4`, the distance is that value plus *r*; if (K + 1)·*r*
  reaches 50000, it is 50000 − *r*; either way the field is then
  acos(1 − 2(*r*/*d*)²) = 2·asin(*r*/*d*). The camera is created with 300 and 0.5
  (`0x1009ec61`), as the HUD's is.
- **Where it stands and which way it looks** — *read*. The creation block's first
  three floats are the camera's position and the next three its angles
  (`0x100367b0` builds its matrix from them). The model view hands it
  (−*d*, 0, 0) and three zero angles (`0x1009ec03`–`0x1009ec57`), and never moves
  it: the camera stands on −x and **looks along +x**, its left +y, its up +z —
  the columns forward, left, up of every camera here
  ([10-sky.md](10-sky.md#fog)).
- **Its 60° spans the view's width.** The field goes into the block's `+0x14`
  (`0x10036a3e`), which the camera's slot 6 (`Terrain.dll:0x100848a0`, `CCamera`,
  the only camera class, which `LoadCamera` makes) stores and hands its view's
  slot 10 (`0x10081f60`): the view's field of view, which `Ngi32.dll` builds as
  the full horizontal field ([10-sky.md](10-sky.md#the-sun-and-the-moon-are-drawn)).
  The panels' 160 × 151 views are 60° across and a little less high; the
  project's 250 × 290 view 60° across and about 68° high.
- **The model's frame is pitched by −0.5 rad about y**: the first frame the camera
  routine builds (`0x1009ec9d`–`0x1009ed6c`, two angles, 0 and −0.5 from
  `0x100e5d70` and `0x100e6980`), kept at the view's `+0x28`, is (cos 0.5, 0,
  −sin 0.5; 0, 1, 0; sin 0.5, 0, cos 0.5) by rows, which leans the model's up toward
  −x, **toward the camera**: the view looks down on the model at 0.5 rad. Its
  translation column is −*c*, the sphere's centre negated (`0x1009ec66`–`0x1009ec91`).
- **The turn is the model's own, under the pitch** — *read*. Each draw
  (`0x1009efea`–`0x1009f060`) builds the turn *R* from a quaternion (cos ½*θ*,
  *a* sin ½*θ*) about the view's axis *a* = (0, 0, 1) (`+0x10`, from the static
  initialiser `0x1009f300`), `ngiGetSinCos` giving sine first
  (`Ngi32.dll:0x1001b6e0`), and `g_FastProc`'s quaternion-to-matrix (its `+0x38`,
  `Ngi32.dll:0x10014540` in plain x87) lays the matrix down transposed from the
  column-vector one, so *R* turns the model **clockwise seen from above**. The
  multiply at `g_FastProc`'s `+0x10` (`Ngi32.dll:0x1001c540`) is *A* · *B* for
  3 × 4 matrices whose fourth column is the translation, as `0x100326a0` sets it.
  The draw forms *R* · T(−*c*), then T(*c*) before it, then the pitch frame before
  that, and hands the model **P · T(*c*) · R · T(−*c*)** as its placement: turned
  about its own centre and its own z, then pitched. The pitch frame's −*c* then
  lands the centre not on the origin the camera looks at but at (P − I) · *c*: a
  centre *h* above the model's origin sits *h* sin 0.5 nearer the camera and
  *h* (1 − cos 0.5) lower. *Derived*, with the stream-2 header's sphere as *c*: of
  the 342 offered parts' meshes, 164 have their centre on their origin and sit
  true; the rest sit off the view's middle, 127 by more than 3 % of the view's
  half-width and at most `e_gun_bs_01` by 18 %; the recording's parts by 5 % or
  less.
- *Seen*: the turn's sense in the recording. From 108.95 to 109.95 s the source's
  first turret — a long box, its red end on the left — turns so that the red end
  rises and shrinks as it goes away and the right end's grille face comes into view
  as it comes near: the model's +y side going to +x, away from a camera on −x, is
  the clockwise turn. The offsets are too small to measure on a turning silhouette.
- **It turns** at 0.00075 rad a millisecond, 0.75 rad a second, a whole turn in
  8.4 s: each draw adds the milliseconds since the last times that rate to its
  angle (`0x1009ef1c`), and the turn is built from `Ngi32.dll`'s `ngiGetSinCos`
  of half the angle, a quaternion's half. *Seen*: a rocket pod turns about half
  round in 3.5 to 4 s; at 108.25 s the L-2f, just in, is side-on, its nose across
  the view, as a camera on the model's x axis sees it at angle 0.
- **Two lights** of the model's own light manager (interface `0xe`, `+0x68`):
  both directional (type 3, slot 12), given 1 through slot 6, **coloured
  (2, 2, 2)**, alpha 0 (slot 3, `0x1009e888`–`0x1009e8f9`, `0x1009e983`), and
  flagged `0x80000000`, so they light their owner alone (slot 13,
  [11-effects.md](11-effects.md#what-a-light-does-to-a-surface--read-and-measured)).
  **Every draw sets their directions again**, (−1, 0, −1) and (1, 0, −1)
  normalised, in space 2 (`0x1009f087`–`0x1009f114`, slot 9), which turns them
  through the manager's object's placement
  ([10-sky.md](10-sky.md#where-the-two-lights-point--read)) — the model's, set
  just before (`0x1009f060`). So **the lights turn with the model**: they come
  down at 45° from its own two sides, ±x, its top lit by both, its sides by one,
  its front and back by neither, however it turns. The draw then turns the z test
  on with writes and `LESSEQUAL` (render states 7, 14 and 23), and draws the mesh
  with flags `0x5f0` (`0x1009ee30`) between the view camera's `ICamera2` slots 3
  and 4 (`0x1009f168`, `0x1009f190`).
- **The scene colour is the world's, this takt** — *read*. The colour a material
  adds (`Terrain.dll:0x100308b8`) is the renderer's `+0x2c`, and the renderer is one
  global, `0x100a5d30` (`0x1002ff22`); each prim buffer's render copies its state
  block from the shader state it was handed (`0x1003d990`–`0x1003da6c`, and its three
  siblings), and every prim buffer is handed the same one, the shader component's
  interface 4, a singleton ([10-sky.md](10-sky.md#the-scene-colour-is-added-to-every-material)).
  Nothing in the model view's draw sends it a colour, and the sky sends its own every
  takt, whether or not the world is drawn. So a preview's materials take the scene
  colour the world's take at that moment of the sky's clock; with no sky yet, the
  shader's own 0.2 grey.

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

**How the specular lights the strip** — *read*, *measured*, and *seen*. The quad
goes out through the sprite draw (`0x1008f970`, flags 1 and 0 from
`0x1009f81f`: the render's slot 6 given 4, and phase 1), whose stage 0 is the texture
**modulated** by the diffuse, its alpha the texture's, alpha-tested at 1 and
up (`Ngi32.dll` phase record 1, `0x100348d0`). The vertices' specular is then
**added** by the device after the stage, held to 1, before the blend by the
texel's alpha. The strips are **one colour, (255, 221, 255), on every texel of
all three** — 7824 of 7824 — their noise in their alpha alone (*measured*), so
a band's texel is (⅔*g*, 134 + *g*, ⅔*g*) ÷ 255, held to 1, laid over the
view by its alpha (221 × 155 ÷ 255 = 134). No module sets
`D3DRENDERSTATE_SPECULARENABLE`: no push of 29 before an indirect call in
`iron3d`, `Terrain`, `Ngi32`, `World3D`, `AniMesh`, `Effect`, `services` or
`Control`, and no 29 as an immediate in `Ngi32.dll` at all, where the same sweep
finds 28, the fog switch, at
12 sites in `iron3d.dll` and 3 in `World3D.dll`, and no phase record carries it
(the 20 records' render states are 15 and 25, one each, and nothing else); so
the device's own state stands. That it adds is the recording's:

*Seen* at 107.67–108.33 s: a bright noisy band crossing the empty project view
from top to bottom in about a second. Its brightest pixels go (63, 175, 63) at
107.75 s, (150, 198, 150) at 108.10 s, mid-sweep, and (91, 185, 92) at
108.30 s — red and blue alike, rising and falling with *g*, which the diffuse
`0xff009b00` alone, green only, cannot give.

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
3. **Tabs**: start with Chassis only; a chassis enables Turrets, Internal
   systems and Armour where it gives them rows and turns the panels to Turrets;
   a turret enables Internal systems, and Weapons where it has a gun socket,
   turning the panels there; a weapon enables Ammo. Keep both panels on the same
   tab. Each tab keeps its own selected row; every fit and removal steps its tab
   to the next row, wrapping round, and a tab a removal empties turns off.
4. **Rows**: a click selects; a second click on the selected row within 0.2 s is
   a double click; a click in the panel's preview counts as a click on its first
   shown row. A double click from the source adds the part to the project, into
   an empty slot for chassis, turrets and weapons; from the destination it
   removes a chassis, turret or weapon.
5. **Prompts**: *SELECT CHASSIS* with no project; then *SELECT TURRET*, *SELECT
   WEAPON* or *TUNE UP OR ACCEPT TO PRODUCTION* by the selected tab.
6. **Previews**: the camera at 2 × the model's radius on −x, looking along +x,
   60° across the view's width; the model placed by P · T(*c*) · R · T(−*c*), R
   turning it clockwise seen from above at 0.75 rad/s about its own z through its
   centre, P the −0.5 rad pitch about y, its top toward the camera, with −*c* in its
   translation; two lights of (2, 2, 2) from (∓1, 0, −1) in the model's own frame;
   the world's scene colour.
7. **Buttons**: accept (enabled with a turret and a valid design) hands the
   design to the factory and closes; clear empties the project; save opens the
   name field and writes `units/<name>.dat` when its typing ends; load lists the
   valid `units/*.dat` a page of four at a time and fits a picked one's parts
   again; exit closes. Take the mouse and Esc only while the mission is played.

## Not established

- ~~What the driven unit's property flag bit `0x8` gates while the designer is
  open~~ — **read**: the flag word is the game camera's (`CCamera` `+0x164`), not
  a unit's, and bit 8 makes its frame render skip the world
  (`Terrain.dll:0x100845e5`, `0x100846fb`), so the world is not drawn behind the
  designer and nothing is paused by it ([Opening and closing](#opening-and-closing--read)).
  ~~Still open: what the game object's `+0x08` value 4 is, and what the word's
  bit 1 does.~~ **Read**: `+0x08` is the game's state word and 4 is *playing*
  ([34-progression.md](34-progression.md#after-the-outcome--read-and-measured)), so
  the designer takes the mouse and Esc only until the mission is won or lost; and
  bit 1 is the view's "placed by the eye", read only by `Control.dll`'s first-person
  eye (`0x100234ec`), which the 9 leaves on ([Opening and closing](#opening-and-closing--read)).
- ~~The condition under which fitting a chassis enables Armour (`0x10052491`).~~
  **Read**: the chassis gives the Armour tab a row, an `i_arm_`-labelled slot —
  the count the slot loop keeps (zeroed at `0x10051fe4`, raised at
  `0x100521a8`); Turrets and Internal systems take the same test on their own
  counts ([Which tab and row a fit leaves](#which-tab-and-row-a-fit-leaves--read)).
- ~~How the font's colour slot turns a row's colour set into its text.~~ **Read**:
  one colour a glyph vertex, top, halfway and foot, a gradient down the glyph —
  `0xffc8c8c8`, white, `0xffc8c8c8` on a selected row, `0xff323296`, white,
  `0xff9696fa` on the rest ([The rows](#the-rows--read-and-seen);
  [12-rsli.md](12-rsli.md#how-the-text-is-coloured--read)).
- ~~The load list's rows, scroll control and loading, beyond their outline.~~
  **Read**: `stdGetValidRobots` lists the `units/*.dat` that read as designs, fit
  the factory's size and are all researched; names holding `bld_unit_`,
  `view_unit_` or `temp_unit` are passed over; four 242 × 21 rows from (200, 320)
  and a scroll control at y 410; a picked file's parts are fitted again in its own
  order, and both panels turn to Chassis. The name field is read with it: 16
  characters in (270, 410)–(370, 430), magenta while typing, saved when the typing
  stops ([The buttons](#the-buttons--read-and-seen)).
- ~~Who fills the recent projects' count (`+0xb8e0`)~~ — already **read** in
  [36-factory.md](36-factory.md#projects--read-and-measured) and
  [23-economy.md](23-economy.md#what-prebuild-does--read-and-seen): the filled
  slots, counted again at each push to the front, by an accepted design and by
  `prebuild` ([The buttons](#the-buttons--read-and-seen)). ~~Still open: the kinds
  2 and 3 (`fr_`, `a_`): which screen opens the designer for them.~~ **Read**:
  none. The factory's call at `0x1009810a` is the only one to the open, with kind 1,
  and the kind's other writers only zero it ([Opening and closing](#opening-and-closing--read)).
- ~~The tooltip's box and timing (`0x1009bbc0`, [35-hud.md](35-hud.md#who-draws-it-and-what-it-hides--read)).~~
  **Read**: it shows after the cursor has held within 2 pixels for more than
  250 ms (`0x1009bc20`), a box of the text plus 8 each way, pale yellow
  `0xfff5f596` outlined black, the text black, 16 below the cursor and turned
  left or up at the screen's edges (`0x1009b970`; [The buttons](#the-buttons--read-and-seen)).
- ~~The destination panel's takt (`0x1004cd00`).~~ Its draw is **read**: `0x1004ccb0`
  is the source's `0x1004bb00` with the side argument 1 and without the source's
  dropping of its preview when the selected tab has no rows. **Read**, the takt: it
  scrolls as the source's, drops its preview when a tab turns on at an empty row,
  and a picked row rebuilds the source's same tab for that row's socket and, when
  it holds a part, the preview ([The rows](#the-rows--read-and-seen)). What the fits
  do to the panels' previews (`+0x4dffc`) is not traced.
- ~~The scene colour a preview's materials take while the designer is up, and the
  order of the turn and the pitch (*inferred*).~~ **Read**: the world's scene colour,
  one for the game, which the sky sends every takt; and P · T(*c*) · R · T(−*c*), the
  turn clockwise about the model's own centre under the pitch, the centre left off
  the origin by (P − I) · *c* ([The previews](#the-previews--read-and-seen)).
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
  not read. ~~**Which tab each of the four `0x330`-byte row arrays holds**~~ —
  **read**: they are the destination panel's, one a tab, which the panel numbers
  0 Chassis, 1 Turrets, 2 Armour, 3 Internal systems, 4 Weapons, 5 Ammo; the four
  are Turrets (`+0xd220`), Armour (`+0x1a0c4`), Internal systems (`+0x26f68`) and
  Weapons (`+0x33e0c`) ([A row's record](#a-rows-record--read-and-measured)).
