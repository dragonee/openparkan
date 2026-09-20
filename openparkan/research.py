"""The research tree, ``MISSIONS/SCRIPTS/*.trf``.

Each ``.trf`` is an NRes archive of twelve streams tagged ``TRF0``..``TRFB``
and every member is named ``ResTree``, which is what it is: the technology
tree the player researches through.  ``Behavior.dll`` owns it.

The twelve streams are one table in columns.  **368 items**, and four of the
streams are exactly that many records wide:

```
TRF0   40 bytes each   four float32, then offsets and ids (below)
TRF1    1 byte  each   a category
TRF2    4 bytes each   how many prerequisites this item has
TRF4    4 bytes each   how many items it unlocks
TRF3   flat            the prerequisites, run together in item order
TRF5   flat            the unlocks, run together in item order
TRF6   text            part ids, NUL-terminated
TRF7   text            short codes -- 'L80mmRG'
TRF8   text            display names -- 'Large Rail Gun'; exactly 368 of them
TRF9   text            descriptions
TRFA   text            per-item stat templates for the UI panel
TRFB   4 bytes each    one packed word per TRF6 entry
```

``TRF3`` and ``TRF5`` are the same graph written twice, once as in-edges and
once as out-edges: across all 26 archives that carry them they are **exact
transposes of each other**, which is what pins the direction.

The engine agrees, and says two more things the data alone did not.
``MisLoad.dll``'s ``LoadResearch`` takes the streams in a fixed order and
**pairs each count with a pointer walked through the flat list four bytes at a
time** -- the same structure this reader builds, arrived at from the other
side.  It requires ten of the twelve and will do without only ``TRF3`` and
``TRF5``; those two are the only tags ever missing from a shipped archive, and
the three that lack one lack both.  And it **gates the whole load on the
directory's second count over ``TRF0`` being 3**, which it is on all 29, so
that field is a format version.

``TRF1`` is the odd one: every other stream is used where it lies, and this
one is copied into a buffer the loader allocates and zeroes first.  A stream
the engine takes a writable copy of is **state**, not a label: three bits,
``IN_TREE``, ``RESEARCHED`` and ``AVAILABLE``, which a completed research
rewrites.  A second gate sits beside it: the directory's second count over
``TRF1`` is read as a boolean, and ``iron3d.dll`` calls a tree with it set one
that "contains debugging information".  It is 0 on all 29.

A ``TRF0`` record is::

    float32 x4        the research energy and ore cost, then the build pair
    int32             byte offset into TRF7, this item's short code
    int32             byte offset into TRF8, this item's display name
    int32             byte offset into TRF9, this item's description
    int32             byte offset into TRFA, this item's stat template
    uint16            this item's own entry in TRFB -- the mapping, reversed
    byte x6           role, kind, sub-kind, branch, size, upgrade level

The last eight bytes were read here as two packed words until the engine's own
accessors said otherwise: ``MisLoad.dll`` hands out a **bounds-checked getter
per byte** for the six at ``+0x22``..``+0x27``.  The middle four are
``objects.dlb``'s classification line in numbers, which is how the game knows
a part's class without reading that file; ``iron3d.dll`` turns them into an
object ``Type`` (``object_type``).

The getters sit on ``IResearch``, the 32-slot interface at ``0x1000e18c``
that a query for interface ``0x502`` returns; the 23-slot table before it is
the research game object's own.

**``TRFB`` is the part-to-item mapping**, which was the open question here.
Each of its 395 entries is two ``uint16``: a byte offset into ``TRF6``, and the
index of the item that researches that part.  All 395 land on a string start,
all 368 items are named, and no item takes more than two parts -- the 27 that
take two are mounting pairs like ``e_tur_bb_01``/``e_tur_bt_01``, one turret
researched once.  Two files that share no bytes agree on all of it: **11455 of
11455** entries land on the item whose display name is the part's own name in
``objects.dlb``.  The record's ``uint16`` at ``+0x20`` points back, so the
mapping is stored both ways round.

Names are not unique -- 216 distinct over 368 items -- because the tree holds
several grades of the same thing.  Use the index.

Everything above is re-derived by ``uv run openparkan verify``.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass
from pathlib import Path

from .nres import NResArchive

#: Where the archives live, relative to the installation root.
DIRECTORY = ("MISSIONS", "SCRIPTS")

#: The twelve streams, in the order the tags run.
STREAMS = tuple(f"TRF{c}" for c in "0123456789AB")

#: Every member of a ``.trf`` carries this name.
MEMBER = "ResTree"

#: How many items the tree holds.  Fixed across every shipped archive.
ITEMS = 368

#: One ``TRF0`` record.
RECORD = 40

#: One ``TRFB`` entry: two ``uint16``, an offset into ``TRF6`` and the index
#: of the item that researches that part.
PART = 4

#: How many parts the library describes, and ``TRFB`` maps.
PARTS = 395

#: The order ``MisLoad.dll``'s reader (``0x10002fe0``) takes the streams in.
READ_ORDER = ("TRF0", "TRF1", "TRFB", "TRF6", "TRF7", "TRF8", "TRF9", "TRFA",
              "TRF2", "TRF3", "TRF4", "TRF5")

#: The two it will load without.  A missing tag anywhere else aborts.
OPTIONAL = ("TRF3", "TRF5")

#: The directory's second count over ``TRF0``, which the loader requires
#: before it reads anything: a format version.
VERSION = 3

#: The same field over ``TRF1``, which the loader keeps as a boolean and
#: ``IResearch`` slot 29 hands out: the tree **contains debugging
#: information**.  ``iron3d.dll`` says so in those words (``0x100605d6``)
#: unless ``Iron_3D.ini`` sets ``[CS] FULL_RESEARCH_TREE``.  0 on all 29.
STATE_FLAG = 0
DEBUG_INFO = STATE_FLAG

#: ``TRF1`` is three bits of state, as ``MisLoad.dll`` reads and writes them:
#: ``IN_TREE`` puts the item in this mission's tree, ``RESEARCHED`` marks it
#: done, and ``AVAILABLE`` means every prerequisite is done.  Completing a
#: research sets the last two and then gives ``AVAILABLE`` to every item in the
#: tree whose prerequisites are all in it and researched (``0x10002c10``).
AVAILABLE = 0x1
RESEARCHED = 0x2
IN_TREE = 0x4

#: The starting states the archives use.  0 is out of the tree, 2 researched
#: but not in it (the wildlife and the hero), 4 waiting on a prerequisite,
#: 5 open to research and 7 granted.
CATEGORIES = {
    0: "special",
    2: "creature",
    4: "main",
    5: "starting",
    7: "basic",
}

#: Record ``+0x23``..``+0x25`` are ``objects.dlb``'s classification line as
#: numbers: the kind, the first sub-kind, and a building's second.  Every
#: value maps onto one token across all 11,455 part entries.
PART_KINDS = {8: "BLD", 9: "SHS", 10: "AMM", 11: "DVC", 12: "WPN"}
PART_SUBS = {
    16: "HNG", 17: "BUN", 18: "INT", 19: "MIN", 20: "PLT", 21: "STR", 22: "TEL",
    23: "TOW", 24: "TMP", 25: "BRD", 26: "GEN", 28: "RUN", 29: "TWL", 30: "TWH",
    32: "SHS", 33: "TUR", 34: "TAR",
    49: "GUN", 50: "FLM", 51: "MIS", 52: "ROC", 53: "LAS", 55: "DVC", 56: "TAS",
    64: "DEF", 65: "RDR", 66: "REP", 67: "FSH", 68: "ARM", 69: "BRN", 70: "DSH",
    71: "ENG", 72: "BAT",
}
PART_BRANCHES = {80: "TUR", 81: "BLD", 82: "DEF", 83: "RDR", 84: "UPG"}
NO_BRANCH = 255

#: What ``iron3d.dll:0x1008a590`` makes of those bytes: a building's ``Type``
#: from its sub-kind -- a bunker's from its size -- and a turret's unit
#: ``Type`` from its role.  Anything else is 0.
KIND_BUILDING = 8
KIND_CHASSIS = 9
SUB_TURRET = 33
SUB_BUNKER = 17
BUILDING_TYPES = {
    16: 0x80000040, 18: 0x80000400, 19: 0x80000004, 20: 0x80000010,
    21: 0x80000008, 24: 0x80000200, 25: 0x80001000, 26: 0x80000002,
    29: 0x80100000, 30: 0x80200000,
}
BUNKER_TYPES = {1: 0x80010000, 2: 0x80020000, 3: 0x80040000}
TURRET_TYPES = {3: 0x1004000, 4: 0x1010000, 5: 0x1020000, 6: 0x1002000}
TURRET_DEFAULT_TYPE = 0x1008000

#: The second thing ``iron3d.dll`` derives from the same bytes
#: (``0x1008a690``): a **part category**, which is the warbot designer's
#: dispatch index and nothing else.  Its two readers both use it as a jump
#: table index -- fitting a part (``0x100519e0``, the table at ``0x10051b88``,
#: entered at category − 1) and taking one off (``0x10053a50``, the table at
#: ``0x10053cdc``, entered at the category itself).
SUB_CHASSIS = 32
SUB_BRAIN = 69
SUB_ARMOUR = 68
KIND_AMMUNITION = 10
KIND_DEVICE = 11
KIND_WEAPON = 12
#: A building part's second sub-kind decides its category.
BRANCH_CATEGORIES = {80: 1, 81: 5, 82: 1, 83: 2, 84: 1}
#: What a category means, and which fit routine the designer runs for it.
PART_CATEGORIES = {
    -1: "none",
    0: "chassis", 1: "turret", 2: "gun", 3: "device",
    4: "ammunition", 5: "building", 6: "brain", 7: "armour",
}


#: The same three bits under the names a second reading gave them
#: (``MisLoad.dll`` slot 26, ``0x10002aa0``).  Finishing a research needs
#: ``STATE_IN_TREE`` and sets the other two; an item in the tree becomes
#: available once every prerequisite is researched (slot 30, ``0x10002c10``).
#: So 7 is researched, 5 open, 4 locked, 2 researched outside the tree, and 0
#: out of the tree.
STATE_AVAILABLE = AVAILABLE
STATE_RESEARCHED = RESEARCHED
STATE_IN_TREE = IN_TREE


class ResearchFormatError(ValueError):
    pass


#: ``Item.role``: a bunker or tower turret, a battle, transport, builder or HQ
#: turret, the hero chassis, an animal; 255 on anything else.
ROLE_BUILDING_TURRET = 1
ROLE_BATTLE = 2
ROLE_TRANSPORT = 3
ROLE_BUILDER = 4
ROLE_HQ = 5
ROLE_HERO = 6
ROLE_ANIMAL = 7
ROLE_NONE = 255


@dataclass(frozen=True)
class Item:
    """One researchable item."""

    index: int
    name: str
    code: str
    category: int
    #: ``(ResearchEnergyCost, ResearchOreCost, BuildEnergyCost, BuildOreCost)``.
    #: The names are the game's own, out of ``objects.dlb``; see
    #: ``descriptions.py``.  Two resources, each charged twice -- once to
    #: research the item and once to build it.  None of them is a time.
    values: tuple[float, float, float, float]
    #: Indices of the items this one needs first.
    requires: tuple[int, ...]
    #: Indices of the items this one opens up.
    unlocks: tuple[int, ...]
    #: The part ids ``TRFB`` maps onto this item -- usually one, sometimes a
    #: pair of mounting variants researched together.
    parts: tuple[str, ...] = ()
    #: The ``uint16`` at record ``+0x20``: this item's own entry in ``TRFB``,
    #: so the part-to-item mapping is written both ways round.
    part_index: int = 0
    #: Record ``+0x22``..``+0x27``.  Six separate fields, not a packed word:
    #: the engine hands out a bounds-checked byte getter for each.  Measured
    #: ranges are 1..7 with 255 for none, 8..12, 16..72, 80..84 with 255 for
    #: none, 0..5 and 0..3: the ``role``, ``part_kind``, ``part_sub``,
    #: ``part_branch``, ``size`` and ``upgrade_level``.
    tail: tuple[int, ...] = ()
    #: ``TRF9``: the record's ``+0x18`` is a byte offset into it, and 150 of
    #: the 368 land on text; the rest on an empty string.
    description: str = ""

    @property
    def role(self) -> int:
        """``ROLE_*``: what a unit part makes a unit, 255 for anything else."""
        return self.tail[0] if self.tail else ROLE_NONE

    @property
    def size(self) -> int:
        """0 tiny, 1 small, 2 medium, 3 large, 4 hero and the A and N letters, 5 E."""
        return self.tail[4] if len(self.tail) > 4 else 0

    @property
    def upgrade_level(self) -> int:
        """``objects.dlb``'s UpgradeLevel for the part, on every shipped record."""
        return self.tail[5] if len(self.tail) > 5 else 0

    @property
    def part_kind(self) -> str:
        """``objects.dlb``'s kind token -- ``BLD``, ``SHS``, ``WPN``..."""
        return PART_KINDS.get(self.tail[1], "") if len(self.tail) > 1 else ""

    @property
    def part_sub(self) -> str:
        """The first sub-kind token -- ``BUN``, ``TUR``, ``GUN``..."""
        return PART_SUBS.get(self.tail[2], "") if len(self.tail) > 2 else ""

    @property
    def part_branch(self) -> str:
        """A building part's second sub-kind -- ``BLD``, ``DEF``, ``TUR``...; else ''."""
        return PART_BRANCHES.get(self.tail[3], "") if len(self.tail) > 3 else ""

    @property
    def object_type(self) -> int:
        """The object ``Type`` ``iron3d.dll`` derives from the record, or 0."""
        if len(self.tail) < 5:
            return 0
        kind, sub = self.tail[1], self.tail[2]
        if kind == KIND_CHASSIS and sub == SUB_TURRET:
            return TURRET_TYPES.get(self.tail[0], TURRET_DEFAULT_TYPE)
        if kind == KIND_BUILDING:
            if sub == SUB_BUNKER:
                return BUNKER_TYPES.get(self.tail[4], 0)
            return BUILDING_TYPES.get(sub, 0)
        return 0

    @property
    def part_category(self) -> int:
        """The designer's part category (``iron3d.dll:0x1008a690``), or −1.

        The same five bytes the ``Type`` comes from, read in a different
        order: a chassis 0, a turret 1, a gun 2, a device 3, ammunition 4, a
        building 5, a brain 6 and armour 7.  A building goes by its **second**
        sub-kind, so a building's turret is a turret and its radar is a gun.
        Anything the chain does not name is −1, which both readers bound out.
        """
        if len(self.tail) < 5:
            return -1
        kind, sub, branch = self.tail[1], self.tail[2], self.tail[3]
        if kind == KIND_CHASSIS:
            if sub == SUB_CHASSIS:
                return 0
            if sub == SUB_TURRET:
                return 1
        if kind == KIND_BUILDING and branch in BRANCH_CATEGORIES:
            return BRANCH_CATEGORIES[branch]
        if kind == KIND_WEAPON:
            return 2
        if kind == KIND_AMMUNITION:
            return 4
        if kind == KIND_DEVICE:
            return {SUB_BRAIN: 6, SUB_ARMOUR: 7}.get(sub, 3)
        return -1

    @property
    def in_tree(self) -> bool:
        """``IN_TREE``: part of this mission's tree at all."""
        return bool(self.category & IN_TREE)

    @property
    def researched(self) -> bool:
        """``RESEARCHED``: already researched at the mission's start."""
        return bool(self.category & RESEARCHED)

    @property
    def available(self) -> bool:
        """``AVAILABLE``: open to research at the mission's start."""
        return bool(self.category & AVAILABLE)

    @property
    def kind(self) -> str:
        return CATEGORIES.get(self.category, str(self.category))

    @property
    def research_cost(self) -> tuple[float, float]:
        """What researching it costs: (energy, ore).  Zero when it is free."""
        return self.values[0], self.values[1]

    @property
    def build_cost(self) -> tuple[float, float]:
        """What building it costs: (energy, ore)."""
        return self.values[2], self.values[3]

    @property
    def root(self) -> bool:
        """True when nothing has to be researched before it."""
        return not self.requires

    @property
    def leaf(self) -> bool:
        """True when it opens nothing further."""
        return not self.unlocks


@dataclass(frozen=True)
class Tree:
    """One ``.trf``: the whole tree as that mission sees it."""

    source: Path
    items: tuple[Item, ...]
    #: Every part id in ``TRFB`` order, which is its own index space: a
    #: record's ``part_index`` indexes this, not ``items``.
    part_ids: tuple[str, ...] = ()

    def __len__(self) -> int:
        return len(self.items)

    def __getitem__(self, index: int) -> Item:
        return self.items[index]

    @property
    def edges(self) -> int:
        return sum(len(i.requires) for i in self.items)

    @property
    def roots(self) -> tuple[Item, ...]:
        return tuple(i for i in self.items if i.root)

    def find(self, name: str) -> tuple[Item, ...]:
        """Every item whose display name matches, case-insensitively."""
        low = name.lower()
        return tuple(i for i in self.items if low in i.name.lower())

    @property
    def parts(self) -> dict[str, int]:
        """Every part id in ``TRFB``, against the item that researches it."""
        return {pid: item.index for item in self.items for pid in item.parts}

    def part_at(self, index: int) -> str:
        """The part id at a ``TRFB`` index, which is what a record's
        ``part_index`` holds."""
        return self.part_ids[index] if 0 <= index < len(self.part_ids) else ""

    def item_for(self, part: str) -> Item | None:
        """The item that researches a part id, case-insensitively."""
        low = part.lower()
        for item in self.items:
            if any(p.lower() == low for p in item.parts):
                return item
        return None


def _text(blob: bytes, offset: int) -> str:
    end = blob.find(b"\0", offset)
    if offset < 0 or offset >= len(blob):
        return ""
    return blob[offset : end if end >= 0 else len(blob)].decode("latin-1")


def _slices(counts: tuple[int, ...], flat: tuple[int, ...]) -> list[tuple[int, ...]]:
    out: list[tuple[int, ...]] = []
    at = 0
    for count in counts:
        out.append(tuple(flat[at : at + count]))
        at += count
    return out


def parse(data: bytes, source: Path | None = None) -> Tree:
    """Read one ``.trf``.  Raises unless the columns agree with each other."""
    where = source.name if source is not None else "<bytes>"
    archive = NResArchive(data)
    blob = {entry.tag: archive.read(entry) for entry in archive.entries}
    missing = [tag for tag in ("TRF0", "TRF1", "TRF7", "TRF8") if tag not in blob]
    if missing:
        raise ResearchFormatError(f"{where}: no {', '.join(missing)}")

    count = len(blob["TRF1"])
    if len(blob["TRF0"]) != count * RECORD:
        raise ResearchFormatError(
            f"{where}: TRF0 is {len(blob['TRF0'])} bytes, not {count} x {RECORD}"
        )

    requires: list[tuple[int, ...]] = [()] * count
    unlocks: list[tuple[int, ...]] = [()] * count
    if "TRF2" in blob and "TRF3" in blob:
        counts = struct.unpack(f"<{count}i", blob["TRF2"])
        flat = struct.unpack(f"<{len(blob['TRF3']) // 4}i", blob["TRF3"])
        if sum(counts) != len(flat):
            raise ResearchFormatError(
                f"{where}: TRF2 counts {sum(counts)} prerequisites, TRF3 holds {len(flat)}"
            )
        requires = _slices(counts, flat)
    if "TRF4" in blob and "TRF5" in blob:
        counts = struct.unpack(f"<{count}i", blob["TRF4"])
        flat = struct.unpack(f"<{len(blob['TRF5']) // 4}i", blob["TRF5"])
        if sum(counts) != len(flat):
            raise ResearchFormatError(
                f"{where}: TRF4 counts {sum(counts)} unlocks, TRF5 holds {len(flat)}"
            )
        unlocks = _slices(counts, flat)

    parts: list[list[str]] = [[] for _ in range(count)]
    order: list[str] = []
    if "TRFB" in blob and "TRF6" in blob:
        table, names = blob["TRFB"], blob["TRF6"]
        for entry in range(len(table) // PART):
            at, item = struct.unpack_from("<HH", table, entry * PART)
            if item >= count or at >= len(names):
                raise ResearchFormatError(
                    f"{where}: TRFB entry {entry} names item {item} at {at}")
            part = _text(names, at)
            parts[item].append(part)
            order.append(part)

    items: list[Item] = []
    for index in range(count):
        fields = struct.unpack_from("<4f4iH6B", blob["TRF0"], index * RECORD)
        values = fields[:4]
        code_at, name_at, description_at, _panel, part_index = fields[4:9]
        tail = fields[9:]
        items.append(
            Item(
                index=index,
                name=_text(blob["TRF8"], name_at),
                code=_text(blob["TRF7"], code_at),
                category=blob["TRF1"][index],
                values=(values[0], values[1], values[2], values[3]),
                requires=requires[index],
                unlocks=unlocks[index],
                parts=tuple(parts[index]),
                part_index=part_index,
                tail=tuple(tail),
                description=_text(blob.get("TRF9", b""), description_at),
            )
        )
    return Tree(source=source or Path(where), items=tuple(items),
                part_ids=tuple(order))


def read(path: Path) -> Tree:
    """Read the tree at ``path``."""
    return parse(path.read_bytes(), path)


def trees(game: Path) -> list[Path]:
    """Every ``.trf`` the installation ships, sorted."""
    return sorted(game.joinpath(*DIRECTORY).glob("*.trf"))


def render(tree: Tree, category: int | None = None) -> list[str]:
    """The tree as indented lines, each root followed by what it opens up.

    An item can be reached by more than one path, so a repeat is marked
    rather than expanded again.
    """
    lines: list[str] = []
    seen: set[int] = set()

    def walk(item: Item, depth: int, trail: frozenset[int]) -> None:
        mark = ""
        if item.index in seen and item.unlocks:
            mark = "  (above)"
        lines.append(
            f"{'    ' * depth}{item.name}"
            f"{f'  [{item.code}]' if item.code else ''}{mark}"
        )
        if mark or item.index in trail:
            return
        seen.add(item.index)
        for nxt in item.unlocks:
            if nxt < len(tree):
                walk(tree[nxt], depth + 1, trail | {item.index})

    for item in tree.items:
        if not item.root:
            continue
        if category is not None and item.category != category:
            continue
        walk(item, 0, frozenset())
        lines.append("")
    return lines
