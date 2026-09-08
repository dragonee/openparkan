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
Huffman + LZSS, and raw Deflate — and the shipped data uses two of them: the
font's pair are `0x040` LZSS, and all 24 sprites are `0x100` raw Deflate.

**All 26 members unpack to exactly the size their entry declares.**

The LZSS is the shape the era used everywhere: a flag byte, then eight items,
least significant bit first; a set bit is a literal, a clear bit a two-byte
match with a 12-bit offset — the first byte plus the high nibble of the second
— and a length of the low nibble plus three. It overruns by one byte on both
members that use it, so the declared size is the authority and the reader
truncates.

`sprites.lib::INTERF8.TEX` declares one byte more than the file holds; the
deflate stream ends before it, so a short read is harmless. fparkan
[documents the same quirk](https://fparkan.popov.link/reference/rsli/), which
is a pleasing independent confirmation that the table decrypts correctly.

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

`gamefont.rlb`'s two members are not Texm. `PAL.PAL` is 66564 bytes and
`ARIALTEX.TFT` 20532; the `.TFT` opens with a table of glyph boxes stepping by
four — `04 02 04 00`, `08 06 08 00`, `0c 09 0c 00` — after forty zero bytes.
Neither is parsed further.

## Prior art

fparkan's [RsLi reference](https://fparkan.popov.link/reference/rsli/) named
the format and gave the entry layout and the list of storage methods, which is
what turned a high-entropy blob into a specific question. The keystream is not
in it; that came out of `Ngi32.dll`. As always with fparkan: documentation
only, never source, and everything checked against the shipped data — see
[09-method.md](09-method.md).
