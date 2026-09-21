# RsLi — `gamefont.rlb` and `sprites.lib`

Two files out of the 118 the game ships are not [NRes](01-nres.md), and between
them they hold a whole 2D half of it: the cockpit, the interface art, the
cursors, the logo, two geyser animations, and a font. The engine calls the
format **RsLi** — `Ngi32.dll` compares an in-memory copy against the literal
`'RsLi'` — and its loader is at `0x100194cd` in that DLL.

They are also, it turns out, the software renderer's 2D half, and
**[nothing in the shipped game opens either of them](#neither-archive-is-opened-by-the-shipped-game--measured)**.
The font the game draws with is `sys.lib`'s, and the interface's are
`ui/font.lib`'s; what is read here about how a glyph is drawn was checked
against all eleven.

Every claim below about what is *in* these files — the header, the cipher, the
storage, the eleven fonts' metrics and their code pages — is re-derived by
`uv run openparkan verify`. The claims about what the engine *does* with them
carry the addresses they came out of.

## Header, 32 bytes

```
0x00  char[2]  'NL'
0x02  uint8    0            reserved
0x03  uint8    1            version
0x04  int16    count        entries
0x06  int16    count        the same value again
0x0E  uint16   0xABBA       set when the entry table is already sorted
0x10  uint32   total        the unpacked sizes summed
0x14  uint32   seed         the entry table's cipher key
0x18  uint32   zero, twice
```

The loader checks the first four bytes exactly as written here — `'NL'`, then
a zero, then a 1 — and `count` twice over. `total` is confirmed: the members'
declared sizes sum to it exactly on both files, 87096 and 1573632.

## The entry table is encrypted

This is what makes these two archives look like noise, and it is why the
"payload" appeared to start in the middle of nothing. `count` entries of 32
bytes follow the header, and every byte of them is XORed with a keystream
generated from two bytes of state:

```
a = seed & 0xFF
d = (seed >> 8) & 0xFF

for each byte of the table:
    a = ((a << 1) & 0xFF) ^ d
    d >>= 1
    plain = cipher ^ a
    d ^= a
```

The state carries across record boundaries without resetting, so nothing short
of the whole table decrypts — and a partial guess gives you nothing to check
against. Decrypted, an entry is:

```
0x00  char[12] name         uppercase ASCII, NUL-padded
0x0C  uint8[4] unread       the loader steps over it
0x10  int16    flags        storage method
0x12  int16    order        this entry's place before the table was sorted
0x14  uint32   unpacked size
0x18  uint32   offset from the start of the file
0x1C  uint32   packed size
```

and the names come out at once:

| archive | members |
|---|---|
| `gamefont.rlb` | `PAL.PAL`, `ARIALTEX.TFT` |
| `sprites.lib` | `COCKPIT.TEX`, `LOGO.TEX`, `SPRITES.TEX`, `SELECTS.TEX`, `INTERF1`–`INTERF8.TEX`, `GEIZER1`–`GEIZER6.TEX`, `2GEIZER1`–`2GEIZER6.TEX` |

## Storage

`flags` picks how a member is packed. The engine defines seven methods — raw,
a byte transform, LZSS, transform + LZSS, adaptive Huffman + LZSS, transform +
Huffman + LZSS, and raw Deflate — and the shipped data uses two: all 24
sprites are `0x100` raw Deflate, and the font's pair are `0x040` LZSS.

**Both are read.** All 24 sprites inflate to exactly the size their entry
declares, and both font members unpack through the LZSS to exactly theirs.

`sprites.lib::INTERF8.TEX` declares one byte more than the file holds; the
deflate stream ends before it, so a short read is harmless. fparkan
[documents the same quirk](https://fparkan.popov.link/reference/rsli/), which
is a pleasing independent confirmation that the table decrypts correctly.

### The LZSS, and why the first attempt was wrong

The bit packing was right the first time: a flag byte, eight items, least
significant bit first, a set bit a literal and a clear bit a two-byte match
whose low nibble of the high byte plus three gives the length and whose
remaining twelve bits give the offset. What was wrong was what the offset
*means*.

**It is an absolute index into the ring buffer, not a distance back.** Both
the copy source and the write position then walk forward through the ring,
each masked to 4096. And the ring is not empty when a member starts: it is
**pre-filled with spaces** and the write position starts at **0xFEE** —
4096 − 18, the classic `N − F` of Okumura's LZSS.

Miss any one of those three and a member still decodes for a few kilobytes
before drifting, which is exactly what made the first attempt convincing:
it reproduced `ARIALTEX.TFT`'s `Tfnt` header and landed a `Texm` magic at
4116, where arithmetic says one belongs, and only then began emitting
maximum-length matches from the wrong place. The size check could not catch
it because the output is truncated to the declared length and then trivially
matches it. **A size check on a truncated decode proves nothing** — what
settled it this time was decoding the atlas and looking at it.

The routine came out of `Ngi32.dll`, and the reason the first search missed
it is worth recording. `rsLoadFast` tests `flags & 0x1e0` — a **mask over the
four "packed" bits** — and hands anything set to `rsLoad`, which switches on
the same masked value: 0 raw, 0x20 transform, 0x40 the plain LZSS, 0x60
transform + LZSS, and above that the Huffman variants. The earlier sweep
looked for the seven storage constants one at a time and never for the mask
over them. The same decoder handles `0x080` by driving the ring through an
adaptive Huffman tree of 627 nodes and starting it at 0xFC4 instead; nothing
ships that way, so `openparkan` does not implement it.

## What is inside

Every one of the 24 sprite members is an ordinary [`Texm`](02-texm.md) once
unpacked — ARGB4444 at 64 × 64, 128 × 128 or 256 × 256 — so it goes straight
through the existing decoder:

```
uv run openparkan ls sprites.lib
uv run openparkan textures sprites.lib --out /tmp/ui --alpha
```

`COCKPIT.TEX` is the radar screen and reticle; `INTERF1.TEX` is window frames,
buttons, the cursor arrow and the check and cross icons.

`gamefont.rlb`'s two are the font and its palette, and both open now.

**`ARIALTEX.TFT`** is a `Tfnt`: a 20-byte header, 256 glyph records of 16
bytes, and an ordinary `Texm` at 4116. A record is `u0`, `u1`, `v0` as texture
coordinates and an `int32` advance, and the two agree — on **all 123** glyphs
the font actually draws, the span `(u1 − u0) × 128` is exactly `advance + 1`.
The other 133 records are a placeholder with an advance of 8. The glyphs sit
in **seven rows 18 pixels apart**, Latin and Cyrillic.

Its atlas is **pixel format 2**, which appears nowhere else in the game: one
byte per pixel, indexing an *external* palette rather than carrying one.

**`PAL.PAL`** is that palette, and it says so. 1024 bytes of BGRA with the
fourth byte always zero, then the literal tag **`Ipol`**, then a 256 × 256
byte table. The table is **symmetric on all 65536 cells** and `table[i][i] ==
i` on 237 of 256 — the other 19 being indices the palette never uses. It is an
interpolation table: given two palette indices it returns the index of their
mixture, which is how an 8-bit renderer blends. Every lit pixel of the font is
index 73, and index 73 is `(255, 255, 255)`, so the glyphs are white and the
engine tints them — and the atlas has **only those two indices**, 2575 pixels
of 73 against 13809 of 0.

```
uv run openparkan font --out font.png
uv run openparkan font --glyphs
```

## Neither archive is opened by the shipped game — *measured*

This is the first thing to know about both files, and it took until the text
pass to notice. **The byte string `gamefont` occurs in no file of the install,
and neither does `sprites`.**

The control is the same search over the other twenty shipped archives, and
every one of them is named somewhere: `Textures.lib` in five files,
`objects.rlb` in 508, `sounds.lib` in 36 mission configs, `Material.lib` in
four, down to `parts.rlb`, `intsys.rlb` and `turrets.rlb`, which only
`objects.rlb` names. `ARIALTEX.TFT` itself is named — in `iron3d.dll` and in
`sys.lib` — but the archive it is fetched from is `sys.lib`, at
`iron3d.dll:0x1005ca97`.

What these two are is the **software renderer's** copy of the 2D half, and the
shipped `Ngi32.dll` has no software renderer. **79 of its 145 exports are dead
stubs**, each a `ret imm16` (some with an `xor eax, eax` first) sized to its own
argument list and folded by the linker onto one of fourteen addresses between
`0x10002590` and `0x10002660`. `vrtTextOut` is `ret 0x18` at `0x10002600`, an
address it shares with `vrtRectangle`, `vrtSetFadeTable`, `vrtStretchNgb` and
`n3dPrimitive`; `ngiSetPalette`, `rsLoadFadeTable`, `rsLoadTexture` and
`rsLoadBitmap` are `ret 0xc` at `0x100025b0`; `vrtPutDib` and 23 others are
`ret 8` at `0x100025a0`. Every `vrt`, `n3d` and `bsp` export is among them.

The control is that the RsLi reader this chapter is about is *not* a stub:
`rsOpenLib` (`0x10019420`), `rsLoadFast` (`0x10019880`), `rsLoad`
(`0x10019900`) and `rsGetInfo` (`0x10019d60`) are real functions, and they are
what the D3D texture loader calls (`0x1000fbd4`, `0x1000fbdd`).

So **nothing in the shipped game reads `PAL.PAL`'s `Ipol` table.** It is the
8-bit blend table of a renderer that was taken out. The D3D font path below
needs no blend table: the glyph's own grey ramp is in the atlas.

## The eleven fonts, and which the game draws with — *read*

`World3D.dll`'s `stdInitGame` takes an archive name, a palette name and a font
name; `iron3d.dll` passes `sys.lib`, `pal.pal` and `ARIALTEX.TFT`
(`0x1005ca97`, `0x1005ca92`, `0x1005ca8d`; the call at `0x1005cadb`).
`sys.lib` is an ordinary NRes with those same two member names, and its
`PAL.PAL` is byte-identical to `gamefont.rlb`'s — but **its `ARIALTEX.TFT` is
a different font**: 21556 bytes against 20532, a header of (8, 7, 8/128, 1)
against (15, 17, 18/128, 1), and an atlas in pixel format **0**, carrying its
own 1024-byte palette rather than borrowing `PAL.PAL`. That one is the
**debug font**: `camGetDebugFont` hands back the
global it is stored in (`World3D.dll:0x10004e50`, `0x1079517c`),
`camPutDebugStr` draws with it (`0x10004e60`), and the loader sets its colour
to `0x000fff0f` on all six vertices (`0x1001363b`) — the bright green debug
text is that constant.

The fonts the interface and the HUD actually draw with are `ui/font.lib`'s
nine, chosen through `ui/menu_resources.cfg`'s font substitutes
([34](34-progression.md)): `gf_`, `tf_` and `mf_` at each of the three screen
sizes are that file's `GAME_FONT`, `TOOL_FONT` and `MENU_FONT`. (It names a
fourth, `HELP_FONT`, that the `fonts` resource has no entry for.)

So **eleven** `Tfnt` ship, and everything below was checked on all eleven.

## The header's four words are the font's metrics — *read*, and *measured*

The 20-byte header is `'Tfnt'` and four words, and the loader copies them into
the font object out of order (`Ngi32.dll:0x1000fbfd`–`0x1000fc1b`), along with
the 4096 bytes of glyph records, which go to `font + 0x38`:

| word | kept at | what it is | getter |
|---|---|---|---|
| 0 | `+0x1038` | the cell a fixed-pitch run gives every glyph (`0x10010e73`) | slot `+0x10`, `0x10010dd0` |
| 1 | `+0x103c` | the glyph's **height**; the cell drawn is one more | slot `+0x0c`, `0x10010d50` |
| 2 | `+0x1048` | that cell's height in **texture coordinates** (`0x10011287`) | — |
| 3 | `+0x1040` | what the pen adds after every glyph's advance | set by slot `+0x14`, `0x10010db0` |

*Measured* on all eleven: word 3 is **1** everywhere; word 2 × the atlas height
is exactly word 1 + 1 — 18 on `gamefont.rlb`'s, 8 on `sys.lib`'s and on
`gf_640`, 13 on `mf_640`, 20 on `mf_1024`; and that same number is the atlas's
**row pitch**, so the rows are packed with nothing between them. The font is
therefore drawn one texel to the pixel in both directions: `(u1 − u0)` × the
atlas width is `advance + 1` on every drawn glyph of all eleven (123, 149 and
165 × 9 of them), and the quad the engine builds is `advance + 1` wide.

## How a glyph is drawn — *read*

The font object is 0x1060 bytes (`0x10007dd0`, the 3D render interface's slot
`+0x58`), its constructor is `0x10010c10` and its vtable `0x10031858` is
installed there at `0x10010c69`:

```
+0x00    vtable
+0x04    the atlas's texture object
+0x08    six ARGB colours, 0xff808080 from the constructor
+0x20    six more, the vertices' specular, zero from the constructor
+0x38    the 256 glyph records, 16 bytes each
+0x1038  the header's four words, as above
+0x1044  the 3D render the font draws through
+0x104c  1 while the translation table applies
+0x1050  the z range, +0x1058 and +0x105c the z and rhw every vertex takes
```

**Text-out is slot `+0x1c`, `0x10010e40`** — `camPutDebugStr` calls it at
`World3D.dll:0x10004ec8` — and it draws one line at the (x, y) it is given, in
whole pixels. Per character it writes **six D3D TL vertices of 32 bytes and
twelve indices**, a triangle strip 0-1-2-3-4-5 expanded to four triangles
(`0x10010f63`), so the glyph is two stacked quads:

| | left column, `tu` = `u0` | right column, `tu` = `u1` |
|---|---|---|
| row 0, `tv` = `v0` | x = X − 0.5, y = Y − 0.5 | x = X − 0.5 + advance + 1 |
| row 1, `tv` = `v0` + word 2 ÷ 2 | y = Y − 0.5 + (word 1 + 1) ÷ 2 | |
| row 2, `tv` = `v0` + word 2 | y = Y − 0.5 + word 1 + 1 | |

The half-pixel is the usual D3D sample offset; the 1.0 added to the right edge
is the literal at `0x10031684` (`0x10011187`). The three rows exist so that the
six colours at `+0x08` can be a gradient down the glyph; `camSetFontColor` and
the loader set all six the same, and the warbot designer's rows do set one,
colour *i* on vertex *i* (`iron3d.dll:0x10046c5e`,
[37-designer.md](37-designer.md#the-rows--read-and-seen)).

**The pen then moves `advance + word 3`** — `0x10011118` adds the header word,
`0x1001113f` adds the record's advance — and the string-width routine (slot
`+0x18`, `0x10010de0`) computes exactly `word 3 × length + Σ advance`
(`0x10010dff`, `0x10010e2a`). With word 3 = 1 on every shipped font, **a glyph
steps its advance plus one**, and since the cell is also `advance + 1` wide the
glyphs tile the line with no gap and no overlap.

*Seen*, and it is worth recording how: a frame of a recording of training
mission 1 has `TSW-1 Warrior` and `Human` in the cockpit's name panels. Taking
each glyph's ink columns out of the atlas and fitting the drawn positions
against the frame's, `gf_1024` at 0.9375 screen pixels per font pixel with the
step `advance + 1` places all twelve runs of `TSW-1 Warrior` to within **0.56
px, mean 0.24**, and all five of `Human` to within 0.50; the same fit with the
step `advance` alone is out by up to 4 px, and `gf_640` at 1.5 or `gf_800` at
1.2 by 4 to 14 px whichever step is used. That also settles which font and
which screen size the recording used — **1024 × 768**, not the 640 × 480 an
earlier round assumed when it measured "MISSION COMPLETE !" on `mf_640` and
concluded the step was the advance alone. It is not; the earlier measurement
was calibrated against the wrong font.

Line spacing is **not the font's**: the routine draws one line, at the y its
caller names, and there is no newline in it. What the font offers a caller is
its height through slot `+0x0c`, which is word 1 — one less than the cell.

## Which byte draws which glyph — *read*, and *measured*

There is no character mapping worth the name. The routine loads the string byte
zero-extended and indexes the 256 records by it (`0x100110e6`, then `shl 4` at
`0x10011100`); the width routine does the same (`0x10010e1a`). **Nothing is
ever replaced by `?`** — every byte reaches a record, and a byte the font has
no glyph for lands on a placeholder that draws nothing and steps the pen its
own advance, which on all eleven fonts is about half the line height.

The one transformation is a **256-entry `int16` table at
`Ngi32.dll:0x10036e50`**, applied only while the font's flag at `+0x104c` is 1
(`0x100110e9`, `0x100110f2`; `0x10010e17`, `0x10010e1f` in the width routine).
It is **Windows-1251 to CP866**:

| from | to | |
|---|---|---|
| 0xC0–0xDF | 0x80–0x9F | А–Я |
| 0xE0–0xEF | 0xA0–0xAF | а–п |
| 0xF0–0xFF | 0xE0–0xEF | р–я |
| 0xA8, 0xB8 | 0xF0, 0xF1 | Ё, ё |

and the identity on the other **190 of 256** entries, ASCII included.

The constructor sets that flag (`0x10010c43`), and the interface's font loader
**clears it on every font it makes**: `services.dll` creates a font through
render slot `+0x58` and immediately calls the font's slot `+0x20`
(`Ngi32.dll:0x10011380`, which writes `+0x104c`) with 0 — at `0x10007cc2` and
again at `0x100099e9`, the two places in that module that make one. So
`ui/font.lib`'s nine are indexed by the raw byte, and the table is there for
the CP866 fonts.

The data says the same from the other side. *Measured* over the drawn records:

- the nine `ui/font.lib` fonts draw an identical set of **165** glyphs, of
  which 79 are above 0x7f — **0xC0–0xFF solid**, plus 0x80, 0x82, 0x85, 0x87,
  0x8A, 0x8C–0x8E, 0x90–0x92, 0x95, 0xA0, 0xA5 and 0xAE. That is Windows-1251's
  Cyrillic block and its punctuation, and **not** CP866's, which would put
  А–Я at 0x80.
- `sys.lib`'s `ARIALTEX.TFT` draws 64 above 0x7f and `gamefont.rlb`'s 56, and
  both draw **only** inside 0x80–0xAF and 0xE0–0xEF — CP866's places, and
  nothing in 0xB0–0xDF or 0xF2–0xFF.

And the strings the game has to draw are Windows-1251. *Measured* over the
install's text: of the **24740** strings in the 112 shipped text resources —
every mission's `mission.cfg`, `briefing.cfg`, `messages.cfg` and `Mistips.mis`,
the six `ui/*.cfg`, the `.man`, `.tbl`, `.lst`, `.dsc` and `.ini` tables, and
`objects.dlb`'s 395 part descriptions — **none carries a byte above 0x7f**:
this is the English release and its text is ASCII. The displayed prose lives in
`DATA/TextRes.dll`'s 13 `STRINGTABLE` leaves, fetched with `LoadStringA`
(imported by `iron3d.dll` and `services.dll`), so the byte the font sees is the
system code page's. Of those **173** strings **3** hold a character above
U+007F, and one of them is `U+00C0` in `"Àskold"` — a Cyrillic **А** left
behind by the translation, and 0xC0 is А in Windows-1251, not in CP866.

## How the text is coloured — *read*

The colour is the vertices' **diffuse**, set through slot `+0x08`
(`0x10010cc0`), which takes two arrays of six dwords: the first becomes the six
diffuse colours at `+0x08` with `0xff000000` ORed in, the second the six
speculars at `+0x20` verbatim. `camSetFontColor` fills the first with one value
and the second with zeros (`World3D.dll:0x10004f00`), as does the loader with
`0x000fff0f`.

Before it draws, text-out hands the atlas to the render's slot `+0x78`
(`0x10008750`) with **phase 8** (`0x100110a4`), and phase 8 is record 13 of
`Ngi32.dll`'s phase table at `0x10036a30` (`0x10036c6c`), whose 25
`{stage, state, value}` triples at `0x10034a00` decode to:

| | |
|---|---|
| stage 0 colour | `MODULATE` of `TEXTURE` by `DIFFUSE` |
| stage 0 alpha | `SELECTARG1` — the texture's alone |
| stages 1–7 | `COLOROP` and `ALPHAOP` `DISABLE` |
| filters | `MAGFILTER` and `MINFILTER` `POINT`, `MIPFILTER` `NONE` — from filter class 8 at `0x10036e20`, the only class the record admits |
| alpha test | `ALPHATESTENABLE` 1, `ALPHAFUNC` `GREATEREQUAL`; the reference is the device's 1 ([07](07-objects.md)) |

and the draw itself (`0x10007b40`) sets `CULLMODE` to `NONE` and toggles
`CLIPPING` (render state 136) by whether the whole run fits the clip rectangle
— `0xd` when it does, `0xc` when it does not — then draws a `TRIANGLELIST`.

So **the run's colour multiplies the atlas; it does not replace it**, and there
is no alpha blending in the record at all. What keeps the cell's background off
the screen is the alpha test, and what gives the atlas an alpha is a header bit:
every one of the eleven fonts has `0x1000000` in its `Texm` header at `+0x14`,
which is one of the four things that put a texture on an **alpha surface**
(`0x1000fe39`), and there the palette's **index 0 is cleared to alpha 0**
(`0x1000f698`) while 1–255 are made opaque (`0x1000f6b0`). Index 0 is the
corner pixel of all eleven atlases and the background of every glyph cell.
([02-texm.md](02-texm.md#what-the-loader-does-with-alpha--read-and-measured)
says no shipped texture sets that bit; that is true of `Textures.lib` and
`Material.lib`, and the eleven fonts are the exception.)

So the glyph is drawn as **its own shading times the run's colour**, written
opaque, with index 0 dropped. That shading is not anti-aliasing: *measured*, an
atlas uses **2 to 5 indices**, index 0 always among them and always its corner
pixel, and the two that carry the glyph are 254 and 51, whose palette colours
are the white `(255, 255, 255)` and the dark grey `(64, 64, 64)` — a white body
and a **shadow at a quarter of it**, on **7 of the 11**. Multiplied by the
run's colour the shadow comes out a quarter-strength shade of the same hue,
which is what a recording of Mission 01's win shows. The other four have no
shadow: the three `tf_` tool fonts are white alone, and `gamefont.rlb`'s own
has 0 and 73, where 73 is also white.

Nothing converts out of display space anywhere along that path ([05](05-engine.md)).

## Not resolved

- **Where a caller of text-out puts the next line.** The font offers its height
  and the routine draws one line; which of the interface's text controls stacks
  lines by the height, by the height plus one, or by something of its own is not
  read. On all eleven fonts the atlas's row pitch is the cell height, so a line
  set at the cell leaves no gap.
- **The fixed-pitch and justified modes.** Text-out takes a cell width and a
  flag (`0x10010e63`–`0x10010eae`, `[esp+0x13]`) and will centre each glyph in a
  cell of at least header word 0; which callers pass them is not read.
- **What `sprites.lib` was for.** Its 24 members decode, but with nothing in the
  install naming the archive, which of them the software build drew where is a
  question about a renderer that is not in this game.

## Prior art

fparkan's [RsLi reference](https://fparkan.popov.link/reference/rsli/) named
the format and gave the entry layout and the list of storage methods, which is
what turned a high-entropy blob into a specific question. The keystream is not
in it; that came out of `Ngi32.dll`. As always with fparkan: documentation
only, never source, and everything checked against the shipped data — see
[09-method.md](09-method.md).
