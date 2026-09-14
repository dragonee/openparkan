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

To be written.
