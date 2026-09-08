"""Probe for the ``NL`` archives -- ``gamefont.rlb`` and ``sprites.lib``.

**Unfinished.**  Analysis scaffolding, not part of the ``openparkan`` library;
nothing imports it.  It is here because it got far enough to be worth writing
down, and because one wrong turn in it is worth recording so nobody repeats it.

fparkan calls this format **RsLi**, and its format reference at
``https://fparkan.popov.link/reference/rsli/`` describes the layout.
Documentation only, never source -- see ``docs/09-method.md`` -- and what
follows separates what that reference says from what has been checked against
the shipped files.

Confirmed here on both files::

    0x00  char[2]  'NL'
    0x02  uint8    0            reserved
    0x03  uint8    1            version
    0x04  int16    count        2 in gamefont.rlb, 24 in sprites.lib
    0x06  int16    count        the same value again
    0x0E  uint16   0xABBA       marker

and the layout ``[header 32][entry table count * 32][payloads]``, which puts
the payload at 96 in ``gamefont.rlb`` and 800 in ``sprites.lib``.

Taken from the reference and **not** confirmed, because the entry table cannot
yet be read::

    entry, 32 bytes
        char[12]  name             uppercase ASCII
        uint8[4]  service
        int16     flags            the storage method
        int16     sort_to_original
        uint32    unpacked size
        uint32    offset
        uint32    packed size

    flags   0x000 raw          0x020 byte transform     0x040 LZSS
            0x060 transform + LZSS                      0x080 Huffman + LZSS
            0x0A0 transform + Huffman + LZSS            0x100 raw Deflate

The entry table is XOR-transformed by a keystream whose initial state is the
low 16 bits of the word at 0x14, running across the whole table without
resetting between records.  **The generator is not documented and has not been
recovered**, so the table stays unreadable and members cannot be located.
Fifteen classic LCGs, over 16- and 32-bit states and four output byte
selections, all fail the test that the top byte of every ``uint32`` in an
entry must decrypt to zero on files this small.

The wrong turn
--------------

Before reading the reference this probe took 0x10 for an unpacked size and
0x14 for a packed size, and concluded the stream ran from ``file size -
packed`` to the end.  On ``gamefont.rlb`` that lands on 96, which is right,
and the payload really is 25991 bytes -- so the arithmetic looked like a
derivation.  It was a coincidence: 0x14 is a *seed*, and on ``sprites.lib``
the same rule gives 776 where the table ends at 800.

That mattered more than the 24 bytes, because it hid the real reason
``sprites.lib`` would not decode: **its members are compressed separately and
by different methods**, one of which is Deflate.  No single pass over the
payload region was ever going to work, and the search that tried harder and
harder LZSS shapes against a single declared size was chasing a number that
means something else.

What does still stand
---------------------

Decoding ``gamefont.rlb``'s payload as one LZSS stream from 96 yields 87057
bytes whose head is unmistakably a font -- forty zeros then a glyph table
stepping by four, ``04 02 04 00``, ``08 06 08 00``, ``0c 09 0c 00``.  That is
consistent with its first member using method 0x040, and it is the one piece
of these files this probe can still read.  The shape: a flag byte, then eight
items, least significant bit first; a set bit is a literal, a clear bit a
two-byte match with a 12-bit offset made of the first byte and the high nibble
of the second, and a length of the low nibble plus three.
"""

from __future__ import annotations

import struct
from pathlib import Path

HEADER_SIZE = 32
ENTRY_SIZE = 32
MAGIC = b"NL"
MARKER = 0xABBA


def header(blob: bytes) -> dict:
    """The 32 bytes at the front of an RsLi archive."""
    if blob[:2] != MAGIC:
        raise ValueError(f"not an NL archive (magic {blob[:2]!r})")
    reserved, version = blob[2], blob[3]
    count, again = struct.unpack_from("<2h", blob, 4)
    marker = struct.unpack_from("<H", blob, 0x0E)[0]
    unknown, seed = struct.unpack_from("<2I", blob, 0x10)
    payload = HEADER_SIZE + count * ENTRY_SIZE
    return {
        "reserved": reserved,
        "version": version,
        "count": count,
        "count_again": again,
        "marker": marker,
        "unknown_0x10": unknown,
        "seed": seed,
        "table": HEADER_SIZE,
        "payload": payload,
        "payload_size": len(blob) - payload,
    }


def unpack_lzss(blob: bytes, start: int, stop: int | None = None) -> bytes:
    """The LZSS shape gamefont.rlb's first member appears to use."""
    out = bytearray()
    pos = start
    end = len(blob) if stop is None else stop
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
        print(f"{name}: {len(blob)} bytes, version {head['version']}, "
              f"{head['count']} entries, marker {head['marker']:#06x}")
        print(f"   table {head['table']}..{head['payload']}, "
              f"payload {head['payload_size']} bytes")
        print(f"   +0x10 {head['unknown_0x10']}, seed {head['seed']}")
        got = unpack_lzss(blob, head["payload"])
        print(f"   payload as one LZSS stream -> {len(got)} bytes, "
              f"head {' '.join(f'{b:02x}' for b in got[:12])}")


if __name__ == "__main__":  # pragma: no cover
    import sys

    report(sys.argv[1] if len(sys.argv) > 1 else ".")
