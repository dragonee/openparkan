# The cockpit HUD — the panels, the radar, the weapons and the messages

What the player sees over the world while driving a unit: the weapons list at
the top right, the message box at the top, the radar in the middle at the
bottom, the target panel at the bottom left and the player's own unit at the
bottom right. This page reads how `iron3d.dll` builds and draws them, and
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
| `+0x14` | `0x10042d10` | 0x1b0 | not read | the reticle: page9's circle (77, 0) 64 × 64, corner arc (54, 28) 23 × 23 and ruler (168, 0) 82 × 15, drawn about (320, 240) in `#37ff37` |
| `+0x10` | `0x1003ed60` | 0x610 | 5 | not triaged here (page6, `ui_menu`, `ui_menu3`) |
| `+0x0c` | holder, then `0x1009c9f0` | 0x10 + 0x5a0 | 6 | **the weapons list** and the guided lock ([25-sensors.md](25-sensors.md#how-the-game-colours-what-it-marks--read-and-measured)) |
| `+0x18` | `0x1003f340` | 0x3c8 | not read | the radar (draw `0x1003fb90`) |
| `+0x1c` | `0x10040a60` | 0x2f0 | 1 | the target panel |
| `+0x20` | `0x10040a60` | 0x2f0 | 2 | the player's own unit |
| `+0x24` | `0x1003eb30` | 0x314 | 10 | the wingman panel: 16 lines 19 apart ([31-packages.md](31-packages.md#the-wingman-menu-from-first-person--read-and-measured)) |
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
  slots 4 and 5, one per axis. Text widths are divided by the same two. What
  the queries return is not read.

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
  looks a name up from the component through `0x1008a470` and `0x1008a4b0`
  (not followed), and falls back to *NONAME*.

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
  read *INF* when the first word of the game's `+0x20` list is 6, which is not
  identified.
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
  - `frame_edge_h` and `_v` (47 × 5, stretched) along the sides between them;
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

- What the display object's two scale queries return, and so how the
  640 × 480 screen maps to a larger one. Text is measured in the device's
  pixels and divided back.
- The game's `+0x20` list: its first word 6 turns every rounds count to *INF*,
  and 5 moves the message box. Neither value is named.
- How a non-hero unit's gun gets its name (`0x1008a470`, `0x1008a4b0`).
- The widget at the HUD's `+0x10` (`0x1003ed60`, id 5).

## The radar and the indicators below it

To be written.

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
  `+0x734`), raised as each of its units is named.
- **The class word** is handed in. It is one of 6200–6205 *"Transport"*,
  *"Builder"*, *"Warrior"*, *"Comm. Center"*, *"Human"*, *"Unknown"*; which
  caller passes which was not followed.

Mission 01 fits both properties:
- property `0x201` is the unit's size class (`units.Unit.size_class`);
- property `0x207` is its chassis profile's `ChassisType`: 1 flying, 2 walking,
  3 wheeled, 4 tracked.

That gives (*measured*, against what the recording prints):
- `tut1_mf1` MFW, `helic` TFW, `tut1_e1` TSW, the dummies SSW;
- *seen*: **MFW-1**, **TFW-2** (the Ntrl clan's two, in file order), **TSW-1**,
  **SSW-1**.

**Line two, at y 469**, is left empty for the hero (Type, record `+0x2c`) and
for a building (its object's slot 11 answering 3). Otherwise:
- **A unit of the player's clan** (record `+0x24`) gets `"[status]"` in the same
  grey. The status is the head order's string from 6180–6197
  ([31-packages.md](31-packages.md#the-orders--measured)), and *"no order"*
  with none. *Seen*: *"[no order]"*, *"[following]"*, *"[searching]"* under
  MFW-1 once captured.
- **Any other unit** with a component whose value `0x400` is above 0 and whose
  value 6 is at least 10,000 gets 6255 *"Dangerous!"* in red `0xffc80000`
  (`0x100766f0`). Value 6 is a gun's round damage; heavy rounds are 10,000 or
  more
  ([29-weapons.md](29-weapons.md)).

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
- Which caller hands the name its class word, and who writes the panel level at
  `AniMesh.dll:0x100225e8`.
- The component value `0x400` in the *"Dangerous!"* test.
- The other six bits of draw flags `0x7f0`.
- Whether the view's begin (interface `0x12` slot 3) clears depth.
