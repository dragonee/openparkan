"""Save games, ``SAVE/*.sav``.

A save is **not a designed file format**.  It is the engine's live object graph
written to disk more or less as it sat in memory: the same 32-byte string
fields the classes use, heap addresses left in place, and buffer tails that
were never zeroed, so a name is not reliably NUL-terminated.  Reconstructing
the object graph from it would mean reconstructing the classes, and this
reader does not try.

What it does read is the part that *is* structured, and the part that is
worth having: **what a save refers to**.  A save names the mission it is in,
the map under it, the research trees in play and every archive member the
world is built from -- and all of those can be checked against the
installation.  Across the six shipped saves, **1158 of the 1162 member
references resolve** into the archive they name.

The header is a real header and is parsed strictly::

    char[4]   "SLOT"
    uint8     version, 1 in every save
    uint8     0 for a campaign mission, 1 for a single one
    int32     length of the mission path
    char      path[length]      'missions/campaign/campaign.05/mission.01/'

Everything after it is recovered by **scanning**, not parsing, and the module
says so where it does: a reference is an archive name followed 32 bytes later
by a member name, which is the shape of the engine's two-string record.  That
is a heuristic with a good hit rate, not a decode.  See ``docs/17-saves.md``.

Everything above is re-derived by ``uv run openparkan verify``.
"""

from __future__ import annotations

import re
import struct
from dataclasses import dataclass
from pathlib import Path

#: The directory the game keeps saves in.
DIRECTORY = "SAVE"

#: The slot index beside the saves, in the engine's ``OBJECT``/``END`` text.
SLOTS = "saveslots.cfg"

#: The first four bytes of every save.
MAGIC = b"SLOT"

#: The version byte, 1 in all six shipped saves.
VERSION = 1

#: The byte after it.  0 on the five campaign saves, 1 on the single mission.
CAMPAIGN, SINGLE = 0, 1

#: The longest mission path worth believing.
MAX_PATH = 260

#: In the engine's two-string record the member name sits this far after the
#: archive name.  Both are 32-byte fields.
MEMBER_AT = 32

#: Archives a save is known to name.
ARCHIVES = ("objects.rlb", "effects.rlb")

_ARCHIVE = re.compile(rb"(" + rb"|".join(a.encode() for a in ARCHIVES) + rb")\x00")
_MEMBER = re.compile(rb"[A-Za-z_][A-Za-z0-9_.\-]{1,30}")
_MAP = re.compile(rb"DATA\\MAPS\\([A-Za-z0-9_]{1,30})\\land", re.IGNORECASE)
_TREE = re.compile(rb"MISSIONS\\SCRIPTS\\([A-Za-z0-9_]{1,30}\.trf)", re.IGNORECASE)


class SaveFormatError(ValueError):
    pass


@dataclass(frozen=True)
class Reference:
    """An archive member the saved world is built from."""

    archive: str
    member: str
    offset: int


@dataclass(frozen=True)
class Slot:
    """One entry of ``saveslots.cfg``."""

    slot: str
    name: str
    filename: str
    empty: bool


@dataclass(frozen=True)
class Save:
    """One ``.sav``: its header, and what it refers to."""

    source: Path
    version: int
    kind: int
    mission: str
    #: The map directory under ``DATA/MAPS``, from the first reference to one.
    map: str
    #: Every ``.trf`` research tree the save names, in file order.
    trees: tuple[str, ...]
    #: Every archive member reference the scan recovered.
    references: tuple[Reference, ...]

    @property
    def campaign(self) -> bool:
        return self.kind == CAMPAIGN

    @property
    def members(self) -> tuple[tuple[str, str], ...]:
        """The distinct ``(archive, member)`` pairs, in first-seen order."""
        seen: list[tuple[str, str]] = []
        for ref in self.references:
            pair = (ref.archive, ref.member)
            if pair not in seen:
                seen.append(pair)
        return tuple(seen)


def parse(data: bytes, source: Path | None = None) -> Save:
    """Read one save: the header strictly, the rest by scan."""
    where = source.name if source is not None else "<bytes>"
    if data[:4] != MAGIC:
        raise SaveFormatError(f"{where}: not a save; opens {data[:4]!r}, not {MAGIC!r}")
    if len(data) < 10:
        raise SaveFormatError(f"{where}: {len(data)} bytes, too short for a header")
    version, kind = data[4], data[5]
    if version != VERSION:
        raise SaveFormatError(f"{where}: version {version}, not {VERSION}")
    (length,) = struct.unpack_from("<I", data, 6)
    if not 0 < length <= MAX_PATH or 10 + length > len(data):
        raise SaveFormatError(f"{where}: mission path length {length}")
    mission = data[10 : 10 + length].decode("latin-1")

    found = _MAP.search(data)
    trees: list[str] = []
    for match in _TREE.finditer(data):
        name = match.group(1).decode("latin-1").lower()
        if name not in trees:
            trees.append(name)

    references: list[Reference] = []
    for match in _ARCHIVE.finditer(data):
        at = match.start() + MEMBER_AT
        member = _MEMBER.match(data, at)
        if member:
            references.append(
                Reference(match.group(1).decode(), member.group().decode(), match.start())
            )

    return Save(
        source=source or Path(where),
        version=version,
        kind=kind,
        mission=mission,
        map=found.group(1).decode("latin-1") if found else "",
        trees=tuple(trees),
        references=tuple(references),
    )


def read(path: Path) -> Save:
    """Read the save at ``path``."""
    return parse(path.read_bytes(), path)


def saves(game: Path) -> list[Path]:
    """Every ``.sav`` the installation holds, sorted."""
    return sorted((game / DIRECTORY).glob("*.sav"))


def slots(game: Path) -> list[Slot]:
    """Read ``saveslots.cfg``: the engine's own index of the save slots.

    Plain text in the engine's ``OBJECT name`` / ``END`` form, with ``#``
    comment banners and tab-separated ``key = value`` lines.
    """
    path = game / DIRECTORY / SLOTS
    out: list[Slot] = []
    name = ""
    fields: dict[str, str] = {}
    for line in path.read_text("latin-1").replace("\r\n", "\n").split("\n"):
        text = line.split("#")[0].strip()
        if not text:
            continue
        if text.startswith("OBJECT"):
            name = text.split(None, 1)[1].strip() if " " in text else ""
            fields = {}
        elif text == "END":
            if name and "filename" in fields:
                out.append(
                    Slot(
                        slot=name,
                        name=fields.get("name", "").strip('"'),
                        filename=fields.get("filename", "").strip('"'),
                        empty=fields.get("empty", "TRUE").upper() == "TRUE",
                    )
                )
            name, fields = "", {}
        elif "=" in text and name:
            key, _, value = text.partition("=")
            fields[key.strip()] = value.strip()
    return out
