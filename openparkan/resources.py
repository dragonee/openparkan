"""The resource descriptor -- how a name in a config file finds its bytes.

The game never hardcodes a path to a sound, a texture or a line of dialogue.
It writes a **descriptor** instead, and the same six-line shape turns up in
``mission.cfg``, in ``ui/*.cfg`` and in ``DATA/TextRes.cfg``::

    object  briefing_sounds
     desc       = "resource"
     library    = "voices.lib"
     libtype    = "multi"
     type       = 4
     T01_T01    = t01_t01.wav          a member, named
     T01_T02    = t01_t02.wav
    end

    object text_resources
     desc       = "resource"
     library    = "data\\TextRes.dll"
     libtype    = "multi"
     type       = 6
     T01_T01    = 8                    a member, numbered
     T01_T02    = 9
    end

``desc = "resource"`` marks the object, the four keys above describe the
library, and **every other key is a binding**: a name the rest of the game
quotes, against either a member of the library or an index into it.

**132 descriptor objects across 32 files bind 751 names, and every one
resolves** -- 507 by member name, 244 by index.  They name seven libraries:
six NRes archives (``voices.lib`` 234 members, ``sounds.lib`` 167,
``ui/minimap.lib`` 53, ``ui/ui_back.lib`` 32, ``ui/ui.lib`` 15,
``ui/font.lib`` 9) and one DLL.

``libtype`` is ``multi`` on all 132.  ``type`` sorts them by what the library
holds: **1** textures (32, the minimap, interface and background archives),
**2** fonts (2), **4** sounds (68, ``sounds.lib`` and ``voices.lib``), **5**
music (29) and **6** text (1).  No descriptor uses 3.  Types 4 and 5 both
name ``sounds.lib`` and the role separates them -- 5 is *only ever*
``ambient_music_loop``, the mission's looping theme, and 4 is every one-shot
beside it, which reads as streamed against sampled though nothing here proves
it.  For the DLL the number is not the game's own: **6 is Win32
``RT_STRING``**, and
``DATA/TextRes.dll`` is a resource-only PE whose string table is the game's
script.  It holds **173 strings** in 13 blocks, ids 8 to 193, under language
1033.  ``TextRes.cfg`` names **173**, and the two sets are equal: every name
has a string and every string has a name.

So a line of dialogue is reached in two hops -- ``TextResID = "T01_T01"`` in a
briefing, ``T01_T01 = 8`` in ``TextRes.cfg``, string 8 in the DLL -- and its
voice in two more, through ``mission.cfg``'s own descriptor into
``voices.lib``.  See `20-briefing.md`.

The strings are UTF-16 and read as English throughout.  One artefact survives
the translation: three of the 173 carry a character above U+007F.  One is the
name *Askold*, which kept its Russian A both times it appears; the other two
are a curly apostrophe and an ellipsis.

Everything above is re-derived by ``uv run openparkan verify``.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass
from pathlib import Path

from .mission import load_cfg

#: What a descriptor object says it is.
RESOURCE = "resource"

#: The keys that describe the library rather than bind a name.
META = ("desc", "library", "libtype", "type")

#: The only ``libtype`` in the shipped files.
LIBTYPE = "multi"

#: The name table for the game's text, relative to the installation.
TEXT_INDEX = ("DATA", "TextRes.cfg")

#: What ``type`` means, as far as the shipped descriptors show it.  3 is
#: unused.  4 and 5 are both sounds and are told apart by role, not library.
TYPES = {1: "textures", 2: "fonts", 4: "sounds", 5: "music", 6: "text"}

#: The one role that ever carries type 5.
MUSIC_ROLE = "ambient_music_loop"

#: Win32 ``RT_STRING``.  A DLL-backed descriptor states the Win32 resource
#: type as its own ``type``, which is why 6 appears in both tables.
RT_STRING = 6

#: Strings per Win32 string block; block *n* holds ids ``(n-1) * 16 + i``.
BLOCK = 16

#: The language every block in ``TextRes.dll`` is filed under: US English.
LANGUAGE = 1033

#: How many strings that table holds, and what they are named by.
TEXTS = 173

#: What the shipped installation carries, which the checks pin so that a
#: parser that quietly drops a binding fails instead of reporting a smaller
#: population as a clean sweep.
DESCRIPTORS = 132
BINDINGS = 751


class ResourceFormatError(ValueError):
    pass


@dataclass(frozen=True)
class Descriptor:
    """One ``desc = "resource"`` object: a library and the names it binds."""

    role: str
    library: str
    libtype: str
    type: int | None
    bindings: dict[str, str]
    source: str = ""

    @property
    def kind(self) -> str:
        """What the library holds, as far as ``TYPES`` knows."""
        return TYPES.get(self.type, "?") if self.type is not None else "?"

    @property
    def numbered(self) -> bool:
        """Whether the bindings are indices rather than member names."""
        return bool(self.bindings) and all(
            v.lstrip("-").isdigit() for v in self.bindings.values())

    def __len__(self) -> int:
        return len(self.bindings)

    def get(self, name: str) -> str | None:
        """The member or index bound to ``name``, case-insensitively."""
        if name in self.bindings:
            return self.bindings[name]
        lower = name.lower()
        for key, value in self.bindings.items():
            if key.lower() == lower:
                return value
        return None


def descriptors(path: str | Path) -> list[Descriptor]:
    """Every resource descriptor in one ``.cfg`` file, in file order."""
    out = []
    source = str(path)
    for role, props in load_cfg(path).items():
        if props.get("desc") != RESOURCE:
            continue
        raw = props.get("type")
        out.append(Descriptor(
            role=role,
            library=props.get("library", ""),
            libtype=props.get("libtype", ""),
            type=int(raw) if raw is not None and raw.lstrip("-").isdigit() else None,
            bindings={k: v for k, v in props.items() if k not in META},
            source=source,
        ))
    return out


def locate(game: Path, library: str) -> Path | None:
    """Resolve a descriptor's ``library`` against the installation.

    The paths are the developers' own: backslash-separated, sometimes doubled
    (``ui\\\\ui.lib``), and in whatever case the author typed.  Nothing else in
    the game writes a path this way, so the walk is here rather than shared.
    """
    here = game
    parts = library.replace("\\\\", "\\").replace("\\", "/").split("/")
    for part in parts:
        if not part:
            continue
        try:
            hit = next((p for p in here.iterdir() if p.name.lower() == part.lower()), None)
        except OSError:
            return None
        if hit is None:
            return None
        here = hit
    return here if here != game else None


# --------------------------------------------------------------------------
# The one library that is not an archive
# --------------------------------------------------------------------------
#
# ``TextRes.dll`` is a resource-only PE, so reading it means walking the
# image's own directories rather than an NRes table.  It is 50 lines of
# structure and no dependency, which is cheaper than either giving up on the
# text or asking the library's users to install a PE parser.


def _sections(data: bytes) -> tuple[list[tuple[int, int, int]], int]:
    """The section map, and the RVA of the resource directory."""
    if data[:2] != b"MZ":
        raise ResourceFormatError("not a PE image: no MZ header")
    lfanew = struct.unpack_from("<I", data, 0x3C)[0]
    if data[lfanew:lfanew + 4] != b"PE\0\0":
        raise ResourceFormatError("not a PE image: no PE signature")
    sections_n = struct.unpack_from("<H", data, lfanew + 6)[0]
    optional_size = struct.unpack_from("<H", data, lfanew + 20)[0]
    optional = lfanew + 24
    magic = struct.unpack_from("<H", data, optional)[0]
    # The data directories sit after the optional header's fixed part, which
    # is 96 bytes for PE32 and 112 for PE32+, minus the count word itself.
    directories = optional + (92 if magic == 0x10B else 108)
    resource_rva = struct.unpack_from("<I", data, directories + 4 + 2 * 8)[0]
    table = []
    for i in range(sections_n):
        off = optional + optional_size + i * 40
        virtual_size, virtual_address = struct.unpack_from("<II", data, off + 8)
        raw_size, raw_offset = struct.unpack_from("<II", data, off + 16)
        table.append((virtual_address, max(virtual_size, raw_size), raw_offset))
    return table, resource_rva


def _offset(sections, rva: int) -> int:
    for virtual_address, size, raw in sections:
        if virtual_address <= rva < virtual_address + size:
            return raw + (rva - virtual_address)
    raise ResourceFormatError(f"RVA {rva:#x} is in no section")


def _entries(data: bytes, base: int, off: int) -> list[tuple[int, int]]:
    """One resource directory's ``(id, offset)`` entries.

    A named entry keeps its high bit set in the id, and a subdirectory keeps
    it set in the offset; the callers below want ids, and every directory in
    ``TextRes.dll`` is numbered.
    """
    named, numbered = struct.unpack_from("<HH", data, base + off + 12)
    out = []
    for i in range(named + numbered):
        out.append(struct.unpack_from("<II", data, base + off + 16 + i * 8))
    return out


def strings(data: bytes, language: int | None = None) -> dict[int, str]:
    """Every ``RT_STRING`` in a PE image, keyed by string id.

    Empty slots are dropped: a block always writes 16 lengths and the unused
    ones are zero, which is the file saying the id is not in use rather than
    that the string is blank.
    """
    sections, rva = _sections(data)
    if not rva:
        return {}
    base = _offset(sections, rva)
    out: dict[int, str] = {}
    for type_id, entry in _entries(data, base, 0):
        if type_id != RT_STRING or not entry & 0x8000_0000:
            continue
        for block, sub in _entries(data, base, entry & 0x7FFF_FFFF):
            if not sub & 0x8000_0000:
                continue
            for lang, leaf in _entries(data, base, sub & 0x7FFF_FFFF):
                if language is not None and lang != language:
                    continue
                data_rva, size = struct.unpack_from("<II", data, base + leaf)
                at = _offset(sections, data_rva)
                end = at + size
                for i in range(BLOCK):
                    if at + 2 > end:
                        break
                    count = struct.unpack_from("<H", data, at)[0]
                    at += 2
                    if count:
                        out[(block - 1) * BLOCK + i] = \
                            data[at:at + count * 2].decode("utf-16-le")
                    at += count * 2
    return out


@dataclass
class TextResources:
    """``TextRes.cfg`` and the DLL it names, joined."""

    names: dict[str, int]
    table: dict[int, str]

    @classmethod
    def open(cls, game: str | Path) -> TextResources:
        game = Path(game)
        index = game.joinpath(*TEXT_INDEX)
        found = descriptors(index)
        if not found:
            raise ResourceFormatError(f"{index}: no resource descriptor")
        d = found[0]
        library = locate(game, d.library)
        if library is None:
            raise ResourceFormatError(f"{index}: no library {d.library!r}")
        return cls(
            names={k: int(v) for k, v in d.bindings.items() if v.lstrip("-").isdigit()},
            table=strings(library.read_bytes()),
        )

    def __len__(self) -> int:
        return len(self.table)

    def get(self, name: str) -> str | None:
        """The text a resource name stands for, or ``None`` if unbound.

        The final briefing names two ids the shipped table does not carry, so
        a caller has to expect the miss; see `20-briefing.md`.
        """
        ident = self.names.get(name)
        return None if ident is None else self.table.get(ident)
