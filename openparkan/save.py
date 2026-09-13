"""Save games, ``SAVE/*.sav``.

A save is **not a designed file format** in the sense of one record layout.
It is a sequence of sections the game's save routine writes one after
another, and most of the bytes inside them are the classes' own memory:
32- and 128-byte string fields with their tails never zeroed, and a few
words of stack.  But the *sequence* is fixed, every section says how long it
is or takes its count from somewhere the reader can reach, and **all six
shipped saves parse to their last byte**::

    char[4]   "SLOT"
    uint8     version, 1 -- a version 0 save lacks the per-clan word below
    uint8     difficulty: 0 EASY, 1 MEDIUM, 2 HARD
    int32     length of the mission path
    char      path[length]      'missions/campaign/campaign.05/mission.01/'

    int32     size, then the world: every game object, one record each
    int32     clan count
    int32     objective count, then per objective
                  char[255] text, int32 state, int32 exempt
    per clan  int32 unknown word  (only when version >= 1)
              int32 x minds       the clan's mind list: a unit id or -1
    int32     count, then 24-byte records of unknown meaning
    int32     count, then per unit design
                  uint32 0xF0F1, uint32 Type, 112-byte components depth first
    int32 x3  1, then two ids or -1
    int32     clan count again
    per clan  int32 size, then that clan's AI state

The one count the file does not carry is the mind list's length: it is each
clan's ``minds`` word from the mission's ``data.tma``, so a full parse needs
the mission (``read(path, game)``).  The world needs nothing else.

**The world** is a 48-byte header -- a 1 and the queue's game time -- then one
record per object, written parent before child::

    uint32    id: class in the top byte, a serial below
    char[128] archive       'objects.rlb', or empty
    char[128] member        'fr_l_gener', 'DATA\\MAPS\\KM_14\\land'
    int32     parent id (0 for none), int32 the parent's slot
    uint32    bytes that follow this 276-byte head
    int32     property 0x803 of an object whose type answers 3, else 0
    uint32    chunk count n, then n sizes, then the chunks

For a model object chunk 0 is a byte per owner saying how many chunks that
owner wrote: the part list, the model, the control system, the wizard and the
behaviour, in that order, absent owners skipped.  The part list is 76-byte
records -- archive, member, the parent part's id, the node or slot on it, the
part's own id; the model's chunk is the object's scale; the control system's
opens on a flags word, the orientation as a quaternion and the position.

That is how a save refers to archive members: **1342 references**, and the
old scan that found them by shape is kept (``references``) because every one
of them now has a home -- 184 record heads, 906 part records, 201 in control
chunks and 51 in unit designs.  See ``docs/17-saves.md``.

Everything above is re-derived by ``uv run openparkan verify``.
"""

from __future__ import annotations

import re
import struct
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

from . import mission as _mission
from .objects import DAT_COMPONENT, DAT_MAGIC, NAME_FIELD, Component, ResourceRef

#: The directory the game keeps saves in.
DIRECTORY = "SAVE"

#: The slot index beside the saves, in the engine's ``OBJECT``/``END`` text.
SLOTS = "saveslots.cfg"

#: The first four bytes of every save.
MAGIC = b"SLOT"

#: The version byte, 1 in all six shipped saves.  The loader
#: (``iron3d.dll:0x100a2bd0``) reads a clan's leading word only when it is at
#: least 1, so it is a real version.
VERSION = 1

#: The byte after it is the difficulty the game was saved at.  The writer
#: takes it from the settings object's ``+0x150`` (``iron3d.dll:0x100a1663``),
#: the loader puts it back there, and ``iron3d.dll:0x10076010`` reads that
#: field as the level ratio's ``EASY``, ``MEDIUM``, ``HARD``.  It is not
#: campaign against single: that the one skirmish save holds 1 and the five
#: campaign saves 0 is the difficulty they were played at.
EASY, MEDIUM, HARD = 0, 1, 2
LEVELS = {EASY: "EASY", MEDIUM: "MEDIUM", HARD: "HARD"}

#: The longest mission path worth believing.
MAX_PATH = 260

#: A length, in front of the world and of each clan's AI state.
LENGTH = 4

#: The world's own header: a 1 and the queue's game time (the value
#: ``SetGameTime`` sets), then ten words of stack (``World3D.dll:0x10009a90``).
WORLD_HEADER = 48

#: A world record's fixed head (``World3D.dll:0x10009bc0``).
RECORD_HEAD = 0x114
#: Where its fields sit in that head.
RECORD_ARCHIVE = 4
RECORD_MEMBER = 0x84
RECORD_NAME = 128
RECORD_PARENT = 0x104
#: The top byte of an id says what the object is: 0x11 the landscape, 0x17 the
#: sky, 0x1b a research tree, and for a model the ``objects.rlb`` tag of the
#: member it names -- 0x13 ``FORT``, a building; 0x14 ``BTLU``, a unit or
#: creature; 0x19 ``BULL``, a round in flight; 0x1a ``STAT``, scenery
#: (*measured*, 184 of 184).
CLASS_SHIFT = 24
KIND_LAND, KIND_BUILDING, KIND_UNIT, KIND_SKY = 0x11, 0x13, 0x14, 0x17
KIND_ROUND, KIND_SCENERY, KIND_RESEARCH = 0x19, 0x1A, 0x1B
KIND_TAGS = {KIND_BUILDING: "FORT", KIND_UNIT: "BTLU", KIND_ROUND: "BULL",
             KIND_SCENERY: "STAT"}
#: The two kinds that save three chunks -- chunk 0, the scale and the control
#: chunk.  Their control chunk is 143 bytes and grows in 8-byte steps, which
#: is all the "450 plus a multiple of 8" ever was: 33 of 37 scenery records
#: take no step, 46 of 47 rounds take one to three.  What an 8-byte step
#: holds is not established.
SMALL_KINDS = (KIND_ROUND, KIND_SCENERY)
CONTROL_SMALL = 143

#: An objective: 255 bytes of text and two words (``iron3d.dll:0x1006b180``).
#: The first word is the state the script sets -- 1 complete, -1 failed, 0
#: open -- and an objective whose second word is non-zero does not hold up
#: the mission's completion (``iron3d.dll:0x1006b130``).
OBJECTIVE_TEXT = 255
OBJECTIVE_DONE, OBJECTIVE_OPEN, OBJECTIVE_FAILED = 1, 0, -1

#: The 24-byte records the game object's member at ``+0x700`` writes
#: (``iron3d.dll:0x10081990``).  What they are is not established.
EXTRA_RECORD = 24

#: A saved unit design is an ``.dat`` assembly written in the ``.dat``'s own
#: layout; the list holds at most five.
DESIGNS_MAX = 5

#: A model object's part record (``AniMesh.dll:0x10003660`` hands the list
#: over, ``0x100036a0`` reads it back).  The list's first entry is the object
#: itself, id 0, and is not saved.
PART_RECORD = 76
PART_PARENT, PART_ATTACH, PART_ID = 64, 68, 72
#: The id a part's ``parent`` holds when it hangs off the object itself.
ROOT_PART = 0

#: Chunk 0's owners, in the order their counts are written
#: (``AniMesh.dll:0x10001c20``).
OWNERS = ("parts", "model", "control", "wizard", "behaviour")

#: The control system's chunk: a flags word, then the orientation ``(w, x, y,
#: z)``, then the position.  An upright object turned by the mission's angle
#: ``a`` holds ``(cos a/2, 0, 0, -sin a/2)`` (*measured*, 34 of 34).
CONTROL_ORIENTATION = 4
CONTROL_POSITION = 20

#: Members whose name marks them as the map's furniture rather than a machine.
SCENERY = ("s_tree", "s_stone")

#: A clan's AI state is 2036 bytes, four more for every script variable
#: (``varset.var`` declares 231, so 2960 on every clan without more), and 28
#: for each open problem of one kind and four for each entry of another
#: (``ai.dll:0x100020f0``).
AI_FIXED = 2036
AI_PER_VARIABLE = 4

#: In the engine's two-string records the member name sits this far after the
#: archive name -- 32 in a part record or a design, 128 in a world record's
#: head.  The scan that finds them by shape is kept: it is how the references
#: are checked against the archives.
MEMBER_AT = (32, 128)

#: The smallest world record, a scenery piece or round with a 143-byte control
#: chunk, and the step its control chunk grows by.  Measured on the gaps
#: between record heads before the record was read.
WORLD_RECORD = 450
WORLD_STEP = 8

#: On a 450-byte record this offset from the archive name is the next
#: record's id, 450 bytes on, whose serial rises in file order.
WORLD_INDEX_AT = 0x1BE
NO_INDEX = 0xFFFF

#: Archives a save is known to name.
ARCHIVES = ("objects.rlb", "effects.rlb")

_ARCHIVE = re.compile(rb"(" + rb"|".join(a.encode() for a in ARCHIVES) + rb")\x00")
#: A member name ends at a NUL.  The buffer after it is whatever was in
#: the heap, so without the terminator the wide record matches garbage.
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
    #: ``MEMBER_AT``.
    field: int = 32


@dataclass(frozen=True)
class Chunk:
    """A run of bytes one owner wrote, as a file offset and a size."""

    offset: int
    size: int


@dataclass(frozen=True)
class Part:
    """One entry of a model object's part list."""

    archive: str
    member: str
    #: The id of the part this one hangs off; ``ROOT_PART`` for the object.
    parent: int
    #: The node of the parent's mesh (an external part) or the slot of its
    #: controller (an internal one) -- ``AniMesh.dll:0x10003760``.
    attach: int
    #: This part's own id, unique within the object.
    id: int


@dataclass(frozen=True)
class WorldObject:
    """One record of the world section."""

    offset: int
    id: int
    archive: str
    member: str
    parent: int
    parent_slot: int
    property: int
    chunks: tuple[Chunk, ...]
    #: Chunk 0's bytes on a model object: how many chunks each of ``OWNERS``
    #: wrote.  Empty on the landscape, the sky and a research tree.
    counts: bytes = b""
    parts: tuple[Part, ...] = ()
    scale: tuple[float, float, float] | None = None
    orientation: tuple[float, float, float, float] | None = None
    position: tuple[float, float, float] | None = None

    @property
    def kind(self) -> int:
        """The object's class, the id's top byte."""
        return self.id >> CLASS_SHIFT

    @property
    def serial(self) -> int:
        return self.id & ((1 << CLASS_SHIFT) - 1)

    @property
    def size(self) -> int:
        """The whole record: head, chunk table and chunks."""
        end = self.chunks[-1].offset + self.chunks[-1].size if self.chunks else 0
        return max(end - self.offset, RECORD_HEAD + LENGTH)


@dataclass(frozen=True)
class Objective:
    text: str
    state: int
    exempt: int


@dataclass(frozen=True)
class Design:
    """A unit design the player keeps: an assembly in ``.dat`` layout."""

    kind: int
    components: tuple[Component, ...]


@dataclass(frozen=True)
class Slot:
    """One entry of ``saveslots.cfg``."""

    slot: str
    name: str
    filename: str
    empty: bool


@dataclass(frozen=True)
class Save:
    """One ``.sav``: its header, its world and, given the mission, the rest."""

    source: Path
    version: int
    level: int
    mission: str
    #: The map directory under ``DATA/MAPS``, from the first reference to one.
    map: str
    #: Every ``.trf`` research tree the save names, in file order.
    trees: tuple[str, ...]
    #: Every archive member reference the scan recovered.
    references: tuple[Reference, ...]
    #: The world section's bytes, and the game clock its header holds.
    world: Chunk = Chunk(0, 0)
    clock: int = 0
    objects: tuple[WorldObject, ...] = ()
    #: Filled by a parse that knows each clan's mind count; empty otherwise.
    clans: int = 0
    objectives: tuple[Objective, ...] = ()
    clan_words: tuple[int, ...] = ()
    minds: tuple[tuple[int, ...], ...] = ()
    extras: tuple[bytes, ...] = ()
    designs: tuple[Design, ...] = ()
    tail: tuple[int, int, int] = (0, 0, 0)
    ai: tuple[Chunk, ...] = ()
    #: True when the parse reached the last byte exactly.
    complete: bool = False

    @property
    def level_name(self) -> str:
        return LEVELS.get(self.level, "HARD")

    @property
    def members(self) -> tuple[tuple[str, str], ...]:
        """The distinct ``(archive, member)`` pairs, in first-seen order."""
        seen: list[tuple[str, str]] = []
        for ref in self.references:
            pair = (ref.archive, ref.member)
            if pair not in seen:
                seen.append(pair)
        return tuple(seen)


def _text(raw: bytes) -> str:
    return raw.split(b"\0", 1)[0].decode("latin-1")


class _Cursor:
    def __init__(self, data: bytes, at: int, where: str) -> None:
        self.data, self.at, self.where = data, at, where

    def take(self, n: int) -> bytes:
        if n < 0 or self.at + n > len(self.data):
            raise SaveFormatError(f"{self.where}: {n} bytes wanted at {self.at}, "
                                  f"{len(self.data) - self.at} left")
        out = self.data[self.at:self.at + n]
        self.at += n
        return out

    def i32(self) -> int:
        return struct.unpack("<i", self.take(4))[0]

    def u32(self) -> int:
        return struct.unpack("<I", self.take(4))[0]


def _model_fields(data: bytes, chunks: Sequence[Chunk]):
    """Chunk 0's owner counts, the part list, the scale and the placement."""
    if not chunks:
        return b"", (), None, None, None
    counts = data[chunks[0].offset:chunks[0].offset + chunks[0].size]
    if len(counts) < 3 or len(chunks) < 1 + counts[0] + 2:
        return counts, (), None, None, None
    parts: list[Part] = []
    for chunk in chunks[1:1 + counts[0]]:
        if chunk.size % PART_RECORD:
            return counts, (), None, None, None
        for at in range(chunk.offset, chunk.offset + chunk.size, PART_RECORD):
            parent, attach, ident = struct.unpack_from("<3i", data, at + PART_PARENT)
            parts.append(Part(_text(data[at:at + NAME_FIELD]),
                              _text(data[at + NAME_FIELD:at + 2 * NAME_FIELD]),
                              parent, attach, ident))
    model = chunks[1 + counts[0]]
    control = chunks[2 + counts[0]]
    scale = orientation = position = None
    if model.size == 12:
        scale = struct.unpack_from("<3f", data, model.offset)
    if control.size >= CONTROL_POSITION + 12:
        orientation = struct.unpack_from("<4f", data, control.offset + CONTROL_ORIENTATION)
        position = struct.unpack_from("<3f", data, control.offset + CONTROL_POSITION)
    return counts, tuple(parts), scale, orientation, position


def _world(data: bytes, start: int, size: int, where: str):
    """The world section: its clock and every object record, to the byte."""
    end = start + size
    if size < WORLD_HEADER:
        raise SaveFormatError(f"{where}: a {size}-byte world has no header")
    clock = struct.unpack_from("<I", data, start + 4)[0]
    cur = _Cursor(data, start + WORLD_HEADER, where)
    found: list[WorldObject] = []
    while cur.at < end:
        at = cur.at
        head = cur.take(RECORD_HEAD)
        ident = struct.unpack_from("<I", head, 0)[0]
        parent, slot, follows, prop = struct.unpack_from("<iiIi", head, RECORD_PARENT)
        if at + RECORD_HEAD + follows > end:
            raise SaveFormatError(f"{where}: object at {at} runs past the world")
        count = cur.u32()
        if LENGTH + LENGTH * count > follows:
            raise SaveFormatError(f"{where}: object at {at} claims {count} chunks")
        sizes = [cur.u32() for _ in range(count)]
        chunks = []
        for n in sizes:
            chunks.append(Chunk(cur.at, n))
            cur.take(n)
        if cur.at != at + RECORD_HEAD + follows:
            raise SaveFormatError(f"{where}: object at {at} is {cur.at - at} bytes, "
                                  f"its head says {RECORD_HEAD + follows}")
        counts, parts, scale, orientation, position = (
            _model_fields(data, chunks) if _text(head[RECORD_ARCHIVE:RECORD_MEMBER])
            else (b"", (), None, None, None))
        found.append(WorldObject(
            offset=at, id=ident,
            archive=_text(head[RECORD_ARCHIVE:RECORD_ARCHIVE + RECORD_NAME]),
            member=_text(head[RECORD_MEMBER:RECORD_MEMBER + RECORD_NAME]),
            parent=parent, parent_slot=slot, property=prop, chunks=tuple(chunks),
            counts=counts, parts=parts, scale=scale, orientation=orientation,
            position=position))
    if cur.at != end:
        raise SaveFormatError(f"{where}: the world ends at {cur.at}, not {end}")
    return clock, tuple(found)


def _design_tree(cur: _Cursor) -> list[Component]:
    raw = cur.take(DAT_COMPONENT)
    flags, attach = struct.unpack_from("<Ii", raw, 2 * NAME_FIELD)
    class_id, children = struct.unpack_from("<Ii", raw, DAT_COMPONENT - 8)
    out = [Component(
        ref=ResourceRef(_text(raw[:NAME_FIELD]), _text(raw[NAME_FIELD:2 * NAME_FIELD])),
        label=_text(raw[72:104]), flags=flags, attach_node=attach,
        class_id=class_id, child_count=children)]
    if not 0 <= children < 64:
        raise SaveFormatError(f"{cur.where}: a design component owns {children} children")
    for _ in range(children):
        out.extend(_design_tree(cur))
    return out


def parse(data: bytes, source: Path | None = None,
          minds: Sequence[int] | None = None) -> Save:
    """Read one save.

    The header is parsed strictly and the world section always.  The sections
    after the world need each clan's mind count, which only the mission's
    ``data.tma`` holds: pass ``minds`` and the parse runs to the end of the
    file and fails loudly if it does not end there.
    """
    where = source.name if source is not None else "<bytes>"
    if data[:4] != MAGIC:
        raise SaveFormatError(f"{where}: not a save; opens {data[:4]!r}, not {MAGIC!r}")
    if len(data) < 10:
        raise SaveFormatError(f"{where}: {len(data)} bytes, too short for a header")
    version, level = data[4], data[5]
    if version != VERSION:
        raise SaveFormatError(f"{where}: version {version}, not {VERSION}")
    (length,) = struct.unpack_from("<I", data, 6)
    if not 0 < length <= MAX_PATH or 10 + length > len(data):
        raise SaveFormatError(f"{where}: mission path length {length}")
    mission = data[10:10 + length].decode("latin-1")

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
            # and report its ``rlb`` tail as a member.
            if any(a.start() <= at < a.end() for a in occupied):
                continue
            member = _MEMBER.match(data, at)
            if member:
                references.append(Reference(
                    match.group(1).decode(),
                    member.group().rstrip(b"\x00").decode(),
                    match.start(), width))

    fields: dict = dict(
        source=source or Path(where), version=version, level=level,
        mission=mission, map=found.group(1).decode("latin-1") if found else "",
        trees=tuple(trees), references=tuple(references))

    cur = _Cursor(data, 10 + length, where)
    if cur.at == len(data):
        return Save(**fields)
    size = cur.u32()
    start = cur.at
    cur.take(size)
    clock, objects_ = _world(data, start, size, where)
    fields.update(world=Chunk(start, size), clock=clock, objects=objects_)
    if minds is None:
        return Save(**fields)

    clans = cur.i32()
    if clans != len(minds):
        raise SaveFormatError(f"{where}: {clans} clans, the mission has {len(minds)}")
    objectives = []
    for _ in range(cur.i32()):
        text = _text(cur.take(OBJECTIVE_TEXT))
        objectives.append(Objective(text, cur.i32(), cur.i32()))
    words, lists = [], []
    for count in minds:
        words.append(cur.i32() if version >= 1 else 0)
        lists.append(tuple(cur.i32() for _ in range(count)))
    extras = tuple(cur.take(EXTRA_RECORD) for _ in range(cur.i32()))
    designs = []
    for _ in range(cur.i32()):
        magic, kind = cur.u32(), cur.u32()
        if magic != DAT_MAGIC:
            raise SaveFormatError(f"{where}: a design opens {magic:#x}, not {DAT_MAGIC:#x}")
        designs.append(Design(kind, tuple(_design_tree(cur))))
    tail = (cur.i32(), cur.i32(), cur.i32())
    again = cur.i32()
    if again != clans:
        raise SaveFormatError(f"{where}: {again} AI blocks for {clans} clans")
    ai = []
    for _ in range(clans):
        n = cur.u32()
        ai.append(Chunk(cur.at, n))
        cur.take(n)
    if cur.at != len(data):
        raise SaveFormatError(f"{where}: {len(data) - cur.at} bytes after the last clan")
    fields.update(clans=clans, objectives=tuple(objectives), clan_words=tuple(words),
                  minds=tuple(lists), extras=extras, designs=tuple(designs),
                  tail=tail, ai=tuple(ai), complete=True)
    return Save(**fields)


def read(path: Path, game: Path | None = None) -> Save:
    """Read the save at ``path``; given the installation, all of it."""
    data = path.read_bytes()
    first = parse(data, path)
    if game is None:
        return first
    tma = game / Path(first.mission.replace("\\", "/")) / "data.tma"
    return parse(data, path, [c.minds for c in _mission.load(tma).clans])


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
