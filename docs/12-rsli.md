# RsLi — `gamefont.rlb` and `sprites.lib`

Two files out of the 118 the game ships are not [NRes](01-nres.md), and they
hold the whole 2D half of it: the cockpit, the interface art, the cursors, the
logo, two geyser animations, and the font. The engine calls the format
**RsLi** — `Ngi32.dll` compares an in-memory copy against the literal
`'RsLi'` — and its loader is at `0x100194cd` in that DLL.

Everything below is re-derived by `uv run openparkan verify`.

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
engine tints them.

```
uv run openparkan font --out font.png
uv run openparkan font --glyphs
```

## Prior art

fparkan's [RsLi reference](https://fparkan.popov.link/reference/rsli/) named
the format and gave the entry layout and the list of storage methods, which is
what turned a high-entropy blob into a specific question. The keystream is not
in it; that came out of `Ngi32.dll`. As always with fparkan: documentation
only, never source, and everything checked against the shipped data — see
[09-method.md](09-method.md).
