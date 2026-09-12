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

The two records have sizes, and they are different sizes.  **Of the 1152 gaps
between consecutive narrow records, 812 are exactly 76 bytes** -- the
two-string record already known.  **Of the 82 gaps below 500 between wide
records, all 82 are 450, 458, 466 or 474**: a 450-byte base plus a multiple of
8, so a world object carries a short variable part as well as a fixed one.

What a world object does *not* carry is its position.  Searching a whole save
on four-byte alignment for a ``float32`` triple matching any position the
mission places finds **1 of 22** on one save and **4 of 27** on another --
chance, against the 96 plausible triples such a file holds -- and no axis
order or sign flip does better, nor does ``float64``.  So a save is not a
snapshot of where everything stands; whatever it keeps about placement is in
some other form.

**The pair is written two ways**, and reading only one of them hid a fifth of
the references.  The archive name is followed by the member name at 32 bytes
in the common record and at **128** in a second one.  A scan that looks only
at 32 does not report the others as unresolved -- it never counts them -- so
its hit rate flatters its coverage.  The wide record's names overlap what the
mission itself places and the narrow record's never do, on every save carrying
both, so they are two record types rather than one field written loosely.

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
#: The engine writes the pair two ways.  A 32-byte archive field is the
#: common one, and **a second record gives the archive 128 bytes**; a scan
#: that knows only the first does not report those as unresolved, it never
#: sees them at all.  184 of the 186 candidates at 128 name a real member.
MEMBER_AT = (32, 128)

#: The narrow record's own length.  812 of the 1152 gaps between consecutive
#: narrow records are exactly this, which is the 76-byte two-string record
#: `18-vocabulary.md` measures fields in.
PART_RECORD = 76

#: The wide record's.  Every one of the 82 gaps below 500 bytes is 450, 458,
#: 466 or 474 -- this base plus a multiple of ``WORLD_STEP`` -- so the record
#: has a fixed part and a short variable one.
#:
#: The multiple separates the world's furniture from the rest: **33 of the 37
#: scenery records take none and no scenery record takes more than one**, while
#: **no other record takes none at all** (26 at one step, 5 at two, 14 at
#: three).  It is not simply a property of the model either -- three of the 24
#: names appear with two different counts -- so it is at least partly the
#: instance's own state.  What it counts is open.
WORLD_RECORD = 450
WORLD_STEP = 8

#: Members whose name marks them as the map's furniture rather than a machine.
SCENERY = ("s_tree", "s_stone")

#: A ``uint16`` in the world record's fixed part.  On scenery it **rises
#: strictly in file order** in every save that has any, with ``NO_INDEX`` for
#: absent -- so it is an identity of some kind, assigned in order.  It is not
#: an index into the mission's object list, nor into its statics or its
#: non-statics: tested under every shift, nothing matches.
#:
#: On other object classes the same offset is not that field at all.  On 14
#: records of one save it holds the low half of ``1.0f``.  **The 450-byte
#: record is not one layout** -- the classes differ, which is what a dump of
#: live objects looks like.
WORLD_INDEX_AT = 0x1BE

#: What that field holds where it holds nothing.
NO_INDEX = 0xFFFF

#: The three ``int32`` that end a part record.  ``+64`` and ``+68`` are used
#: by different kinds of part rather than by all of them: ammunition carries
#: 10..20 at ``+64`` and **0 at +68 on every record**, weapons carry 1..7 at
#: ``+64``, and buildings, devices and chassis mostly carry 0 there and vary
#: at ``+68`` instead.  ``+72`` is the ordinal `18-vocabulary.md` describes.
PART_FIELDS = (64, 68, 72)

#: Archives a save is known to name.
ARCHIVES = ("objects.rlb", "effects.rlb")

_ARCHIVE = re.compile(rb"(" + rb"|".join(a.encode() for a in ARCHIVES) + rb")\x00")
#: A member name ends at a NUL.  The buffer after it is whatever was in
#: the heap, so without the terminator the wide record matches garbage:
#: 323 candidates instead of 186, and only 184 of them real either way.
_MEMBER = re.compile(rb"[A-Za-z_][A-Za-z0-9_.\-]{1,30}\x00")
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
    #: How far the member name sat after the archive name -- one of
    #: ``MEMBER_AT``.  The two are different records, not one written loosely:
    #: the wide one carries names a mission also places, the narrow one does
    #: not, on every save that has both.
    field: int = 32


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
    occupied = list(_ARCHIVE.finditer(data))
    for match in occupied:
        for width in MEMBER_AT:
            at = match.start() + width
            # The wide offset can land inside a neighbouring ``objects.rlb``
            # and report its ``rlb`` tail as a member.  A field never starts
            # inside another archive name.
            if any(a.start() <= at < a.end() for a in occupied):
                continue
            member = _MEMBER.match(data, at)
            if member:
                references.append(Reference(
                    match.group(1).decode(),
                    member.group().rstrip(b"\x00").decode(),
                    match.start(), width))

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
