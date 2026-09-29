# The cockpit HUD — the panels, the radar, the weapons and the messages

What the player sees over the world while driving a unit: the weapons list at
the top right, the message box at the top, the radar in the middle at the
bottom, the target panel at the bottom left and the player's own unit at the
bottom right; and over them, the objectives screen on F12 and the satellite
map on M. This page reads how `iron3d.dll` builds and draws them, and
measures the result against the install's art and a recording of Mission 01.

**Every claim is tagged**, as in [15-behaviour.md](15-behaviour.md):
- *measured* is re-derived by `openparkan verify`;
- *read* comes from the disassembly at the address given;
- *derived* follows from the two;
- *guess* fits and is not established.

The HUD's art is the `textures` resource of `ui/game_resources.cfg`: pages of
`ui/ui.lib`, each a 256 × 256 `Texm` ([02-texm.md](02-texm.md)). Its fonts are
the `fonts` resource beside it ([34-progression.md](34-progression.md#after-the-outcome--read-and-measured)).

## The weapons and the messages

### Everything is drawn on a 640 × 480 screen — *read*

**The builder** (`iron3d.dll:0x100433a0`) makes the cockpit's widgets. Their
draw methods run each frame from `0x10043b20`; `0x10043c70` files a widget by
its id.

| HUD field | built by | size | id | what it is |
|---|---|---|---|---|
| `+0x14` | `0x10042d10` | 0x1b0 | 8 | the reticle: page9's circle (77, 0) 64 × 64, corner arc (54, 28) 23 × 23 and ruler (168, 0) 82 × 15, drawn about (320, 240) in `#37ff37` ([below](#the-reticle--read)) |
| `+0x10` | `0x1003ed60` | 0x610 | 5 | the indicators under the radar ([below](#the-indicators--read-and-seen)) |
| `+0x0c` | holder, then `0x1009c9f0` | 0x10 + 0x5a0 | 6 | **the weapons list** and the guided lock ([below](#the-guided-lock--read)) |
| `+0x18` | `0x1003f340` | 0x3c8 | 0 | the radar (draw `0x1003fb90`, [below](#the-radar--read-and-seen)) |
| `+0x1c` | `0x10040a60` | 0x2f0 | 1 | the target panel |
| `+0x20` | `0x10040a60` | 0x2f0 | 2 | the player's own unit |
| `+0x24` | `0x1003eb30`, its 16 lines by `0x1009d910` | 0x314 | 10 | the wingman panel: a line per wingman 19 apart from (0, 0) ([31-packages.md](31-packages.md#the-wingman-menu-from-first-person--read-and-measured)) |
| the panel's `+0x310` | `0x1007a4c0` | 0x194 | — | its order menu, a scrolling list with `scroll_up_icon` and `scroll_down_icon` |

The message box is not one of them. It is a single object the game keeps
beside the HUD ([below](#the-message-box--read-and-measured)).

**Coordinates** are those of a 640 × 480 screen:

- **A sprite is cut** from a page by `0x1008f830`, with arguments (page, page
  size 256, x, y, w, h, quarter turns). The UVs start half a texel in.
- **It is drawn** by `0x1008f970` with (x₀, y₀, x₁, y₁, colour, specular, and
  two flags) as a quad. Corner (x₀, y₀) takes the sprite's (u₀, v₀), so **a
  quad given x₁ < x₀ draws the sprite mirrored**.
- **Screen scale.** Every coordinate is multiplied by two queries of the
  display object before it reaches the device: `services.dll`'s `getDisplay`,
  slots 4 and 5, one per axis. Text widths are divided by the same two. The
  two are the display mode's width ÷ 640 and height ÷ 480
  ([below](#how-the-radar-draws--read)).

**The compound-control skin** is `ui/compaund.cfg`: 39 named pieces, each a
texture name, an offset, a size and an optional rotation. The texture
`ui_menu` is `game_resources.cfg`'s entry 10, `ui_menu1.tex` (*measured*).

- **The loader** (`0x100989b0`) reads 37 of the 39 names into a table of
  0x8c-byte slots, 39 loads in all (*measured*). `ccres_long_button_off` and
  `_normal` are read twice, into slots 17–18 and 22–23. The two radio buttons
  are never read, and slot 13 is never filled.
- **The lookup** (`0x10098850`) takes a kind and a variant:

| kind | slot | pieces |
|---:|---|---|
| 0 | 24 + v | `ending_stub`, `ending_text` |
| 1 | 26 + v | `lamp_stub_ending_off` / `_normal` / `_pressed` |
| 2 | 29 + v | `lamp_text_ending_off` / `_normal` / `_pressed` |
| 3 | 32 + v | `separator_left_text`, `separator_double_text` |
| 4 | 20 + v | `body_stub`, `body_text` |
| 5 | 5 + v | `ray_emitter_off` / `_normal` / `_pressed` |
| 6 | 9 | `ray_body` |
| 7 | 8 | `ray_ending` |
| 8 | v | lamps: 0 red, 1 yellow1, 2 yellow2, 3 green, 4 black |
| 9 | 34 + v | `frame_corner_1`–`_4` |
| 10 | 38 + v | `frame_edge_v`, `frame_edge_h` |
| 11, 12, 13 | 10, 14, 17 + v | exit, short and long buttons |
| 14 | 22 + v | the second copy of the long buttons |
| 15, other | 40, 24 | an empty slot; `ending_stub` |

Every piece lies on its page and holds art (*measured*). The name bar's
`ray_body` and `ray_ending` are dark green (0, 102, 0) at 20% alpha, so the
bar is translucent. The lamps and `body_text` are opaque.

**A row is built with a pen** (`0x100999b0`–`0x1009a9d7`). Each primitive
draws its piece at the pen and moves the pen by the piece's width. Direction 1
runs right to left: the piece covers pen − w − 1 to pen, mirrored, and the pen
moves back by w.

The primitives are:

| primitive | piece |
|---|---|
| `0x10099a30` | kind 0 |
| `0x10099b60` | kind 2 |
| `0x10099c90` | kind 3 |
| `0x10099d80` | kind 5 |
| `0x10099e70` | kind 7 |
| `0x1009a8f0` | kind 8, a lamp |
| `0x10099f60` | a text box: `body_text` stretched to a width, the text centred in `GAME_FONT` |
| `0x1009a380` | a bar: `ray_body` stretched to a width, a fill, and a centred text |

A text box or bar given no width takes 1.1 × its text's width.

### The weapons list — *read*, and *measured*

**What is listed** (`0x10075680`, once per unit record, flag `+0xa4`). The
unit record's `+0x68` holds a 36-byte entry for each of its unit's devices of
type 2, a gun, in device order (`0x1009fe50`). Each entry keeps:

| entry | what |
|---|---|
| `+4`, `+8` | the gun's component index, the turret before it ([29-weapons.md](29-weapons.md#the-players-target-reaches-the-turret--read)) |
| `+0xc` | the gun's rounds left when the list was built: −1 for an unlimited magazine |
| `+0x10` | its rounds left, rewritten every takt (`0x10075cf5`) |
| `+0x14` | its name |
| `+0x20` | selected |
| `+0x21`, `+0x22`, `+0x23` | latches: no rounds, no energy, destroyed |

- **Rounds left** come from interface `0x202`'s property `0x53`, which is
  interface `0x204`'s `0x700` (`Control.dll:0x1002e734`).
- **The name.** On the hero (`ROBOT_HERO`), the guns take `iron3d.dll`'s
  strings 3071–3074 in turn, filled at `0x10074b71`: *AUTOCANNON 25mm*,
  *PLASMA RIFLE LS*, *BATTLE LASER ER* and *AWB MISSILE*. Any other unit
  names a gun by the **research code of the part its device belongs to**, and
  falls back to *NONAME* (*read*, `0x10075a61`–`0x10075ac9`):
  1. interface `0x202` slot 16 (`Control.dll:0x1002eca0`) gives the device's
     `+8`, the part the load message named when the factory built it
     (`0x1002d7a5`). A clip fitted to a gun re-parses the gun's device and
     leaves `+8` alone (`0x1002d890`), so a clipped gun is still its gun part;
  2. the object's interface `0x24` slot 2 (`AniMesh.dll:0x100025e0`) finds that
     part in the agent's part table and gives its member name, the
     `objects.rlb` record — `e_gun_bl_15`;
  3. `0x1008a470` hands the name to the player clan's tree at `0x1010c380`
     ([16-research.md](16-research.md)), `IResearch` slot 2
     (`MisLoad.dll:0x10002a40`): the first `TRFB` entry whose `TRF6` part id
     matches under `_stricmp` (`0x1000d950`) gives its item, else −1;
  4. `0x1008a4b0` asks slot 13 (`0x10002e70`) for that item's `TRF7` short
     code, which answers 0 for an item out of range — and the name stays
     *NONAME*.

  So a fitted gun reads its own code, a gun built into a turret or an animal
  its turret's or its chassis's — the seven turrets with guns of their own are
  `e_tur_bb_11`, `_12`, `e_tur_bt_11`, `_12`, `e_tur_lb_07`, `e_tur_lt_07` and
  the hero's `e_tur_ht_02` — and a part the player's tree does not list reads
  *NONAME*. *Measured*: no part id is listed twice in any of the 29 trees, so
  the first match is the only one. Mission 02's warbot reads LRL36S, LRL36S and
  LFT from `e_gun_bl_15` twice and `e_gun_bc_06` in `tut2_pl.trf`, as the
  recording shows from 249.4 s
  ([39-boarding.md](39-boarding.md#against-the-recording--seen)).

**Where.** The id-6 holder (`0x1003ecb0`) draws one row a gun, while the unit
has guns:

- the right edge is at x 640 and row *i* is at y 19 *i*;
- each row is 19 high (`0x1009cd30`);
- no rows are drawn while the satellite map is open. That is
  `CMD_JAMES_SATELLITE_MAP`, 739, which sets byte `+0x261` of the game's `+0x2c`
  → `+0x14` object (`0x100740f0`, `0x10074100`). The lock still runs then.

**A row, right to left from x 640** (`0x1009d4a1`–`0x1009d5b6`). *derived*
spans on the 640 × 480 screen:

| piece | width | spans | shows |
|---|---:|---|---|
| `ending_text` | 5 | 634–640 | |
| `body_text` | 12 | 622–635 | the row's key, *i* + 1, centred |
| a lamp | 19 | 603–623 | the gun's state, below |
| `body_text` | 24 | 579–604 | the rounds, centred |
| `separator_left_text` | 5 | 574–580 | |
| `ray_emitter_off` | 10 | 564–575 | |
| `ray_body`, the bar | 120 | 444–565 | the name, centred, over the charge fill |
| `ray_ending` | 6 | 438–445 | |

- **The rounds** are `"%4d"` of `+0x10`, padded to four characters before
  centring. They read string 5094, *INF*, instead when `+0xc` is −1. They also
  read *INF* when the first word of the game's `+0x20` list is 6: that list
  is the interface's `CState` mode stack, and 6 a building's screen
  ([39-boarding.md](39-boarding.md#the-game-view-keeps-a-stack-of-modes--read)).
- **The bar is the gun's charge, not its ammunition.** The fill is interface
  `0x202`'s property `0x62`, the component's level (`0x204` id 1,
  `Control.dll:0x1002bbb1`). For a gun the level is capacitor ÷ value 1
  ([29-weapons.md](29-weapons.md#a-gun-is-a-capacitor-a-magazine-and-a-clock--read)).
  - The fraction is taken as 0 when it is below 0 or above 1, and made a
    percentage *p*, rounded.
  - The fill is a rectangle anchored at the bar's right edge (`0x1009a380`).
    Across, it runs from 2 in from that edge to *p* % of 120 in from it. Down,
    it runs from 3 below the row's top to 3 above its bottom. With *p* = 0
    there is no fill.
  - Its colour is ARGB `0x80800000` (half-alpha dark red) under 20 %,
    `0x80808000` (olive) from 20 % to under 80 %, and `0x80008000` (green)
    from 80 %.
- **The lamp** is black while the gun is not selected. That is bit 4 of the
  manual controller's slot 6 (`World3D.dll:0x1000f000`), read into `+0x20`
  each frame. A selected gun's lamp comes from its report word
  ([29-weapons.md](29-weapons.md#the-guns-takt-a-stroke-then-the-interval)),
  through interface `0x202` slot 10 and the case table `0x1009d8cc`:

  | report | the gun | lamp | name |
  |---:|---|---|---|
  | 0 | ready, not asked to fire | green | |
  | 1 | its target inside the gate, locking | yellow2 | |
  | 2 | no target for a lock | yellow2 | string 6250 *OUT OF RANGE*, `#ff5c5c` |
  | 3 | a stroke under way | yellow2 | |
  | 4 | waiting its interval | yellow1 | |
  | 5 | destroyed, or no rounds | red | |
  | 6 | its capacitor short of a shot | yellow1 | |
  | 7 | not ready (arm folded), or the target beyond value 8 | yellow2 | *OUT OF RANGE* |
  | 8 | the target off the barrel | yellow2 | *OUT OF RANGE* |

- **The key box is a label.** It shows *i* + 1, the number that selects the
  gun ([29-weapons.md](29-weapons.md#the-button-reaches-the-selected-guns)). It
  has no pressed state; the `_pressed` pieces are not used here.
- **Text** is `GAME_FONT`, white.
  - While the wingman selector is on (the HUD's `+0x40`, state not 0), every
    row's text is grey `#808080` and every lamp black (`0x1009b700`, the
    widget's `+0x21`).
  - The key and the rounds use the row colour; the name uses the name colour.
- **Voices** (`0x1009d5ba`–`0x1009d834`) play only for the unit record at
  the world's `+0xaec`, in view states 1, 3 and 6, each once as its latch
  rises:
  - `VOICE_WEAPON_DESTR` when report 5 comes with the gun's node destroyed
    (interface `0x204` id `0x300`, `Control.dll:0x1002bc00`);
  - `VOICE_WEAP_AMMO_OUT` when the rounds reach 0;
  - `VOICE_WEAP_ENERGY_OUT` when the charge falls under 0.01.

**Against the recording of Mission 01** (960 × 720, 1.5 × the screen; not
re-derived by `verify`):

- The rows are 19 apart from the top.
- The name bar runs 444.7–565.3.
- The four names and the counts *493*, *150*, *INF* and *4* read as above.
- At 150 s the lamps of the cannon and the laser are lit and the plasma
  rifle's and missiles' are dark, as the guns that start selected
  ([29-weapons.md](29-weapons.md#the-button-reaches-the-selected-guns)).
- The laser's fill stops short of the bar's left end.

### The guided lock — *read*

The routine that draws a weapon row (`0x1009cd30`) first draws that gun's
**lock**. It draws four corners that close on the target while a guided gun
locks, and it plays the lock's two sounds. The holder (`0x1003ecb0`) calls it
for every gun of the driven unit, in list order. A counter *k* starts at 0 each
frame and grows by 4 after each lock drawn. Unlike the row, the lock runs while
the satellite map is open.

**When it draws.** The lock draws only when all of these hold:

- **The gun's report word** (interface `0x202` slot 10, `+0x11c`) is 0 or 1
  (`0x1009ce00`): locked and waiting, or locking.
- **The gun's round is marked 16.** Property `0x64` answers a gun with its
  `+0x9c` (`Control.dll:0x1002e60b`), which holds the 27 words it copied from its
  round's frame at link (`0x100297a4`). Its `+0x60` is the frame's `+116`. It
  must read `0x10` (`0x1009ce0d`). *Measured* over `weapon.rlb`'s 66 rounds:
  exactly the 20 that carry a seeker read 16 there. Of the others, 36 read 4, 6
  read 0 and 4 read 12. So only a guided gun draws a lock: the hero's plasma
  rifle and missiles, not its cannon or laser.
- **The driven unit has a target** (the record's `+0x38` list, `0x1009ce18`).
- **The gun is selected** (the entry's `+0x20`) **and has rounds** (`+0x10`
  above 0 unsigned, so an unlimited −1 passes).
- **The lock's share *f* is not 0.** It is slot 10's float: the gun's property
  `0xf00`, `+0x17c` ([29-weapons.md](29-weapons.md#a-guided-gun-waits-for-a-lock--read-and-measured)).
- **The target's point projects** through the main camera (`0x100cd1a0`). Its
  (x, y) is divided by the display's two scales, so it lies on the 640 × 480
  layout.

**Where the corners go.** Each corner is 9 × 9. With *f* from 0 to 1, each edge
is rounded to the nearest pixel (`fistp`):

| edge | at *f* = 0 | at *f* = 1 | formula |
|---|---|---|---|
| left, L | 30 − *k* | x − 20 − *k* | (x − 50) *f* + 30 − *k* |
| top, T | 30 − *k* | y − 20 − *k* | (y − 50) *f* + 30 − *k* |
| right, R | 630 + *k* | x + 20 + *k* | (x − 610) *f* + 630 + *k* |
| bottom, B | 450 + *k* | y + 20 + *k* | (y − 430) *f* + 450 + *k* |

- The corners are drawn in turn (`0x1008f970`, flag 1, specular black): (L, T)–(L
  + 9, T + 9), (R − 9, T)–(R, T + 9), (R − 9, B − 9)–(R, B) and (L, B − 9)–(L + 9,
  B).
- So the corners start at the HUD's (30, 30)–(630, 450). As the lock counts they
  close on a 40-pixel square about the target. A second locking gun's corners
  stand 4 further out.

**The art.** The weapons list's constructor (`0x1009c9f0`) cuts one 9 × 9 piece
of `page9` four ways (`0x1009cc50`–`0x1009ccde`):

| corner | widget | cut from | quarter turns |
|---|---|---|---:|
| top left | `+0x36c` | (0, 32) | 0 |
| top right | `+0x3f8` | (0, 32) | 1 |
| bottom right | `+0x484` | (−1, 31) | 2 |
| bottom left | `+0x510` | (−1, 31) | 3 |

**The colour** is `0xff14GG14`, with GG = 205 − round(*f* × −50)
(`0x1009ce84`–`0x1009cea0`). It runs from (20, 205, 20) as a lock starts to
(20, 255, 20) when it is done.

**The sounds.** After a lock is drawn, the widget reads the time since its stamp
`+0x59c`. The stamp is one for all the rows. It uses `getTimer`'s slot 3, which
gives the milliseconds since the stamp × 0.001 (`services.dll:0x10005890`):

- **Report 1, locking:** past 0.35 s the stamp is renewed and `TARGET_ZOOM`
  plays (`0x1009d06e`).
- **Report 0, locked:** past 0.2 s the stamp is renewed and `TARGET_READY` plays
  (`0x1009d109`).
- Both go through the sound server's slot 2, as the theme does.

`ui/game_resources.cfg` binds `TARGET_ZOOM` to `i_trg_zoom.wav`, a 35 ms beep,
and `TARGET_READY` to `i_trg_ready.wav`, 97 ms, both 22,050 Hz mono in
`sounds.lib` (*measured*). So a locking gun beeps about three times a second,
and a locked one about five times.

**What the share is** (`Control.dll:0x10029be0`, the gun's slot 10). The time
driver calls it for each component every frame, after the component's events
(`0x1002d317`). In report 1 it writes 1 − lock left ÷ value 9, or 1 when value 9
is not positive. In report 4 it writes the wait's progress. In any other report
it leaves the share as it was. The result is held to 0..1. So *f* grows from 0
as a locking gun's lock counts down. A lock that has just refilled reads 0 and
draws nothing. A gun locked and not asked to fire keeps its last share, just
short of 1.

*Seen*: the player of the Mission 01 and Mission 03 recordings never fires the
plasma rifle or the missiles. Their counts stay at 150 and 4 throughout, so no
recording shows the lock.

### The message box — *read*, and *measured*

**One box at a time** (the game's pointer `0x1010c07c`, shown byte
`0x1010c080`).

- **When a message arrives** (`0x1007ed50`), a message of kind 1–4 goes into
  the history (`0x1007f060`). The current box is deleted, and a new one is made
  from the newest message and shown (`0x1007f580`).
- **Where messages come from.**
  - A script's `MESSAGE_INFO` gives kind 4 when the message sets
    `info_system`, and 3 otherwise
    ([34-progression.md](34-progression.md#messages--read-and-measured)).
  - The game's own lines, such as 3040 *Vacant vehicle detected...*, are kind
    2 (`0x1007eb60`).
  - Records of other kinds go to slot 20 of `World3D.dll`'s `GetQueue` object
    and make no box (not followed).
- **The header** (`0x1007f750`, table `0x1007f9a0`):

  | kind | header |
  |---:|---|
  | 1 | 6171 *from: %s*, a name the `GetQueue` object's slot 19 gives for the record's `+0xc` |
  | 2 | 1541 *from: System* |
  | 3 | 3057 *from: Training assistant* |
  | 4 | 6214 *from: Information assistant* |

- **Lifetime.** Each frame (`0x1007f4f0`) deletes the box once 20 seconds have
  passed since it was made. That is the timer's slot 3, seconds since a
  millisecond stamp (`services.dll:0x10005890`). In the recording, two boxes
  that no later message replaced left 19.75 s and 20.0 s after their text
  appeared.
- **Hiding.** `CMD_PAGER` (729, F2) hides the box, or shows it again while it
  lives (`0x10071ed5`). `CMD_HELP` (749, F1) opens the history.

**The text** (`0x1007fad0`):

- **Wrapping.** The text is wrapped in `GAME_FONT` to the box's width less 10.
- **At most six lines.** A text of more than six keeps six. The sixth then loses
  words from its end until it and *...* fit 0.95 of that width, and *...* is
  added.
- **The footer.** A truncated text gets a footer, 6208 *Press %s to see more*,
  with the key bound to `CMD_HELP`.

**The box** (`0x1007fe80`), on the 640 × 480 screen:

- **Position.** x 230, y 0, width 182.
  - When the view is in state 2, or the game's `+0x20` list starts with 5, it
    moves to x 374, y 352, width 266.
  - That is re-tested each frame; a change re-wraps the text.
- **The line step** *l* is `GAME_FONT`'s height + 2, over the vertical scale,
  rounded.
- **The height.** With *n* lines and *k* = 1, or 2 with a footer, the box runs
  to y + (*n* + *k*) *l* + ⌊*k l* / 2⌋ + 16.
- **The frame** (`0x1009afa0`, `0x1009abf0`):
  - `frame_corner_1`–`_4` (8 × 8, turned 0°, 90°, 180°, 270°) at the corners;
  - `frame_edge_h` and `_v` (47 × 5, stretched) along the sides between them,
    placed as [the satellite map's frame](#the-panel-in-the-cockpit--read-and-seen)
    reads;
  - the inside filled from 5 in, in `0x80008000`, green at half alpha.
- **The header** is white at (x + 8, y + 9).
- **The lines** are `#dcdcdc`, from 1.5 *l* below the header, *l* apart.
- **The footer** is white, 1.5 *l* below the last line.
- The box plays no sound; the message's voice is the message's own.

**Against the recording** (not re-derived):

- The box spans 230–412 across and 1–87 down.
- The six lines are 8 apart, so *l* = 8 there. The box's bottom at
  (6 + 2) × 8 + 8 + 16 = 88 agrees.
- The header sits 12 above the first line, 1.5 *l*.
- The footer sits 12 below the last line, *l* + *l* / 2.
- The text *"Special systems of the battle suit"...* ends its sixth line in
  *...* above *Press F1 to see more*.

### Not established

- ~~What the display object's two scale queries return.~~ Answered: the mode's
  width ÷ 640 and height ÷ 480
  ([How the radar draws](#how-the-radar-draws--read)).
- ~~The game's `+0x20` list.~~ Answered: it is the interface's `CState`, a
  stack of modes: 0 on foot, 1 in a bot, 3 an HQ, 4–6 a building's screen, 7
  the game menu ([39-boarding.md](39-boarding.md#the-game-view-keeps-a-stack-of-modes--read)).
  The names at `0x1005a5cc`, once read as its modes, belong to a separate
  global.
- ~~How a non-hero unit's gun gets its name (`0x1008a470`, `0x1008a4b0`).~~
  **Read**: the short code the player clan's research tree gives the part the
  gun's device belongs to, found by the part's name, or *NONAME*
  ([The weapons list](#the-weapons-list--read-and-measured)).
- Whether `getTimer` runs on real time or on a clock. Its slots read the clock
  object at `+4` when one is set (slot 0 sets it), else `timeGetTime`. So it is
  not known whether the guided lock's beeps pause with the game.
- ~~The widget at the HUD's `+0x10` (`0x1003ed60`, id 5).~~ Answered: the
  indicators ([below](#the-indicators--read-and-seen)).

## The radar and the indicators below it

Three of the widgets the HUD builder (`iron3d.dll:0x100433a0`) makes draw the
middle of the screen:
- **the radar**: widget id 0, constructor `0x1003f340`, draw `0x1003fb90`, with
  its two gauges `0x1003f6a0` and `0x1003f900`;
- **the row of indicators** under it: id 5, `0x1003ed60`, draw `0x1003f150`;
- **the reticle**: id 8, `0x10042d10`, draw `0x10042ed0`.

A fifth tag is used here. *Seen* marks what a gameplay recording of Mission 01
shows (960 × 720, so 1.5 × the HUD's space). `openparkan verify` cannot re-derive
it.

### How the radar draws — *read*

This adds to [Everything is drawn on a 640 × 480
screen](#everything-is-drawn-on-a-640--480-screen--read).

**The two scales.** `services.dll` keeps them in the display mode's record
(`0x10004610`):
- `sx`, the mode's width × 1/640, which `IDisplay` slot 4 returns
  (`0x10004a80`);
- `sy`, its height × 1/480, from slot 5 (`0x10004a90`).

So at a wider aspect the HUD stretches rather than keeping its shape
(*derived*).

**The sprite draw's other arguments** (`0x1008f970`, after the corners):
- **Colour** is the vertex colour. It multiplies the art (*seen*: the gauges'
  grey art shows orange).
- **Blend** 1 selects `Ngi32.dll`'s blend mode 4 (`SRCALPHA`/`INVSRCALPHA` with
  alpha test, [07-objects.md](07-objects.md); slot 6, `0x10008680`). Blend 0
  selects mode 0.
- **Stage** picks slot 30's stage set, 7 or 1 (`0x10008750`, not read). Every
  sprite here passes 0.
- **Four corners.** `0x1008fa30` draws a cut on four given corners.

**Lines, circles, rectangles and text** come from `services.dll`'s GUI server,
the display's `+8` (vtable `0x1003a198`):

| slot | what | arguments |
|---:|---|---|
| 0 | coordinates are HUD units (1, the default) or screen pixels (0) | flag |
| 1 | a line | x0, y0, x1, y1, colour, clip |
| 2 | a circle outline, 20 segments | x, y, r, colour, clip |
| 3 | a rectangle outline | x0, y0, x1, y1, colour, keep alpha |
| 4 | a filled rectangle | x0, y0, x1, y1, colour, keep alpha |
| 5 | text with a black shadow | font, string, x, y, colour |
| 6 | a triangle outline, apex up at `y − r` | x, y, r, colour |

The addresses are `0x100022b0`, `0x100017f0`, `0x10001910`, `0x10001a60`,
`0x10001bc0`, `0x10001cf0` and `0x10001de0`. Unless told to keep it, a
rectangle's alpha is forced to 255.

### The radar — *read*, and *seen*

**The art.** The radar's centre is `(320, 396)`, held at the widget's `+0xc` and
`+0x10` (`0x1003f3a5`). Every sprite is blended.

| what | page | source | destination | colour |
|---|---|---|---|---|
| the disc, left half | page6 | (156, 0) 99 × 153 | (320, 327) → (221, 480), mirrored | white |
| the disc, right half | page6 | (155, 0) 100 × 153 | (320, 327) → (421, 480) | white |
| the altitude icon | ui_menu3 | (215, 0) 15 × 28 | (230, 383) → (245, 411) | white |
| the speed icon | ui_menu3 | (199, 0) 15 × 28 | (396, 383) → (411, 411) | white |
| the altitude bar | page6 | (121, e) 31 × (105 − e) | (271, 347 + e) → (240, 452), mirrored | (255, 180, 80) |
| the speed bar | page6 | (121, e) 31 × (105 − e) | (370, 347 + e) → (401, 452) | (255, 180, 80) |
| the view wedge | page7 | a fan about (192, 64), radius 64 | a triangle about the centre, radius 75 | white |
| north and south | page6 | (192, 154) 9 × 11 | four corners on the ring | blue, red |

- **The art is the disc's right half.** Its straight left edge is the disc's
  middle; the left half is the same art mirrored.
- **The disc's art also holds** the boxes for the two figures and the slots for
  the indicators' lamps.
- **`page6`, `page7` and `ui_menu3`** are `ui_tex6.tex`, `ui_tex7.tex` and
  `ui_menu3.tex` (*measured*).

**The order** (`0x1003fb90`):
1. the two halves and the two icons;
2. the altitude (`0x1003f6a0`), then the speed (`0x1003f900`);
3. the view wedge;
4. north and south;
5. the sweep ring;
6. the contacts;
7. the range figure.

**The view wedge is the camera's field of view.**
- **The camera.** `0x1007e6a0` finds the unit's first class-4 component. It
  keeps that component's view (interface 8) at the record's `+0x4c`.
- **The angle.** The view's slot 7 fills a parameter block. Its `+0x14` is the
  camera's value 2, which `Control.dll:0x100238b0` writes there beside values 0
  and 1 at `+0xc` and `+0x10`. That value is 1.3 rad on every camera
  ([30-turrets.md](30-turrets.md)).
- **The shape.** The wedge is one triangle (`0x1003fd40`). Its apex is at the
  centre. Its far corners are 75 from it at π/2 ± half the angle, measured with
  y up, so **the wedge always points up**. Their UVs take the same angles on
  page7's fan, radius 64 about (192, 64).
- **Its width.** A 1.3 rad view makes it 74.5° wide (*derived*).

**The disc turns with the camera** (*read*, `0x1003fe9c`).
- **The heading.** θ is the angle of the first column (x, y) of the camera
  matrix, from the view's interface 6, property 2 (`[+0]`, `[+0x10]`). It is an
  arctangent of y ÷ x, less π when x ≤ 0, so it is `atan2`.
- **North and south.** North is drawn at θ − π/2, clockwise from up; south is
  opposite.
- **A contact.** A contact at bearing β (the angle of the vector to it) is drawn
  at β − θ + π/2 counter-clockwise from the right, with y up
  (`0x100402ef`).
- **Which axis faces** (*seen*). That this column is the direction the camera
  faces is *seen*: at 128 s the hero still faces its targets to the east, and
  the north mark stands on the left.

**North and south** (`0x1003ffa0`, `0x10040078`).
- **The shape.** page6's arrow is drawn on four corners. Its top row is at radius
  68 and its bottom row at radius 56, at its angle ± 5°, so it points outward.
- **The colours.** North is blue (10, 10, 225) and south red (225, 10, 10).

**The contacts** are the entries of the driven unit's target list
([25-sensors.md](25-sensors.md#the-players-target--read-and-measured)).
- **Its record.** Each id is looked up among the game's units (`0x10072d10`),
  then its buildings (`0x100728a0`). Which list is which is *derived* from what
  each branch calls.
- **What is skipped.** An entry with neither record is skipped, and so is one
  whose owner word is `0xfffe` or `0xffff`.
- **The colour.** It is the clan rule's (`0x10065440`).
- **The place.** `d` is the distance across the ground from the translation of
  the unit's own matrix to the record's `+4`, `+8`. The contact goes at
  `60 × d ÷ R` from the centre, where `R` is the sensor range: property `0x50`,
  through `0x10091b30`, and 1 m without a radar. Nothing clamps it. The list
  holds only what the radar found within its range, so a contact lands inside
  the 60-unit disc (*derived*).
- **The marks** are drawn in screen pixels (GUI slot 0 set to 0):

  | mark | when | size |
  |---|---|---|
  | a cross | a unit whose chassis profile's `ChassisType` is 1, a flyer | arms `int(4·sx)` wide and `int(4·sy)` tall, each rounded down to even |
  | a filled square | any other unit | `int(3·sx)` × `int(3·sy)` |
  | a filled square | a building | `int(5·sx)` × `int(5·sy)` |
  | a rectangle outline | the list's current target (`+4`) | from 2 px left of and above the mark to 2 px (the cross) or 1 px (the squares) beyond it, unscaled |
  | a magenta triangle outline | a unit chosen in the wingman selector (`0x1006df50`) | radius `int(5·sx)`, GUI slot 6 |

  - **Which chassis flies.** The test is `0x10075f70`. It reads the unit
    behaviour's variable `0x207`, which `Behavior.dll:0x1000a571` answers with
    the chassis profile at `+0x7c0`; `ChassisType` is that profile's first
    field.
  - **Seen, 150 s.** `helic`, a flyer, is a grey cross.
  - **Seen, 230 s.** Both captured flyers are light-blue crosses and the
    current target is outlined; `tut1_e1` is a red square.
  - **Seen, 128 s.** The target dummies do not show at all.

**The sweep ring and its ping** (`0x10040094`).
- **When it runs.** Only while the unit answers property `0xa9`. The machine
  getter (`Control.dll:0x1000e6c0`) sends that id to the device getter as 9, the
  radar's value 4, its period, via `0x1000e831` ([14-controls.md](14-controls.md)).
- **Its length.** `P` is that period × 0.002 s, so 750 ms gives 1.5 s.
- **The ring.** Each frame a circle of radius `int(60 × t ÷ P)` is drawn about
  the centre, in HUD units, in (0, 155, 0) at alpha 140.
- **The ping.** When `t` passes `P` the stamp restarts. Unless the game object's
  byte `+0xe5` is set, the sound `RADAR` (`i_radar.wav`, `game_resources.cfg`)
  plays.
- **Seen.** `i_radar.wav` matches in the recording's audio at gaps that are
  multiples of 1.51 s, from 110 s to 255 s. The ring is too faint to pick out in
  the video.
- **A correction.** The `RADAR` string at `0x1004011d` is that sound's name, not
  a label.

**The range figure** (`0x1004061d`).
- **The value.** `R`, rounded.
- **The look.** `GAME_FONT`, green (0, 255, 0), at y 467.
- **The place.** It is centred in the 37 units from x 303:
  `x = round(303 + (37 − width ÷ sx) ÷ 2)`, where the width comes from the
  font's slot 6 in pixels.
- **On the hero.** Its radar reads 300 m ([33-units.md](33-units.md)), the
  recording's 300. Mission 03's hero, `tut3_p.dat`, carries a 250 m radar, and
  its recording's range figure reads 250
  ([34-progression.md](34-progression.md#seen-in-a-recording-1)).

**The altitude** (left, `0x1003f6a0`).
- **The figure.** `a = round(z − W)`, in white `GAME_FONT` at y 446, centred in
  the 28 units from x 226.
  - `z` is the unit record's `+0xc`.
  - `W` is the world's `+0xad8`. The world fills it once at load, from the
    landscape's `ITerrain` slot 11 (`iron3d.dll:0x100a1ec6`).
- **What W is.** Slot 11 (`Terrain.dll:0x10019180`) returns the z of the first
  vertex of the first face whose surface has bit `0x02`, water. It is −1 on a
  map with none (`0x10017d60`, `0x10017df7`).
  - Water is one flat plane per map (`check_water`), so **the altitude is
    height above the water** (*derived*).
  - Tut_1's water lies at −1.725 m (*measured*).
- **The bar.**
  - `p = (a + 100) × 100 ÷ 200`, held to 0..100, and
    `e = (100 − p) × 105 ÷ 100`, both in integers.
  - The bar's art starts `e` rows down, so it fills from the bottom: empty at
    100 m below the water, half at the water, full 100 m above.
- **Seen.** 25 falling to 22 while the hero walks down the island at 126–129 s.

**The speed** (right, `0x1003f900`).
- **The figures.**
  - `v = round(speed × 3.6)`, where `speed` is property `0x28`, the length of
    the step velocity ([24-motion.md](24-motion.md)).
  - `top = round(top speed × 3.6)`, where `top speed` is property `0x90`
    (`Control.dll:0x1000dd8c`): the controller block's forward top speed at
    file `+48`, which the stat panel prints as "Max speed".
- **The look.** White `GAME_FONT` at y 446, centred in the 28 units from x 388.
- **The bar.** `p = v × 100 ÷ top` (0 with no top), held to 0..100; then as the
  altitude.
- **Seen.** 50 while the hero runs at 14 m/s (50.4 km/h), and 0 standing.

### The indicators — *read*, and *seen*

Eight slots share one draw (`0x1003f270`), given a slot and a state 0, 1 or 2:

- **The lamp.** page6's (202 + 18 × state, 154), 17 × 20, drawn white and
  blended at `(x, 459) → (x + 18, 484)`. The three lamps are dark, lit white and
  lit red.
- **The icon.** 15 × 15, at `(x + 1, 463) → (x + 16, 478)`. It is tinted grey
  (128, 128, 128) for state 0, white for 1 and red (255, 0, 0) for 2.

| slot | x | icon | state 1 (2 for slot 4) when | key |
|---:|---:|---|---|---|
| 0 | 222 | ui_menu (97, 126), a wrench | the repair system is on: the first class-15 component in state `0x20` (`0x10076e10`) | G, `CMD_REPAIRSYS_ON` |
| 1 | 240 | ui_menu (238, 222) | the camera's infrared is on: the view's flag `0x20` (`0x10035c20`) | N, `CMD_CAMERA_INFRARED` |
| 2 | 258 | ui_menu (239, 126) | camouflage is on: the class-10 component in state `0x1000` (`0x10076e40`) | H, `CMD_CAMOUFLAGE_WEAR` |
| 3 | 280 | ui_menu (223, 142), a figure | the interface's `CState` mode is 0 (`FREE`) or 1 (`SELECT_ATTACK_TARGET`) | — |
| 4 | 344 | ui_menu3 (180, 27) | red for 3 s after *"Risk area! Landing impossible."* | — |
| 5 | 366 | ui_menu (113, 94), a palm | the unit's auto-driver level (`+0x9c`) is 0 | Y, `CMD_JAMES_AUTO_DRIVER` |
| 6 | 384 | ui_menu (113, 110) | the level is 1 | Y |
| 7 | 402 | ui_menu (81, 94) | the level is 2 | Y |

- **Infrared.** The camera class (`Control.dll:0x10023a00`) sets the view's flag
  `0x20` on state `0x1000`, clears it on `0x2000` and flips it on `0x4000`.
  That last is `CIS_INFRARED_INV`, which `hero.tbl` sends on N.
- **The mode.** The `CState` is the game object's `+0x20`, and its mode is the
  front of its stack ([30-turrets.md](30-turrets.md)).
- **The warning.**
  - `0x10063500` shows string 6211 with `VOICE_RISK_AREA`, sets the `CState`'s
    byte `+0x31` and stamps the time.
  - The `CState` takt (`0x10062950`) clears the byte once 3 s have passed.
  - Slot 4 is otherwise grey.
- **The auto-driver.** Y steps the level 0 → 1 → 2 → 0
  ([31-packages.md](31-packages.md)). `ui_other_d.man` puts it on A.
- **Seen.**
  - Grey icons read about 133 in the recording and lit ones 255.
  - Repair (slot 0) and camouflage (slot 2) turn from grey to lit between 112 s
    and 118 s, while the tutorial goes through the battle suit's systems.
  - The figure and the palm stay lit throughout; the other slots stay grey.

### The reticle — *read*

All of it is green (55, 255, 55) and blended (`0x10042ed0`):

| piece | page9 source | destinations |
|---|---|---|
| the circle | (77, 0) 64 × 64 | (288, 208) → (352, 272) |
| the corner arcs | (54, 28) 23 × 23 | (279, 260) → (300, 281); (279, 220) → (300, 199); (361, 260) → (340, 281); (361, 220) → (340, 199) |
| the tick strips | (168, 0) 82 × 15 | (232, 233) → (314, 248); (408, 233) → (326, 248) |

- **The corner arcs** are one art, mirrored into the four corners.
- **The tick strips** are the same art twice, the second mirrored.
- **A dot.** Last comes a 2 × 2 filled green (0, 255, 0) square at
  `(320 − 24·b, 240 − 24·a)`. `a` and `b` are `[+0]` and `[+0x10]` of the
  camera view's interface 6, property 0.

### Not established

- What the camera's property 0 holds, so what the reticle's dot shows.
- In which view states the three widgets are drawn.
- The game object's byte `+0xe5`, which silences the radar's ping.
- Where text's `(x, y)` sits on a glyph: the font's slot 7 takes it as given.
- Slot 30's stage sets 1 and 7.
- What slot 3's `CState` modes 0 and 1 mean to the player.
- Why the target dummies are not on the radar. The list holds radar contacts
  only, so they are presumably never detected (*guess*: a dummy gives off no
  signature).

## The target panel and the player's own unit

Two 150 × 174 panels stand in the bottom corners. The left one shows the
driven unit's current target, the right one the driven unit itself. Both show:

- the unit seen through a camera, each part coloured by its life;
- six shield sectors around it;
- two arcs, its life and its batteries;
- its name.

The target panel also frames the target in the world and prints its distance.
One routine draws both.

What a recording of Mission 01 shows (960 × 720, 1.5 × the HUD's 640 × 480) is
marked *seen*: looked at, not re-derived by `verify`.

### Two widgets, one routine — *read*

The HUD builder (`iron3d.dll:0x100433a0`) makes the two widgets at its `+0x1c`
and `+0x20`, with ids 1 and 2. Each holds a panel object at `+0xc` built by
`0x10040a60`.

**The widgets call the routine `0x10040f30`** with the driven unit's record, a
record to show, and a flag:
- The left widget (`0x10040940`) passes the current target of the unit's list,
  list `+4` ([25-sensors.md](25-sensors.md#the-players-target--read-and-measured)),
  with flag 0.
- The right widget (`0x10040a40`) passes the unit itself, with flag 1.

**The panel object's constructor** (`0x10040a60`) sets up:

| field | what |
|---|---|
| `+4` | the sprite `targeter_range` |
| `+0x90` | the sprite `targeter_back` |
| `+0x11c` | the sprite `targeter_life` |
| `+0x1a8` | the sprite `targeter_energy` |
| `+0x2c0` | the page `ui_menu3`, which the arcs are cut from |
| `+0x2c4` | the frame's clock |
| `+0x2c8` | the shield sectors' object (`0x10042700`) |
| `+0x2cc` | the panel's camera (`0x10036440`) |
| `+0x2d0` | string 6178, *"m"* |
| `+0x2dc`, `+0x2e0` | the low-battery and low-life voice timers |

**How a sprite is loaded and drawn.**
- **A sprite is named** in `ui/hq.cfg`, which the game loads into its `+0xac`
  (`0x1005f720`).
- **Loading** (`0x1008f450`) reads the sprite's `texture`, `offset_x`,
  `offset_y`, `width`, `height` and `rotate`.
  - The code asks for the keys in lower case, and the panel's blocks spell them
    in upper case, so the lookup ignores case (*derived*).
  - A `rotate` of 90, 180 or 270 turns the corners by one, two or three steps.
- **Cutting** (`0x1008f830`) takes the rectangle from (x + 0.5)/256 to
  (x + w)/256, and likewise in v. The 256 is the page size, handed in.
- **Drawing** (`0x1008f970`) takes (x₀, y₀, x₁, y₁, colour, specular, a, b)
  on the 640 × 480 HUD.
  - The corners are scaled by the display's two scale queries (`getDisplay`
    slots 4 and 5), 1.5 each on the recording.
  - The texture is tinted by the colour.
  - Every call here passes a = 1, which draws blend mode 4, over and
    alpha-tested
    ([07-objects.md](07-objects.md#how-a-material-reaches-the-device--read-and-measured)),
    with specular black.
  - **x₀ > x₁ mirrors the sprite.**

### The sprites — *measured*

All nine sprites are `ui/hq.cfg` objects. `ui_menu3` is `ui.lib`'s
`ui_menu3.tex`, and `ui_menu` its `ui_menu1.tex`. `verify` finds each rectangle
on drawn art (28–89% opaque).

| sprite | page | x, y | w × h | what it is |
|---|---|---|---|---|
| `targeter_back` | `ui_menu3` | 0, 82 | 150 × 174 | the panel: the green dial, two lamp sockets, the name box |
| `targeter_range` | `ui_menu3` | 0, 51 | 39 × 16 | the black box the distance is printed in |
| `targeter_life` | `ui_menu` | 49, 94 | 15 × 15 | the icon labelling the life arc |
| `targeter_energy` | `ui_menu` | 113, 126 | 15 × 15 | the icon labelling the energy arc |
| `left_shld` | `ui_menu3` | 192, 82 | 26 × 79 | a side sector, grey |
| `frwd_shld` | `ui_menu3` | 185, 59 | 61 × 19 | the inner top sector |
| `back_shld` | `ui_menu3` | 151, 194 | 89 × 29 | the inner bottom sector |
| `top_shld` | `ui_menu3` | 115, 59 | 69 × 19 | the outer top sector |
| `bott_shld` | `ui_menu3` | 150, 224 | 105 × 31 | the outer bottom sector |
| the arc (cut in code) | `ui_menu3` | 151, 82 | 40 × 94 | a hatched arc, grey, the life and energy bars |

The sector sprites and the arc are grey: the draw tints them.

### What is drawn, in order — *read*, and *measured*

Rectangles are (x₀, y₀)–(x₁, y₁) on the 640 × 480 HUD. The own panel's sits
490 to the right, mirrored where marked.

| # | what | target panel | own panel | colour |
|---|---|---|---|---|
| 1 | `targeter_back` | (0, 306)–(150, 480) | (640, 306)–(490, 480), mirrored | white |
| 2 | `targeter_life` | (129, 411)–(144, 426) | (496, 411)–(511, 426) | white |
| 3 | `targeter_energy` | (5, 411)–(20, 426) | (620, 411)–(635, 426) | white |
| 4 | the six sectors | [below](#shields-six-sectors--read) | | by fill |
| 5 | the frame in the world | [below](#the-frame-around-the-target-in-the-world--read) | — | the mark colour |
| 6 | the unit | the view (9, 315)–(137, 443) | (503, 315)–(631, 443) | by part |
| 7 | the life arc | (149, 314 + t)–(109, 408), mirrored | (491, 314 + t)–(531, 408) | `#19ffaf` |
| 8 | the energy arc | (0, 314 + t)–(40, 408) | (640, 314 + t)–(600, 408), mirrored | `#ffb450` |
| 9 | the name and its status | centred in x 4–142, y 460 and 469 | centred in x 498–636 | grey, or red |
| 10 | `targeter_range` and the distance | (108, 440)–(147, 456); the text at y 444 | — | green |

**Where the drawing stops.**
- Rows 1–3 are drawn every frame.
- **With no target**, or when either record has no object (`+0x3c`), the
  routine returns there (`0x10041177`). *Seen*: from 110 s to 140 s the left
  panel holds only its dial, two icons and an empty name box.
- **When the unit is too big for the view** the routine also returns before
  rows 7–10 (`0x10041902`). The test is: twice the shown object's radius is
  more than the span between the two figures `GetShade()`'s slot 12 hands back,
  its `+4` and `+8`.

The icons mark which arc is which. Life is by the icon at the panel's inner
edge, energy by the one at its outer edge. The own panel mirrors both, so on
each panel the energy arc runs down the screen's outer side (*read*; *seen*).

### Total health: the teal arc — *read*

**The value** is the life percentage (`0x1007e980`), the nearest whole number to
100 × the object's property `0x31`.
- Property `0x31` is the control system's current total life over its total at
  load (`Control.dll:0x1000df55`: `+0x590 ÷ +0x58c`). The load sets both to the
  nodes' summed life (`0x1000fa70`).
- So the arc is the unit's life over its full life, all nodes together
  ([26-damage.md](26-damage.md#hit-points--read-and-measured)).

**How it is drawn** (`0x10041ea0`):
- The percentage p is held to 0–100.
- t = 94 × (100 − p) ÷ 100, in integers.
- The arc sprite is cut again each frame, from (151, 82 + t), 40 × (94 − t).
  It is drawn from y 314 + t down to 408, so it empties from the top.
- The tint is `0xff19ffaf`.

### Battery: the orange arc — *read*

**The value** is the object's property `0x73` (`Control.dll:0x1000e3d1`). That
is the device getter's id 1 on the control system's `+0x38`: the batteries'
fill ([14-controls.md](14-controls.md#the-join-with-the-controller--read)).
- The fill is the sum of every battery's charge (value `0x200`) over the sum of
  their capacities (value 0) (`0x1002b42b`).
- It is 1 if any capacity is negative.
- **With no capacity the query fails**, and the property hands back 0
  (`0x1000dff8`), so a unit with no battery shows **an empty arc** (*derived*).

It is drawn as the life arc is, from 100 × the fill, tinted `0xffffb450`.

### Shields: six sectors — *read*

**The value** is the object's life-system property `0x79` (`0x10042a00`,
through `QueryInterface 0x16`).
- It is the device manager's slot 11 (`Control.dll:0x1002c430`), which writes
  six floats into a static buffer (`0x10043390`).
- It needs a fight shield and a deflector. **Without both it fails and no
  sector is drawn.**
- Sector *i*'s float is its fill (the fight shield's `+0x98 + 4i`) × the
  deflector's level (`+0x4c`) × a figure the deflector's owner answers for its
  node, slot 3 with 1. Taking that figure for the node's condition makes the
  float the share of docs/26's effective strength that the sector keeps
  (*derived*,
  [26-damage.md](26-damage.md#shields-a-generator-a-deflector-six-sectors--read-and-measured)).

**Each sector's colour runs from red to green.** With v = int(255 × fill), the
colour is `0xff000000 | (255 − v) << 16 | v << 8`: red at 0, green when full.

| sector | side | sprite | target panel | own panel |
|---|---|---|---|---|
| 0 | front | `frwd_shld` | (44, 321)–(105, 340) | (535, 321)–(596, 340) |
| 1 | back | `back_shld` | (30, 409)–(119, 438) | (521, 409)–(610, 438) |
| 2 | left | `left_shld` | (16, 334)–(42, 413) | (507, 334)–(533, 413) |
| 3 | right | `left_shld`, mirrored | (133, 334)–(107, 413) | (624, 334)–(598, 413) |
| 4 | top | `top_shld` | (40, 308)–(109, 327) | (531, 308)–(600, 327) |
| 5 | bottom | `bott_shld` | (22, 420)–(127, 451) | (513, 420)–(618, 451) |

- **The own panel is not mirrored:** its sectors are the target's moved 491
  right.
- **The sectors are the unit's own sides.** The target panel draws them the same
  way whichever way the target faces the camera.

*Seen*: full sectors are bright green rings on both panels. At about 250 s the
enemy's top and bottom rings turn red while the four others stay green.

### The unit in the middle — *read*, and *seen*

**Where it is drawn.** The view is (9, 315)–(137, 443) on the target panel and
(503, 315)–(631, 443) on the own, each corner scaled like the sprites
(`0x10041036`, `0x10041107`).

**The camera** is a World3D object of kind 5, made once by
`AddNewObjectToGame` (`0x100367b0`). It is given 300, 0.5 and 1.2, kept at
`+0x88`, `+0x84` and `+0x8c` of the block its slot 6 takes: far, near and
field of view (*guess*). Each frame (`0x100418a3`–`0x10041cc0`):

1. **What it looks at.**
   - *c*, *r* are the centre and radius of the sphere the shown object's
     interface `0x20` answers (slot 3).
   - **The view direction *d*:**
     - On the target panel, the normalised line from the driven unit's sphere
       centre to the target's. A building never looks up: a rising *d* has its
       z zeroed and is normalised again.
     - On the own panel, the unit's own forward axis, its matrix's y column.
       So the own view shows the unit from behind.
2. **Distance and field.**
   - K = 1 ÷ sin 30° = 2 (`0x10040f00`).
   - Normally the camera stands **2r from the centre** and the field is
     **1.25 × 60° = 75°**, so the sphere fills four fifths of the view.
   - If (K − 1)·r falls short of the shade's `+4`, the distance is `+4` + r.
   - If (K + 1)·r passes its `+8`, the distance is `+8` − r.
   - Either way the field is then 1.25 × 2·asin(r ÷ distance).
3. **The eye** is *c* − *d* × distance. The frame's axes are *d*, *s* =
   **Z** × *d* and *d* × *s*, so z is up. A vertical *d* takes fixed axes.

   **The sphere and the matrix are the object's as it is drawn** (*derived*).
   Both come from the object, whose matrix the machine sets every frame
   between its state step's two poses by the step's phase (`Control.dll:0x10015a50`,
   [24-motion.md](24-motion.md#playing-a-state--read-and-measured)). So the
   camera moves with the drawn unit, and a unit running holds still in its
   view, its limbs moving. A camera placed from where a step will end jumps a
   stride at a time: on Mission 01's hero running, up to 0.26 m in a tick.
4. **Drawing.**
   - The view's rectangle goes to the camera (`0x10036d20`) and the device.
   - Stage 0's texture is unset and blend mode 0 set.
   - The z test is on with writes, `D3DRS_ZFUNC` `LESSEQUAL`, for the draw and
     restored after.
   - The shown object's mesh is drawn through interface `0x18` slot 11 with
     **flags `0x7f0`** (`0x10041dd1`). The main camera is made current again
     after.
   - No clear of the view was found. *Seen*: the dial shows through around the
     model.

**Each part is coloured by its life** (`AniMesh.dll:0x10014b30`). Draw flag
`0x200`, one of the seven bits of `0x7f0`, does four things:
- It draws at the level held in `AniMesh.dll:0x100225e8`, 1 in the image.
- It puts the camera in mode 2 (slot 8) for the draw.
- Before each node's batches, it hands the camera (slot 30) the colour
  **(0.5, 0, 0, 1) + life × (−0.5, 0.5, 0, 0)**. Life is the node's life over
  its maximum (node `+0x124`), and the two quads are the mesh's `+0x21c` and
  `+0x22c`, set by its constructor (`0x10006b7a`).
- Afterwards it restores the camera's colour and mode.

So an intact part is green (0, 0.5, 0) and a destroyed one red (0.5, 0, 0). How
the camera applies the colour in mode 2 was not read.

*Seen*:
- **Colours.** An intact dummy is a flat (39, 162, 41); a damaged part of it at
  146 s is (140, 59, 38); a destroyed part goes brownish.
- **Shading.** Bots show light-green shading, the dummy none.
- **Level.** The models are coarse.

### Name and status — *read*, and *seen*

**Both lines print in `GAME_FONT`** (the game's `+0x10`), each centred in 138
from x 4 on the target panel or 498 on the own:
x = int((138 − width ÷ scale) × 0.5 + x₀).

**Line one, at y 460, is the unit's name** in grey `0xffc8c8c8`. The name is
the string the unit's behaviour holds (`IBehaviour` slot 41,
`Behavior.dll:0x1000cb50`, `+0xb04`). `iron3d.dll` names each unit through a
slot of its record (`0x10075d50`):

| the record's Type | name |
|---|---|
| `ROBOT_HERO` `0x1020000` | 6230 *"Human"* |
| `0x20000000`, an animal | 6253 *"Animal"* |
| a robot (`0x1000000` set) whose record's `+0x64` answers its query 2 with 0 or less | 6076 *"Tiny Tower"* |
| anything else | `"%s-%d %s"` |

In `"%s-%d %s"`:
- **The three letters** (`0x10076270`):
  - size, from object property `0x201`: T 1, S 2, M 3, L 4;
  - chassis, from `0x207`: F 1, S 2, W 3, T 4, A 5, U 6;
  - class, from Type: B builder, T transport, W warrior, C HQ, H hero;
  - `?` for anything else.
- **The number** is one more than a count the unit's clan keeps (its clan entry
  `+0x734`), raised as each of its units is named — **every** unit, whichever
  string it gets: all four rows of the table jump to the one `inc` at
  `0x10075eb2`. So the hero's *"Human"* counts in the player's clan, an
  animal's in nature's and a *Tiny Tower*'s in its own.
- **The class word** is handed in. It is one of 6200–6205 *"Transport"*,
  *"Builder"*, *"Warrior"*, *"Comm. Center"*, *"Human"*, *"Unknown"*. **The
  unit record's bind passes it** (*read*): the naming routine is slot 1 of the
  record's vtable (`0x100e64d0` + 4), which the bind calls at `0x10074f2b` with
  the word `0x10076490` gives by Type (`0x10074e88`,
  [38-designs.md](38-designs.md#the-name--read-and-seen)). Each class letter
  takes its own word.

**Which robots are *Tiny Tower*s** (*read*, and *measured*). The record's
`+0x64` is the unit's `IDeviceManager`, which the *"Dangerous!"* test below
reads too, and its slot 4 with `edx` 2 (`0x10075e1e`–`0x10075e2a`)
is the getter's **id 2, the batteries' capacity**
([14-controls.md](14-controls.md#the-join-with-the-controller--read)). Its case
(`Control.dll:0x1002b4e9`) walks the control system's components and, for each
battery (class `0x13`), takes value 0:
- **a value below 0 is answered at once** (`0x1002b519`–`0x1002b524`, stored at
  `0x1002b98c`), true, the negative capacity itself;
- otherwise the sum is answered when it is above 0 (`0x1002b53d`–`0x1002b55e`);
- a sum of 0 or no battery at all answers false (`0x1002b9dc`), and the name
  goes on to `"%s-%d %s"`.

So the query answers 0 or less only for a unit with a **battery of negative
capacity**, which the economy reads as unlimited
([23-economy.md](23-economy.md#a-power-shortage-lowers-efficiency-once-the-batteries-run-down--read-and-measured)).
*Measured* over the install (`openparkan verify`):
- of the **374** robot designs under `UNITS`, **17** carry one, and every one of
  them stands on the chassis `R_B_06`, the *Small Tower* L-22w, whose own
  controller lists an unslotted battery of capacity −1 that no fitted part
  replaces; all 17 are walking warriors (Type `0x1008000`, `ChassisType` 2);
- the control: **355** answer above 0, and **2** answer nothing — the target
  dummies `l_targ` and `M_targ`, with no battery, which the recording names
  *"SSW-1 Warrior"* as the rule says;
- of the **267** robots the 29 missions place, **28** are *Tiny Tower*s, in
  seven missions: *Outflanking Maneuver* (2, `12tower`), `CAMPAIGN.02/Mission.02`
  (2), `CAMPAIGN.03`'s missions 1, 3 and 4 (4, 3 and 7), `CAMPAIGN.04/Mission.02`
  (4) and `Multi.05` (6).

No recording shows one named: *Outflanking Maneuver*'s player never targets its
two towers (every second of its 462 s sampled). What that recording does show
is the count. Its player clan holds the hero and `12wel1`, and the wingman reads
**SWW-2** *"[following]"*: the hero was the clan's first. Its enemy's `12spd1`s
read **TSW-4** to **TSW-6** and its `12wlk1` **SSW-8**, the file's order. The
units the enemy makes in play that it shows read **TSW-12** to **TSW-17**, which
fits the two towers taking 10 and 11; that no unit made in play took them is not
established. *The Field Base* agrees: its builder is **SWB-2** after the hero,
and the five warbots its factory makes **SSW-4** to **SSW-8** after the builder
and the transport.

Mission 01 fits both properties:
- property `0x201` is the unit's size class (`units.Unit.size_class`);
- property `0x207` is its chassis profile's `ChassisType`: 1 flying, 2 walking,
  3 wheeled, 4 tracked.

That gives (*measured*, against what the recording prints):
- `tut1_mf1` MFW, `helic` TFW, `tut1_e1` TSW, the dummies SSW;
- *seen*: **MFW-1**, **TFW-2** (the Ntrl clan's two, in file order), **TSW-1**,
  **SSW-1**.

**A building is named by its Type and its size class** (*read*). The building
record's slot 1 (`0x10033720`, vtable `0x100e5c4c`) calls `0x100338d0` and hands
the string to the behaviour as its name (interface `0x10` slot 42). The routine
asks the behaviour for its Type (slot 14) and for variable `0x201`, the size
class (slot 26), which a building takes from the fourth letter of its root
record's member name: `l` 2, `m` 3, `b` 4, `e` 5
([38-designs.md](38-designs.md#the-catalogue--read-and-measured)). Then:

| Type | size 2 | 3 | 4 | 5 |
|---|---|---|---|---|
| `BUILDING_MINE` `0x80000004` | 6031 *Small Mine* | 6032 *Medium* | 6033 *Large* | — |
| `BUILDING_STORAGE` `0x80000008` | 6036 *Small Warehouse* | 6037 | 6038 | — |
| `BUILDING_INSTITUTE` `0x80000400` | 6046 *Small Res. Center* | 6047 | 6048 | 6049 *Enhanced Res. Center* |
| `BUILDING_PLANT` `0x80000010` | 6051 *Small Factory* | 6052 | 6053 *Large Factory* | — |
| `BUILDING_RUINE` `0x80002000` | 6061 *Ruins* | 6062 | 6063 | 6064 |
| `BUILDING_BRIDGE` `0x80001000` | 6066 *Bridge* | 6067 | 6068 | 6069 |
| `BUILDING_MAINTELEPORT` `0x80000200` | 6071 *Teleport* | 6072 | — | — |

and by Type alone: the generator 6041 *Small Generator*, the outpost
(`0x80000040`) 6056 *Small Outpost*, the towers 6077 *Light Tower* and 6083
*Heavy Tower*, the bunkers 6086, 6092 and 6098 *Small*, *Medium* and *Large
Bunker*. A size the table has no string for (—), or any other Type, is 6205
*Unknown*. The ruins', the bridges' and the teleports' strings read the same at
every size.

*Measured* over the 29 shipped missions: all **167 of 167** placed buildings get
a string, none *Unknown* — 47 Small Generators, 5 each of the three mines, 4
Small Warehouses, 10, 2 and 16 Small, Medium and Large Factories, 6 Small
Outposts, 4, 1 and 4 Small, Medium and Enhanced Res. Centers, 5 Teleports, 19
Bridges, 3 Ruins, 19, 5 and 1 Small, Medium and Large Bunkers, and 6 Light
Towers. The engine's earlier rule, by the letter between the first two
underscores with anything but `m` and `b` taken as small, named the 4 `fr_e_inst`
Small and left the 19 bridges, 3 ruins and 5 teleports unnamed: 31 of the 167.

**Line two, at y 469**, is left empty for the hero (Type, record `+0x2c`) and
for a building (its object's slot 11 answering 3). Otherwise:
- **A unit of the player's clan** (record `+0x24`) gets `"[status]"` in the same
  grey. The status is the head order's string from 6180–6197
  ([31-packages.md](31-packages.md#the-orders--measured)), and *"no order"*
  with none. *Seen*: *"[no order]"*, *"[following]"*, *"[searching]"* under
  MFW-1 once captured.
- **Any other unit** with a component whose value `0x400` is above 0 and whose
  value 6 is at least 10,000 gets 6255 *"Dangerous!"* in red `0xffc80000`
  (`0x100766f0`). The values are `IDeviceManager`'s (the record's `+0x64`,
  slot 6, `Control.dll:0x1002bb40`): `0x400` is the life left in the device's
  node over its maximum, the value the boarding test reads as property `0x52`
  ([39-boarding.md](39-boarding.md#the-turrets-life-is-its-bodys-node-1--read-and-measured)),
  so the device must not be shot off; value 6 is a class-2 gun's `+0x174`, the
  damage of the last round it made
  ([13-control.md](13-control.md#not-established)); heavy rounds are 10,000 or
  more ([29-weapons.md](29-weapons.md)).

### Distance — *read*

**The value** is the straight-line distance in 3D, from the driven unit's sphere
centre (interface `0x18` slot 9) to the target's (a building's through
interface `0x20`) (`0x10041475`).
- The own panel passes 0.
- **Only a distance above 0 is shown.**

**How it is drawn.**
- `targeter_range` goes at (108, 440)–(147, 456).
- The text is the distance as a whole number, a space, and string 6178 *"m"*.
  It is in `GAME_FONT` and green `0xff00ff00`, centred in 31 from x 112, at
  y 444.
- *Seen*: *"55 m"*, *"0 m"* at a few metres.

### The frame around the target in the world — *read*

This refines
[25-sensors.md](25-sensors.md#how-the-game-colours-what-it-marks--read-and-measured).
It is drawn on the target panel only (`0x10041497`–`0x100416f9`).

**Where it is.**
- The target point is projected through the main camera (`0x100cd1a0`), and
  its (x, y) is divided by the display scale.
- **When the projection fails** the frame's clock restarts (`+0x2c4` = now).
- **A target whose owner word reads `0xfffe`** gets no frame.

**How big it is.** The scale is s = (record `+0x10` ÷ `+0x14`) ÷ distance ×
radius:
- × 2 for an object of kind 4, × ⅔ otherwise;
- **s below 0.025 is made 0.1**, and s above 1.1 is held to 1.1;
- the radius is the target's interface `0x20` sphere's, or 0 without one.

**How it is drawn** in the rule's colour, through `getGUIServer` slot 3:
- **Steady:** a square of half-side 200·s about the projection.
- **For its first second** after the clock restarts it eases, with e = 1 −
  elapsed:
  - the centre is the projection moved (320 − x, 240 − y) × e toward the
    screen's middle;
  - the half-side is 200 × (s + (1.1 − s) × e).

  So it closes from a 440-pixel square at (320, 240).

### Voices — *read*

On the own panel only (flag 1), each voice at most once every 20 s by its
timer:
- **Life under 20%** plays `VOICE_LIFE_LOW`, `vc_014.wav`.
- **A battery fill under 0.2** plays `VOICE_BATT_LOW`, `vc_004.wav`.

### Mission 01 — *measured*, and *seen*

| unit | battery | fight shield and deflector | the panel shows |
|---|---|---|---|
| `tut1_p`, the hero | 4,080 | small, 90% | both arcs, six sectors; *"Human"* |
| `tut1_mf1`, `helic`, `tut1_e1` | 12,000, 3,720, 3,720 | medium, tiny, tiny | both arcs, six sectors |
| `l_targ`, `M_targ`, the dummies | none | none | the life arc only, no sectors |

*Seen*: the dummy at 141–148 s has only its teal arc, turns partly red as it is
hit, and prints *"SSW-1 Warrior"* above a falling distance.

### Not established

- How the camera draws a mesh in mode 2 with the colour slot 30 hands it: a
  flat or a lit tint, and why the recording's green is brighter than
  (0, 128, 0).
- The panel camera's three figures (300, 0.5, 1.2) as far, near and field; and
  what `GetShade()` slot 12's `+4` and `+8` are.
- The driven unit record's `+0x10` and `+0x14` in the frame's scale.
- What interface `0x20`'s sphere is, beside interface `0x18`'s bounding sphere.
- ~~Which caller hands the name its class word~~ — **read**: the unit record's
  bind, which calls the name, the record's slot 1, with the word `0x10076490`
  gives by Type (`0x10074e88`, `0x10074f2b`). ~~Which robots are *Tiny
  Tower*s~~ — **read** and **measured**: those whose device manager's id 2, the
  batteries' capacity, answers a negative capacity; 17 of the 374 robot designs,
  all on the Small Tower chassis `R_B_06` and its battery of −1, and 28 of the
  267 placed robots. The name raises its clan's count for every unit, the hero's
  and a *Tiny Tower*'s too
  ([Name and status](#name-and-status--read-and-seen)). Still open: who writes
  the panel level at `AniMesh.dll:0x100225e8`.
- ~~The routine that names a building (strings 6031–6098).~~ **Read**:
  `0x100338d0`, from the building record's slot 1, by the behaviour's Type and
  its size class `0x201`; all 167 placed buildings get a string
  ([Name and status](#name-and-status--read-and-seen)).
- ~~The component value `0x400` in the *"Dangerous!"* test.~~ **Read**: the
  life left in the device's node over its maximum, `IDeviceManager` slot 6's
  case `0x1002bc3c`
  ([Name and status](#name-and-status--read-and-seen)).
- The other six bits of draw flags `0x7f0`.
- Whether the view's begin (interface `0x12` slot 3) clears depth.

## The objectives screen

The mission's objectives, centred over a dimmed view: *"Primary objectives"*
in green, a line an objective with its state, and *"Press F12 to close"*. It
opens by itself when a mission starts and closes 7 seconds later; F12 opens
and closes it at will.

### Who draws it, and what it hides — *read*

**The interface's screens** are one object, the game's `+0x2c`, built as the
mission loads (`0x1008ce90`, called at `0x1005e0f1`):

| field | built by | size | what it is |
|---|---|---|---|
| `+0x04` | `0x100433a0` | 0x15c | the cockpit HUD ([above](#everything-is-drawn-on-a-640--480-screen--read)) |
| `+0x10` | `0x1006a210` | 0x594 | **the objectives screen** |
| `+0x14` | `0x10072f90` | 0x264 | **the satellite map** ([below](#the-satellite-map)) |
| `+0x1c` | `0x100672c0` | 0x290 | the help screen, titled `Help:` |
| `+0x20` | `0x10065230` | 0x460 | the eight clans' signs, 32 × 32 cells of `icons` along y 224, which the unit marker draws ([25-sensors.md](25-sensors.md#the-unit-markers-layout--read-and-measured)) |
| `+0x24` | a byte of its own | 1 | the pause byte Esc clears first ([34-progression.md](34-progression.md#after-the-outcome--read-and-measured)) |

The other fields (`+0x08`, `+0x0c`, `+0x18`) were not read.

**Its draw** (`0x1008d200`) runs in the interface pass while the mission is
being played ([34-progression.md](34-progression.md#after-the-outcome--read-and-measured)).
During a briefing (the level's state word `+0x710` is 5) it draws only the
briefing (`0x1008d319`). Otherwise it switches on the `CState` mode
([30-turrets.md](30-turrets.md)); in modes 0–2 and 6 (`0x1008d37f`) it draws,
in order:

1. the HUD's widgets (`0x10043b20`), when the player drives a unit;
2. the satellite map (`0x10073750`);
3. the message box (`0x1007f4f0`);
4. the game's `+0x30` overlay (its slot 7, not read);
5. the objectives screen (`0x1006a9d0`), which draws nothing unless it is up
   (its byte `+0x592`).

**While the screen is up, it alone is drawn** in those modes: the test of
`+0x592` at `0x1008d3ac` jumps past steps 1–4 (`0x1008d3f6`, `0x1008d411`).
No HUD, no map, no message box. In modes 3–5 (`0x1008d51c`, `0x1008d444`) the
map and the message box still draw, under it.

**What else stops** (*read*):
- the interface's mouse handlers return at once (`0x1008d6ef` in
  `0x1008d690`; `0x1008dad8` in the cursor pick `0x1008da40`);
- the tooltip timer (`0x1009bc20`) is not run (`0x10060c7e`);
- `CMD_JAMES_WINGMAN_MENU` (740) does nothing (`0x1007255b`).

The mission is not paused: the game frame's tests
([34-progression.md](34-progression.md#when-the-mission-handler-runs--read))
do not look at the screen (*derived*). The player's controls go through the
unit's control table and still turn the view (*seen*, 100–106 s). A message
that arrives meanwhile voices at once. Its box is made then and is 20 s old
from that moment, so it shows for what is left of its 20 s once the screen
closes (*derived*; *seen*: a box at 107 s).

### What it shows — *read*, and *seen*

**Built once** (`0x1006a210`):
- the footer from 1017 *"Press %s to close"* and the name of the key bound to
  `CMD_JAMES_MISSION_OBJ`, 731 (`0x1006a2b5`);
- the header 3062 *"Primary objectives"*, and 3061 *"Additional objectives"*.

In a network game (the game's `+0xe4`) the header is 6247 *"Multiplayer
statistics"*, with `ui/hq.cfg`'s `objpanel_icon_hero` and
`objpanel_icon_system` sprites. That game's statistics (`0x1006b470`) are not
read.

**Drawn each frame** (`0x1006a9d0`, the list `0x1006ac10`), on the 640 × 480
screen, all in `MENU_FONT` (the game's `+0x14`) through the GUI server's text
with a shadow (slot 5):

| what | where | colour | text |
|---|---|---|---|
| the dim | a filled rectangle (0, 0)–(640, 480), slot 4, alpha kept | black at `0x99`, 60%; at `0xcc`, 80%, in `CState` modes 3–5 | — |
| the header | y 80 | `0xff00ff00`, green | 3062 |
| each primary objective | y 110 + 20 *i* | by its state | `"%s : %s"`, the objective's text and its state word |
| the bonus header, when there is a bonus objective | y 260 | green | 3061 |
| each bonus objective | y 290 + 20 *i* | by its state | as above |
| the footer | y 450 | white | 1017 with the key |

- **Centred on x 320.** A text starts at 320 − round(*w* ÷ sx × 0.5). *w* is
  the font's width for it in screen pixels (the font's slot 6), and sx is the
  display's horizontal scale
  ([How the radar draws](#how-the-radar-draws--read)). The bonus header
  halves after rounding, 320 − ⌊round(*w* ÷ sx) ÷ 2⌋.
- **The step is a fixed 20**, not the font's height.
- **Which list** an objective is in follows its exempt word: 0 primary, 1
  bonus ([34-progression.md](34-progression.md#objectives-and-the-end-of-a-mission--read-and-measured)).
- **The text is `mission.cfg`'s as written.** Mission 01's carry their own
  numbers: *"1. Destroy all the targets on the island"*.
- **The state word** (`0x1006af90`):

  | state | word | colour |
  |---:|---|---|
  | 1 | 1014 *complete* | `0xffebebeb` |
  | −1 | 1027 *failed* | `0xffff6464` |
  | anything else, 0 open | 1015 *in progress* | `0xff787878` |

*Seen* in the recording at 103 s:
- the header's top is at 81.3, centred on 319.0;
- the three lines' tops are at 110.7, 130.7 and 151.3, centred on 319.7;
- the footer's top is at 451.3;
- the header reads (15, 237, 16) and the footer white;
- the lines' brightest pixels are about 127 grey;
- the view behind is visibly darker than before and after.

### When it opens and closes — *read*, and *seen*

**When a mission starts** (`0x1005e117`), the screen opens and arms its
closing: `+0x592`, `+0x590` and `+0x591` are all set to 1. That needs both:
- the parameter block's `+0x154` set. A load from inside the game clears it
  ([34-progression.md](34-progression.md#after-the-outcome--read-and-measured)),
  so at least a game loaded that way does not open it (*derived*);
- the game's `+0xe5` clear, parameter mode not 3.

**It closes by itself 7 seconds after it is first drawn** (`0x1006ab94`):
1. The first draw with `+0x591` set stamps the time into `+0x58c` and clears
   `+0x591`.
2. Every later draw asks the timer for the seconds since
   ([The message box](#the-message-box--read-and-measured)).
3. Once they pass 7.0 (the float at `0x100e63a8`), `+0x592` and `+0x590` are
   cleared.

The draw does not run during a briefing, so the 7 s run from its end
(*derived*). *Seen*: in the recording the screen is up from 99.55 s to
106.65 s (±0.05), 7.1 s, starting on the first frame after the briefing's fade.

**F12**, `CMD_JAMES_MISSION_OBJ` (731, `0x1007210e`), toggles `+0x592`. It is
refused in `CState` mode 7, while the help screen is up, or while the game is
paused (the screens object's `+0x24`).
- **Closing** clears `+0x590`, which disarms the timer.
- **Opening** leaves `+0x590` as it was, so a screen opened with F12 stays up
  until it is closed (*derived*).

**Esc** (the key-down handler, `0x10070e85`) closes the screen when it is up
and the `CState` mode is not 7. The earlier uses of Esc come first
([34-progression.md](34-progression.md#after-the-outcome--read-and-measured)):
lifting a pause, leaving after an outcome, the help screen, a briefing.

### Not established

- The game's `+0x30` overlay.
- ~~What `CState` mode 7 is.~~ The game menu ([39-boarding.md](39-boarding.md#the-game-view-keeps-a-stack-of-modes--read)).
- The network game's statistics (`0x1006b470`).
- Why the lines read about 127 rather than 120 in the recording (their shadow,
  or the video's compression).

## The satellite map

A 266-unit square at the top right, over where the weapons list was: the
mission's minimap tinted green and see-through, a compass in its corner, and
marks for the player's units and buildings.

### The object — *read*

It is the screens object's `+0x14` (`0x10072f90`, vtable `0x100e64bc`).

**Its rects** (`0x100560b0` builds (x₀, y₀, x₁, y₁)):

| field | rect | used |
|---|---|---|
| `+0x1bc` | (374, 0)–(640, 266) | **in the cockpit** |
| `+0x1f8` | (374, 63)–(640, 329) | in the commander's views |
| `+0x214` | (374, 43)–(640, 350) | around that one, with its title bar |

**The alpha** (`+0x90`) is `iron_3d.ini`'s `[CS]` `MAP_ALPHA`, 128 when it is
absent, held to 30–255. The destructor (`0x10073360`) writes it back. The
shipped `Iron_3D.ini` says 128.

**Loaded with the mission** (`0x10073550`, from `0x1008d5d0`):
- the minimap: the whole 256 × 256 of the texture `mission.cfg`'s `minimap`
  resource names, which the resource manager's slot 3 opens (`0x100736e7`).
  Every map in `ui/minimap.lib` is 256 × 256 (*measured*), and Mission 01's is
  `tut1.tex` ([04-missions.md](04-missions.md));
- `exit_icon` and `map_compass_icon`, from the skin at the game's `+0xac`:
  `ui_menu` (241, 70) 13 × 13, and page6 (132, 106) 21 × 42. That skin is
  `ui/hq.cfg`, the one file that names them (*derived*);
- 5074 *"Satellite map"*, the commander's title.

### Opening it — *read*

- **M**, `CMD_JAMES_SATELLITE_MAP` (739, `0x100724dd`), opens it
  (`0x100740f0` sets `+0x261`) or closes it (`0x10074100` clears `+0x261` and
  `+0x262`). Nothing else is tested.
- **] and [**, `CMD_INC_MAP_ALPHA` (751, `0x10072657`) and `CMD_DEC_MAP_ALPHA`
  (752, `0x1007266b`), work only while it is open:
  - the alpha gains 12 (`0x10074550`) or loses 12 (`0x100745b0`), held to
    30–255;
  - `+0x262` is set and the time stamped, so the alpha's label shows for 1 s.

  So *Increase map transparency*, as `Command.dsc` calls 751, makes the map
  more opaque (*derived*).
- **The weapons list** draws no rows while it is open
  ([The weapons list](#the-weapons-list--read-and-measured)).
- It is also opened and closed from the commander's screens (`0x1002c778`,
  `0x10079993`, `0x10084630`, `0x1008437d`, `0x1008fd18`), and closed by Esc in
  view state 2 (`0x10071027`). `0x1008437d` is the title bar's exit button
  ([The commander's variant](#the-commanders-variant--read-and-seen)) and
  `0x10084630` the column's map button; the rest were not followed.

### The panel in the cockpit — *read*, and *seen*

**Each frame while open** (`0x10073750`):
1. It picks the variant by the level's view state `+0x710`.
   - 1, 3, 4 and 6 are the cockpit's
     ([25-sensors.md](25-sensors.md#how-the-game-colours-what-it-marks--read-and-measured)).
     They take the panel at `+0x1bc` (`0x100737c0`).
   - Any other state takes the commander's (`0x10073830`).
2. It draws the marks (`0x10074220`).

**The panel** (`0x100748a0`), on (x₀, y₀, x₁, y₁) = (374, 0, 640, 266):

1. **The frame**, `0x1009abf0` (the message box's without its fill). With
   `ui/compaund.cfg`'s 8 × 8 corners and 47 × 5 edges, every piece white:

   | piece | from | to |
   |---|---|---|
   | `frame_corner_1` | (x₀, y₀) | (x₀ + 8, y₀ + 8) |
   | `frame_corner_2` | (x₁ − 8, y₀) | (x₁, y₀ + 8) |
   | `frame_corner_3` | (x₁ − 8, y₁ − 8) | (x₁, y₁) |
   | `frame_corner_4` | (x₀, y₁ − 8) | (x₀ + 8, y₁) |
   | `frame_edge_h`, top | (x₀ + 8, y₀) | (x₁ − 8 + 1, y₀ + 5) |
   | `frame_edge_h`, bottom, flipped | (x₀ + 8, y₁) | (x₁ − 8 + 1, y₁ − 5) |
   | `frame_edge_v`, left, mirrored | (x₀ + 5, y₀ + 8) | (x₀, y₁ − 8 + 1) |
   | `frame_edge_v`, right | (x₁ − 5, y₀ + 8) | (x₁, y₁ − 8 + 1) |

   Each row gives the two corners the sprite draw is handed, in its order. A
   "to" left of or above its "from" mirrors the piece that way. The 8s and 5s
   are each sprite's own width and height (`+0x84`, `+0x88`): an edge is as
   thick as its height, `frame_edge_v` included.
2. **The minimap**, on (x₀ + 5, y₀ + 5)–(x₁ − 5, y₁ − 5), that is (379, 5)–(635,
   261): one texel a unit.
   - Its colour is the alpha over `#37ff37`, (alpha << 24) | `0x37ff37`
     (`0x10074940`), with blend 1.
   - The grey image is tinted green and at 128 lets half the view through.
3. **The alpha's label**, while `+0x262` is set (`0x10074640`). See below.
4. **`map_compass_icon`**, white: (x₁ − 5 − 21 − 1, y₁ − 5 − 42 − 1)–(x₁ − 5 −
   1, y₁ − 5 − 1), that is (613, 218)–(634, 260). Its art is a blue triangle
   pointing up over a green ring over a red one pointing down: north up and
   south down, as on the radar.

**The alpha's label** (`0x10074640`), in `GAME_FONT` (the game's `+0x10`):
- **The text.** alpha × 100 ÷ 255, rounded down to a multiple of 5, then `%`.
- **The place.** x = x₀ + 5 + 6. The top is y₁ − 5 − 6 − (the font's height +
  1) ÷ sy, rounded.
- **The box behind it**, filled `0xff007300`: from 3 left of and 3 above the
  text, to 2 + its width ÷ sx right of x and 4 + its height ÷ sy below its top.
- **The text colour** is `#37ff37`.
- **It hides** once 1.0 s has passed since the alpha changed (`+0x25c`).

**Where a point goes** (`0x100741a0`). The world (x, y), rounded to whole
units, goes to (x₀ + 5 + round(256 x ÷ L), y₁ − 5 − round(256 y ÷ L)).
- *L* is the float at `+8` of the level's `+0x718` record. The map-edge test
  compares positions with the same float (`0x10035eeb`), so it is the map's
  side (*derived*).
- **The whole map, north up.** There is no zoom or turning.
- On Tut_1, *L* is its terrain's side, 1746.59 (*derived*: the minimaps cover
  their terrain's full extent,
  [03-terrain.md](03-terrain.md#the-strongest-check-the-games-own-art)).
- The inverse (`0x10074110`) serves a click on the map
  (`0x10073fd0`, `0x1008dbd8`).

**The marks** (`0x10074220`), in this order:

1. **Every entry of the level's `+0x700` list** (`0x10081a70`): a 3 × 3 filled
   square, (x − 1, y − 1)–(x + 2, y + 2), in `0xff646400`. The entry's
   (x, y) is at `+0`, `+4`, and it must answer non-zero at `+0xc`. Ore
   deposits, by the colour (*guess*); Mission 01 has none.
2. **Every building** (`+0x71c`, `0x100347f0`): an icon by its type, drawn 20 ×
   20 about its place, (x − 10, y − 10)–(x + 10, y + 10).
   - **The colour** is the clan rule's, or white when the player's own
     building is selected (`+0x80`).
   - **The icons** are 24 × 24 cells of the `icons` page (`0x10064f10`; type to
     index `0x1009f4c0`):

     | type (`varset.var`) | icon cell | the picture (*seen*, `ui/ui.lib`'s `icons.tex`) |
     |---|---|---|
     | `BUILDING_GENERATOR` `0x80000002` | (0, 24) | a capacitor with a plus |
     | `BUILDING_MINE` `0x80000004` | (24, 24) | a shovel and a pick, crossed |
     | `BUILDING_STORAGE` `0x80000008` | (48, 0) | stacked crates |
     | `BUILDING_PLANT` `0x80000010` | (72, 24) | two gears |
     | `BUILDING_BUNKER_SMALL`, `_MEDIUM`, `_LARGE` | (96, 24) | a flag |
     | `BUILDING_HANGAR` `0x80000040` | (120, 24) | a wrench |
     | `BUILDING_INSTITUTE` `0x80000400` | (48, 24) | a pair of compasses |
     | `BUILDING_TOWER_MEDIUM`, `_LARGE` | (168, 24) | a tower |
     | `BUILDING_MAINTELEPORT` `0x80000200` | (216, 24) | an arrow up over bars |
     | anything else, the bridge and the ruin included | no mark | |

     The cells are white art; the colour tints them. They are drawn with
     `0xff000000` as the specular and blend 1 (`0x100348ca`).

3. **Every unit** (`+0x720`, `0x10077690`), in screen pixels (GUI slot 0 set to
   0): its map point (x, y) goes to (round(x · sx), round(y · sy)).
   - **A flyer** (the `0x207` test, as on the radar) is a cross in the clan
     rule's colour. Its arms are 4 · sx wide and 4 · sy tall, each rounded and
     then made even.
   - **Any other unit** is a filled square 3 · sx by 3 · sy, rounded, from
     half its size left of and above the point.
   - **A selected unit** (`+0x80`):
     - a white outline, from 2 px left of and above the mark to 2 px (the
       cross) or 1 px (the square) beyond it;
     - a white line from the mark's centre, 1.8 × its size along the record's
       (`+0xd8`, −`+0xdc`). Those are the x and y of the second column of the
       unit's world matrix (`+4`, `+0x14` of interface `+0x3c` slot 8, kind 2),
       its local +y axis, which is its forward. The record's bind copies them
       (`0x10074e27`–`0x10074e33`), and so does its takt each time it runs
       (`0x10075761`–`0x1007576d`), with the position beside them;
     - its route, lines in `0xff64c864` with 3 × 3 squares in `0xffc8c864` at
       the points, starting from the unit.
   - **The hero** (`Type` `0x1020000`), when it is not a flyer, gets the
     outline and the line even when not selected, then in green `0xff00ff00`.
4. **In view states 2 and 4 only**, the camera, in yellow `0xffffff00`: a
   square (x ± 2, y ± 2) with its two diagonals, and a tick from 2 to 7 units
   out along its view.

**Which get a mark.** The unit or building must be the player's clan's, or
**on a contact list of one of the player's units this frame** (`0x1007e660`,
*read*):

- **The test.** It passes at once when the record's owner (`+0x24`) is the
  player's clan. Otherwise it looks for the record's object id (`+0x28`, taken
  from `IGameObject` slot 9 as the record binds, `0x1007e51f`) in the player
  clan record's id list, `+0x54`–`+0x58` (`0x10039370`).
- **Who fills the list.** Every unit record's takt runs its target list's takt
  ([25-sensors.md](25-sensors.md#the-players-target--read-and-measured)).
  That rebuilds the list from the unit's radar contacts. Then, for every unit
  and not only the driven one (`0x10090c09`), it appends each listed id that
  is not yet there to the list of **the unit's own clan** (`0x10090da8` →
  `0x100393a0`).
- **Who empties it.** The game's run loop, which pumps the window messages
  and runs until its state is 3 (back-edge `0x1005ef9e`), empties every clan's
  list on each pass (`0x1005e780` → `0x10039410`).
- **So** another clan's building shows while a unit of the player's clan has
  it in radar range, and goes when none does (*derived*). That the refill runs
  between the emptying and the draw is *derived* from the recording, which
  shows other clans' buildings.

An exception runs when the game's `+0xea` byte is set and `0x1005a850`
answers; that was not read. Also skipped:
- an owner word of `0xfffe`;
- a unit whose `+0x3c` object answers 0 at slot 3;
- a hostile clan's unit whose first order is `0x13`, `ORDER_ROBOT_SHUTDOWN`.

*Seen*, the recording at 118.5 s, on 960 × 720:
- **The frame** is dark from x 561 to 566, and the green starts at 568 (374 to
  377 and 379 on the layout). The right edge is dark from 954 to 958, the top
  from 0 to 5, the bottom from 394 to 397.
- **The compass.** Its blue triangle lies in (620–627, 224–231) and its red in
  (620–626, 248–254), inside the icon's (613, 218)–(634, 260).
- **One mark** at (453, 192): a light-blue square in a green outline with a
  short tick. That is the hero, the one unit of the player's clan; Mission 01's
  bridges take no icon.
- **The green over the sky** reads (126, 224, 141). Half of `#37ff37` over a
  near-white texel, plus half the sky (192, 192, 224), gives (122, 216, 138),
  blended in display space as the HUD's art is (*derived*).

*Seen*, the recording of *Teleport* (Mission 04, `Tut_4`, L = 1697), on both
variants:

- **At 84 s**, in the commander's map, the only marks are at the player's
  start, in light blue: the generator's capacitor, the HQ's square (outlined
  in white, being selected) and the helicopter's small cross, with the yellow
  camera. There is nothing yet at the
  neutral factory (778, 990) or research centre (960, 874).
- **As the helicopter nears them**, their icons appear in grey (160, 160,
  160), the neutral clan's colour: the factory's gears from 120 s and the
  research centre's compasses from 128 s. Each lies within about 10 m of its
  building's place.
- **At 144 s the research centre's icon is gone**, with the helicopter about
  270 m from it, beyond its 250 m radar. At 136 s it had been about 240 m away.
- **The factory's icon turns light blue** between 175.0 and 175.5 s, as
  *"Building is captured"* shows.
- **In the cockpit's map at 362 s** it is white, while the player has the
  factory selected at its screen.
- **The teleport's arrow** appears in grey at 382 s, once the walking hero has
  come near.

### The commander's variant — *read*, and *seen*

In view state 2, the command camera's (any state but the cockpit's 1, 3, 4 and
6), the map is `0x10073830`'s: a title bar, the panel on
`+0x1f8`, (374, 63)–(640, 329), with its marks and the camera, and a line under
it. The pieces are the pen primitives'
([Everything is drawn on a 640 × 480 screen](#everything-is-drawn-on-a-640--480-screen--read)),
and their sizes `ui/compaund.cfg`'s: `ending_text` 5 × 19, the exit buttons
35 × 19.

**The title bar**, the pen from (374, 43) (`0x1007390c`, `0x10073914`):

| x | piece | what |
|---|---|---|
| 374–379 | `ending_text` (`0x10099a30`) | |
| 379–605 | a text box 226 wide (`0x10099f60`) | 5074 *Satellite map* centred in `GAME_FONT`, in `#37ff37` (`0x10073921`) |
| 604–610 | `ending_text` again, mirrored | the pen turned right to left (`0x100999b0` with 1), moved 5 on, and the end drawn back from 610; then turned back (`0x1007393f`–`0x10073963`) |
| 605–640 | the exit button (`0x1009a260`), variant 1 | `exit_button_normal`, and `exit_icon` 13 × 13 at (+15, +3), (620, 46)–(633, 59), tinted `0xfff0f0f0` |

- **The exit button's rectangle is the map's `+0x230`.** The draw stores it
  from the pen as it reaches the button (`0x10073968`–`0x1007399b`): x from
  605, y from 43, to 35 right and 20 down, (605, 43)–(640, 63). Its fields are
  `+0x238`, `+0x23c`, `+0x244` and `+0x248` of an object whose vtable
  (`0x100e42f0`) the constructor sets at `+0x230` (`0x10073128`).
  - **A click in it closes the map.** The commander's column click tests it
    second, after the lock, while the map is open (`+0x261`), with the
    object's half-open rectangle test (`0x10035bd0`), and closes the map
    (`0x10074100`, `0x10084343`–`0x1008437d`,
    [41-commander.md](41-commander.md#what-a-click-on-the-column-does)).
  - **The cursor on it shows *Close***: the draw hands the tooltip manager
    (`0x1009bbc0`) the map's `+0x24c` (`0x100739a6`–`0x100739d9`), a tooltip
    the constructor made with 6169 *Close* (`0x10073196`–`0x100731bc`).
  - In the map's own code (`0x10072f90`–`0x10074a00`) the draw is the only
    writer of its corners, so the click can answer only once the commander's
    variant has drawn (*derived*).
- **The mirrored end caps the box.** The button is drawn after it, over all
  but its first column (*derived*).
- **The button's variant sets its lamp and its icon's tint.** The primitive
  tints the icon `0xff808080` for variant 0, `0xfff0f0f0` for 1, white for 2
  (`0x1009a29b`–`0x1009a2bb`). A page's header draws the same button with
  variant 2, `_pressed` and a white icon, and no mirrored end: its box runs
  straight into the button ([36-factory.md](36-factory.md#the-page-header)).

**The line under the map**, the pen from (374, 330) (`0x10073a1e`, `0x10073c38`,
`0x10073e15`). It names **the object under the cursor, when the cursor's pick
came from the map**: the cursor object at game `+0x24` with its `+0x1c` set
([42-selection.md](42-selection.md#the-pick-under-the-cursor--read)).

**Its clan colour is the player's word towards the object's clan** (*read*).
The two tests are the clan record's relation helpers (`0x10039440`,
`0x10039460`), called on the player's clan record with the object's owner
(`0x10073a36`–`0x10073a54`). Each asks the record's SuperAI (`+0x50`) slot 8 for
its word towards that clan
([25-sensors.md](25-sensors.md#clan-relations-the-files-words-straight-through--read-and-measured))
and answers whether it is 0 and whether it is 1:

| the player's word towards the object's clan | colour |
|---|---|
| 0, hostile (`0x10039440`) | red `0xffff0000` |
| 1, neutral (`0x10039460`) | grey `0xff808080`, from `0xffffff81 + 0xff8080ff` (`0x10073a59`–`0x10073a60`) |
| anything else: 2, allied, and the player's own clan, which the loader gives 2 towards itself | light blue `0xff8080ff` |

Unlike the marks' colour rule
([25-sensors.md](25-sensors.md#how-the-game-colours-what-it-marks--read-and-measured))
it asks no clan type, so a neutral clan is grey by its word, 1 towards everyone.

**Its sign is the clan's emblem** (*read*). The table at the game's `+0x2c` →
`+0x20` is the screens object's eight clans' signs, 32 × 32 cells of `icons`
along y 224 ([Who draws it](#who-draws-it-and-what-it-hides--read)), and the
line takes entry `+0x14` of the owner's clan record (`0x738` − `0x724`), each
0x8c bytes, a sprite (`0x10073a80`–`0x10073a97`). That is the index the unit
markers draw their sign by, which a single-player mission sets to the clan's own
([25-sensors.md](25-sensors.md#the-unit-markers-layout--read-and-measured)).
The line asks no clan type for it either.

**An icon piece** (`0x1009a7a0`) is a `body_text` square as wide as the piece is
tall, its `+0x88`, 19: the piece is drawn white from the pen to 19 on, its
sprite from 2 in to 2 short of the far corner, 15 × 15, in the colour handed in,
and the pen moves 19 (`0x1009a827`–`0x1009a8db`). A 32 × 32 sign is so drawn at
15 × 15.

| the object | pieces, left to right |
|---|---|
| a building (node kind 3), not the ruin `0x80002000` | `ending_text`; an icon piece with the clan's sign in the clan colour; an icon piece with the building's icon, tinted by its size class (`0x100344e0`, [41-commander.md](41-commander.md#the-building-pages-5-to-8--read-and-seen)); an empty text box 19 wide; the end mirrored back from the pen over the box's last 6 (`0x10073b4a`–`0x10073b6d`); `ray_emitter_off`; a bar 183 wide with its name (interface `0x10` slot 41) over its life (`0x1007e980`), the text white; `ray_ending` |
| a unit, not an animal | the same, with its two icons (`0x10077120`) in place of the building's and no empty box; the end drawn back from 5 past the pen (`0x10073d7b`–`0x10073d9c`), under the emitter; and in the bar `"%s [%s]"` of its name and its head order's status (`0x10076f90`) |
| nothing, an animal or a ruin | `ending_text`, an empty text box 57 wide, the mirrored end, `ray_emitter_off`, an empty bar 183 wide, `ray_ending` |

Every kind puts its emitter at x 436, 374 + 5 + 3 × 19 or 374 + 5 + 57, and its
bar from 446 to 629 (*derived*).

So the line is not the selection's. It is a readout of what the pointer rests
on over the map (*derived*).

*Seen*, Mission 04's recording at 84 s, on 960 × 720 divided by 1.5, with the
map open over the Battle units page:
- **The title.** *Satellite map* in green over x 458.7–524.0: its middle is
  491.3 against the box's 492. It sits on a black box that ends at about 606.
- **The exit button.** A green lamp at about 612–615, and the icon's white at
  621–630.
- **The page header's exit** at 340–352 has a white lamp: the pressed variant.
- **The line under the map** is the empty one. A dark box runs from 375 to
  433, the mirrored end to about 437, the emitter to 446, and an empty bar from
  446. Nothing lies under the cursor on the map.

*Seen*, *The Field Base* at 421 s, the cursor over the base's marks at the top of
the commander's map, on 960 × 720 divided by 1.5:
- **The line reads *"SFW-2 Warrior [patrolling]"*** on a green bar, the enemy's
  second flyer, `Enm` towards whom the player's word is 0.
- **The sign** is a red emblem over x 382.7–394.7, inside the piece's
  15-wide sprite at 381–396: red, the hostile word's colour.
- **The two icons** are pale red, a small unit's `0xffff8080`: the warrior's
  crossed swords over 401.3–414.0 and the flyer's over 420.0–433.3, against
  400–415 and 419–434 as read.
- **The dark box ends at 435.3**, the emitter runs to about 446, and the bar's
  green starts at 446.7: the 436 and 446 above.

From 417 s to 420 s the same line is the empty one.

### Not established

- What the level's `+0x700` list holds. ~~The player clan record's `+0x54`
  list~~ — **read**: the ids on its units' contact lists this frame
  ([The panel in the cockpit](#the-panel-in-the-cockpit--read-and-seen)).
- ~~That `+0xd8` and `+0xdc` are the unit's heading (*guess*, from the line's
  use).~~ — **read**: the x and y of its world matrix's second column, its
  forward axis, copied by the record's bind and each takt
  (`0x10074e27`, `0x10075761`–`0x1007576d`,
  [The marks](#the-panel-in-the-cockpit--read-and-seen)).
- ~~The commander's variant (`0x10073830`): its title bar at (374, 43), its exit
  icon and the selected unit's line at y 330, beyond their places.~~ —
  **read**: an `ending_text`, a 226-wide box with 5074 in `#37ff37`, a mirrored
  end and the exit button in its `normal` variant, whose rectangle (605, 43)–(640,
  63) is the `+0x230` the column's click closes the map on. The line at y 330
  names what the cursor points at on the map, not the selection
  ([The commander's variant](#the-commanders-variant--read-and-seen)).
- ~~The clan sprite table the line under the commander's map draws from (the
  game's `+0x2c` → `+0x20`), and the relations `0x10039440` and `0x10039460`
  answer.~~ — **read**: the table is the screens object's eight clans' signs,
  indexed by the clan record's `+0x14`, the unit markers' emblem; the two
  helpers answer whether the player's clan's SuperAI holds word 0 or word 1
  towards the object's clan, so the sign is red for a hostile clan, grey for a
  neutral one and light blue for the player's own or an ally. *Seen* at 421 s of
  *The Field Base*: a red sign and two pale red icons before *"SFW-2 Warrior
  [patrolling]"* ([The commander's variant](#the-commanders-variant--read-and-seen)).
- The blinking square of game mode 8 (`0x10074430`), and the game's `+0xea`
  byte.
- The map's other openers and closers.
