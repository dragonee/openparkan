"""Probe for the ``NL`` archives -- ``gamefont.rlb`` and ``sprites.lib``.

**Unfinished.**  This is analysis scaffolding, not part of the ``openparkan``
library, and it is here because it got far enough to be worth writing down.
Nothing imports it.

The header is settled::

    0x00  char[2]  'NL'
    0x02  uint16   1            version
    0x04  uint16   count        2 in gamefont.rlb, 24 in sprites.lib
    0x06  uint16   count        the same value again
    0x08  6 bytes  zero
    0x0E  uint16   0xABBA       marker
    0x10  uint32   unpacked     87096 and 1573632
    0x14  uint32   packed       25991 and 99633
    0x18  uint32   zero
    0x1C  uint32   zero

``packed`` is exactly ``file size - 96`` on ``gamefont.rlb`` and ``file size -
776`` on ``sprites.lib``, so the stream sits at the end of the file and there
is a block of 64 or 744 bytes between the header and it.  What that block is
is not known; it is high-entropy on both, so it is not a plain directory.

``sprites.lib``'s ``unpacked`` reads as 24 members of 65536 + 32 bytes each --
24 * 65568 = 1573632 exactly -- which fits 24 sprites of 256 x 256 with a
32-byte header apiece.

The payload is **LZSS**, and ``gamefont.rlb`` all but decodes:

* a flag byte, then eight items, least significant bit first;
* a set bit is a literal byte;
* a clear bit is a two-byte match -- a 12-bit offset made of the first byte
  and the high nibble of the second, and a length of the low nibble plus 3.

That yields **87057 bytes of a declared 87096** -- 0.04% short -- and the
output is unmistakably a font: forty zero bytes and then a glyph table
stepping by four -- ``04 02 04 00``, ``08 06 08 00``, ``0c 09 0c 00``, ``10 0d
10 00`` -- which is a run of glyph boxes.  Where the last 39 bytes go is not
established.

``sprites.lib`` does **not** decode with those parameters: from offset 776 it
yields 352625 bytes of a declared 1573632.  A search over offsets, over the
12/4, 4/12, 11/5, 10/6 and 8/8 splits, over minimum lengths 1 to 4 and both bit
orders finds nothing exact for it, so either its stream starts somewhere
else or the two files are not packed the same way.

No shipped binary contains the ``'NL'`` magic, the ``0xABBA`` marker or either
file's name, so the loader has not been found either.
"""

from __future__ import annotations

import struct
from pathlib import Path

HEADER_SIZE = 32
MAGIC = b"NL"
MARKER = 0xABBA


def header(blob: bytes) -> dict:
    """The 32 bytes at the front of an NL archive."""
    if blob[:2] != MAGIC:
        raise ValueError(f"not an NL archive (magic {blob[:2]!r})")
    version, count, again = struct.unpack_from("<3H", blob, 2)
    marker = struct.unpack_from("<H", blob, 0x0E)[0]
    unpacked, packed = struct.unpack_from("<2I", blob, 0x10)
    return {
        "version": version,
        "count": count,
        "count_again": again,
        "marker": marker,
        "unpacked": unpacked,
        "packed": packed,
        "start": len(blob) - packed,
    }


def unpack(blob: bytes, start: int | None = None) -> bytes:
    """Decode the LZSS stream.  Right for gamefont.rlb bar 39 bytes."""
    if start is None:
        start = header(blob)["start"]
    out = bytearray()
    pos = start
    end = len(blob)
    while pos < end:
        flags = blob[pos]
        pos += 1
        for _ in range(8):
            if pos >= end:
                break
            if flags & 1:
                out.append(blob[pos])
                pos += 1
            else:
                if pos + 1 >= end:
                    break
                low, high = blob[pos], blob[pos + 1]
                pos += 2
                offset = (low | ((high & 0xF0) << 4)) or 4096
                length = (high & 0x0F) + 3
                for _ in range(length):
                    source = len(out) - offset
                    out.append(out[source] if source >= 0 else 0)
            flags >>= 1
    return bytes(out)


def report(game: str | Path) -> None:
    game = Path(game)
    for name in ("gamefont.rlb", "sprites.lib"):
        blob = (game / name).read_bytes()
        head = header(blob)
        got = unpack(blob)
        print(f"{name}: {len(blob)} bytes, {head['count']} members")
        print(f"   declares {head['unpacked']} unpacked from {head['packed']} "
              f"packed, stream at {head['start']}")
        gap = head["unpacked"] - len(got)
        print(f"   decodes to {len(got)} -- short by {gap}" if gap > 0
              else f"   decodes to {len(got)} -- over by {-gap}")
        print(f"   first bytes: {' '.join(f'{b:02x}' for b in got[:24])}")


if __name__ == "__main__":  # pragma: no cover
    import sys

    report(sys.argv[1] if len(sys.argv) > 1 else ".")
