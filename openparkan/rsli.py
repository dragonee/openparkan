"""Reader for **RsLi** archives -- ``gamefont.rlb`` and ``sprites.lib``.

The other two archives.  Everything else the game ships is
[NRes](01-nres.md); these two are not, and they hold the 2D half of the game:
the interface art, the cockpit, the logo, the geyser animations, and the font.

The engine calls the format ``RsLi`` -- ``Ngi32.dll`` compares an in-memory
copy against the literal ``'RsLi'`` -- and its loader validates a 32-byte
header::

    0x00  char[2]  'NL'
    0x02  uint8    0            reserved
    0x03  uint8    1            version
    0x04  int16    count        entries
    0x06  int16    count        the same value again
    0x0E  uint16   0xABBA       set when the entry table is already sorted
    0x10  uint32   total        the unpacked sizes summed
    0x14  uint32   seed         the entry table's cipher key
    0x18  uint32   zero, twice

then ``count`` entries of 32 bytes, then the members::

    0x00  char[12] name         uppercase ASCII, NUL-padded
    0x0C  uint8[4] unread       the loader steps over it
    0x10  int16    flags        storage method
    0x12  int16    order        this entry's place in the unsorted table
    0x14  uint32   unpacked size
    0x18  uint32   offset from the start of the file
    0x1C  uint32   packed size

**The entry table is encrypted**, which is what makes these two archives look
like noise.  The cipher is two bytes of state seeded from the word at 0x14 --
``a`` from its low byte, ``d`` from its second -- and per byte of the table::

    a = ((a << 1) & 0xFF) ^ d
    d >>= 1
    plain = cipher ^ a
    d ^= a

It runs across the whole table without resetting between records, which is why
nothing short of the whole table decrypts.  Once it does, the names come out:
``PAL.PAL`` and ``ARIALTEX.TFT`` in the font archive, and ``COCKPIT.TEX``,
``LOGO.TEX``, ``INTERF1``..``INTERF8`` and two geyser animations in the other.

``flags`` picks a storage method.  Two of the seven occur here: the 24 members
of ``sprites.lib`` are ``0x100`` raw Deflate, and the two of ``gamefont.rlb``
are ``0x040`` LZSS.

**Deflate is read; LZSS is not.**  All 24 sprite members inflate to exactly the
size their entry declares and decode as ordinary ``Texm`` textures -- 4444 at
64 x 64, 128 x 128 and 256 x 256 -- so they go straight through
``openparkan.texm``.  The two font members do not: the obvious 12-bit offset,
4-bit length shape reproduces their first few kilobytes and then falls apart,
emitting maximum-length matches from the wrong place, and the size check does
not catch it because the output is truncated to fit.  ``read`` refuses them
rather than hand back plausible rubbish; see ``docs/12-rsli.md``.
"""

from __future__ import annotations

import struct
import zlib
from dataclasses import dataclass
from pathlib import Path

MAGIC = b"NL"
VERSION = 1
HEADER_SIZE = 32
ENTRY_SIZE = 32
#: Set in the header when the entry table is already in order.
PRESORTED = 0xABBA

#: Storage methods.  The engine defines seven; the shipped data uses three.
STORE_RAW = 0x000
STORE_TRANSFORM = 0x020
STORE_LZSS = 0x040
STORE_TRANSFORM_LZSS = 0x060
STORE_HUFFMAN_LZSS = 0x080
STORE_TRANSFORM_HUFFMAN_LZSS = 0x0A0
STORE_DEFLATE = 0x100

STORE_NAMES = {
    STORE_RAW: "raw",
    STORE_TRANSFORM: "transform",
    STORE_LZSS: "lzss",
    STORE_TRANSFORM_LZSS: "transform+lzss",
    STORE_HUFFMAN_LZSS: "huffman+lzss",
    STORE_TRANSFORM_HUFFMAN_LZSS: "transform+huffman+lzss",
    STORE_DEFLATE: "deflate",
}

#: The LZSS window, and the smallest match it can encode.
LZSS_WINDOW = 4096
LZSS_MIN_MATCH = 3


class RsLiFormatError(ValueError):
    pass


def is_rsli(path: str | Path) -> bool:
    """Whether a file begins with the RsLi header."""
    with open(path, "rb") as fh:
        return fh.read(4) == MAGIC + bytes((0, VERSION))


def decrypt_table(cipher: bytes, seed: int) -> bytes:
    """Undo the entry table's stream cipher.  It is its own inverse."""
    a = seed & 0xFF
    d = (seed >> 8) & 0xFF
    out = bytearray(len(cipher))
    for i, byte in enumerate(cipher):
        a = ((a << 1) & 0xFF) ^ d
        d >>= 1
        out[i] = byte ^ a
        d ^= a
    return bytes(out)


def unpack_lzss(data: bytes, size: int) -> bytes:
    """**Wrong past the first few kilobytes.**  Kept for the record only.

    A flag byte, eight items, least significant bit first; a set bit a
    literal, a clear bit a two-byte match with a 12-bit offset and a length of
    four bits plus three.  That reproduces the head of both font members --
    ``ARIALTEX.TFT``'s ``Tfnt`` magic and the ``Texm`` at 4116 -- and then
    starts emitting 18-byte matches from the wrong offset, so the tail is a
    repeat of whatever came before.  ``read`` does not use it.
    """
    out = bytearray()
    pos = 0
    end = len(data)
    while pos < end and len(out) < size:
        flags = data[pos]
        pos += 1
        for _ in range(8):
            if pos >= end or len(out) >= size:
                break
            if flags & 1:
                out.append(data[pos])
                pos += 1
            else:
                if pos + 1 >= end:
                    break
                low, high = data[pos], data[pos + 1]
                pos += 2
                offset = (low | ((high & 0xF0) << 4)) or LZSS_WINDOW
                length = (high & 0x0F) + LZSS_MIN_MATCH
                for _ in range(length):
                    source = len(out) - offset
                    out.append(out[source] if source >= 0 else 0)
            flags >>= 1
    return bytes(out[:size])


@dataclass(frozen=True)
class Entry:
    name: str
    flags: int
    #: Where this entry sat before the table was sorted.
    order: int
    size: int
    offset: int
    packed: int

    @property
    def storage(self) -> str:
        return STORE_NAMES.get(self.flags, f"unknown {self.flags:#x}")


class RsLiArchive:
    """``gamefont.rlb`` or ``sprites.lib``, with its entry table decrypted."""

    def __init__(self, data: bytes, source: str = "<rsli>"):
        self.data = data
        self.source = source
        if len(data) < HEADER_SIZE or data[:2] != MAGIC:
            raise RsLiFormatError(f"{source}: not an RsLi archive")
        if data[3] != VERSION:
            raise RsLiFormatError(f"{source}: version {data[3]}, expected {VERSION}")
        count, again = struct.unpack_from("<2h", data, 4)
        if count < 0 or count != again:
            raise RsLiFormatError(f"{source}: entry count {count} / {again}")
        self.presorted = struct.unpack_from("<H", data, 0x0E)[0] == PRESORTED
        self.total, self.seed = struct.unpack_from("<2I", data, 0x10)
        table_end = HEADER_SIZE + count * ENTRY_SIZE
        if table_end > len(data):
            raise RsLiFormatError(f"{source}: {count} entries do not fit")
        table = decrypt_table(data[HEADER_SIZE:table_end], self.seed)
        self.entries: list[Entry] = []
        for i in range(count):
            record = table[i * ENTRY_SIZE : (i + 1) * ENTRY_SIZE]
            flags, order = struct.unpack_from("<2h", record, 0x10)
            size, offset, packed = struct.unpack_from("<3I", record, 0x14)
            self.entries.append(
                Entry(
                    name=record[:12].split(b"\0")[0].decode("latin-1"),
                    flags=flags,
                    order=order,
                    size=size,
                    offset=offset,
                    packed=packed,
                )
            )

    @classmethod
    def open(cls, path: str | Path) -> RsLiArchive:
        path = Path(path)
        return cls(path.read_bytes(), str(path))

    def __len__(self) -> int:
        return len(self.entries)

    def __iter__(self):
        return iter(self.entries)

    def read(self, entry: Entry) -> bytes:
        """Unpack one member.

        ``INTERF8.TEX`` declares one byte more than ``sprites.lib`` holds, so
        a short read is allowed; the deflate stream ends before it anyway.
        """
        raw = self.data[entry.offset : entry.offset + entry.packed]
        if not raw:
            raise RsLiFormatError(f"{self.source}: {entry.name} is outside the file")
        if entry.flags == STORE_RAW:
            out = raw[: entry.size]
        elif entry.flags == STORE_DEFLATE:
            out = zlib.decompressobj(-zlib.MAX_WBITS).decompress(raw)
        else:
            raise RsLiFormatError(
                f"{self.source}: {entry.name} uses storage {entry.storage}, "
                f"which is not implemented"
            )
        if len(out) != entry.size:
            raise RsLiFormatError(
                f"{self.source}: {entry.name} unpacked to {len(out)} bytes, "
                f"not the {entry.size} it declares"
            )
        return out

    def read_name(self, name: str) -> bytes:
        for entry in self.entries:
            if entry.name.lower() == name.lower():
                return self.read(entry)
        raise KeyError(name)
