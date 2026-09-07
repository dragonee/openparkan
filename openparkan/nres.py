"""Reader for the NRes container format used by Parkan: Iron Strategy.

Every archive the game ships -- ``*.rlb``, ``*.lib``, ``*.dlb``, ``*.res``,
``*.trf`` and the per-map ``Land.msh`` / ``Land.map`` files -- is the same
container.  See ``docs/01-nres.md`` for the byte-level layout.
"""

from __future__ import annotations

import struct
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path

MAGIC = b"NRes"
HEADER_SIZE = 16
ENTRY_SIZE = 64


class NotAnNResArchive(ValueError):
    """Raised when a file does not carry the ``NRes`` magic."""


@dataclass(frozen=True)
class NResEntry:
    """One member of an NRes archive."""

    type_raw: bytes  # 4 bytes: either a FourCC tag or a little-endian int
    name: str
    offset: int
    size: int
    index: int
    #: Number of elements in the payload, where the member is an array.  Zero
    #: on members that are not arrays.  ArealMap.dll reads its areal count from
    #: this field, and for terrain streams it equals size / stride exactly.
    element_count: int = 0
    #: A second count, used alongside ``element_count`` where a payload holds
    #: two arrays -- the interior path graph's nodes and links, for instance.
    link_count: int = 0

    @property
    def tag(self) -> str:
        """FourCC as text, or ``#12`` style for numeric type ids."""
        if all(32 <= b < 127 for b in self.type_raw):
            return self.type_raw.decode("ascii").strip()
        return f"#{struct.unpack('<I', self.type_raw)[0]}"

    @property
    def type_id(self) -> int:
        return struct.unpack("<I", self.type_raw)[0]

    def __repr__(self) -> str:  # pragma: no cover - debugging aid
        return f"NResEntry({self.tag} {self.name!r} off={self.offset} size={self.size})"


class NResArchive:
    """Random-access reader over an NRes archive held in memory.

    Archives top out around 57 MB (``Textures.lib``), so reading the whole
    file is simpler than seeking and costs little.
    """

    def __init__(self, data: bytes, source: str = "<bytes>"):
        if data[:4] != MAGIC:
            raise NotAnNResArchive(
                f"{source}: expected {MAGIC!r}, found {data[:4]!r}"
            )
        self.data = data
        self.source = source
        _, self.version, count, declared_size = struct.unpack_from("<4sIII", data, 0)
        if declared_size != len(data):
            raise ValueError(
                f"{source}: header declares {declared_size} bytes, file is {len(data)}"
            )
        directory = len(data) - count * ENTRY_SIZE
        if directory < HEADER_SIZE:
            raise ValueError(f"{source}: directory of {count} entries does not fit")
        self.entries: list[NResEntry] = []
        for i in range(count):
            rec = data[directory + i * ENTRY_SIZE : directory + (i + 1) * ENTRY_SIZE]
            self.entries.append(
                NResEntry(
                    type_raw=rec[0:4],
                    name=rec[20:52].split(b"\0")[0].decode("latin-1"),
                    size=struct.unpack_from("<I", rec, 12)[0],
                    offset=struct.unpack_from("<I", rec, 56)[0],
                    index=struct.unpack_from("<I", rec, 60)[0],
                    element_count=struct.unpack_from("<I", rec, 4)[0],
                    link_count=struct.unpack_from("<I", rec, 8)[0],
                )
            )

    @classmethod
    def open(cls, path: str | Path) -> NResArchive:
        path = Path(path)
        return cls(path.read_bytes(), str(path))

    def __len__(self) -> int:
        return len(self.entries)

    def __iter__(self) -> Iterator[NResEntry]:
        return iter(self.entries)

    def read(self, entry: NResEntry) -> bytes:
        return self.data[entry.offset : entry.offset + entry.size]

    def find(self, name: str) -> NResEntry:
        """Look up by name, case-insensitively (the game is inconsistent)."""
        low = name.lower()
        for e in self.entries:
            if e.name.lower() == low:
                return e
        raise KeyError(f"{self.source}: no entry named {name!r}")

    def read_name(self, name: str) -> bytes:
        return self.read(self.find(name))

    def by_type(self, tag: str) -> list[NResEntry]:
        return [e for e in self.entries if e.tag == tag]

    def one_of_type(self, type_id: int) -> bytes:
        """Read the single member with the given numeric type id.

        Per-map files (``Land.msh``) use numeric type ids as stream selectors
        and every member shares one name, so the type is the only usable key.
        """
        matches = [e for e in self.entries if e.type_id == type_id]
        if len(matches) != 1:
            raise KeyError(
                f"{self.source}: expected exactly one member of type {type_id}, "
                f"found {len(matches)}"
            )
        return self.read(matches[0])

    def has_type(self, type_id: int) -> bool:
        return any(e.type_id == type_id for e in self.entries)

    def layout_gaps(self) -> list[tuple[int, int]]:
        """Byte ranges covered by neither the header, a member, nor the directory.

        Used by the verifier to prove the format model accounts for every byte.
        """
        spans = sorted((e.offset, e.offset + e.size) for e in self.entries)
        gaps: list[tuple[int, int]] = []
        cursor = HEADER_SIZE
        for start, end in spans:
            if start > cursor:
                gaps.append((cursor, start))
            cursor = max(cursor, end)
        directory = len(self.data) - len(self.entries) * ENTRY_SIZE
        if cursor < directory:
            gaps.append((cursor, directory))
        return gaps


def is_nres(path: str | Path) -> bool:
    with open(path, "rb") as fh:
        return fh.read(4) == MAGIC
