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
the engine takes a writable copy of is **state**, not a label -- the category
is where an item *starts*, and 5 reads as already available.  A second gate
sits beside it: the directory's second count over ``TRF1`` is read as a
boolean, and it is 0 on all 29, so nothing shipped turns that switch on.

A ``TRF0`` record is::

    float32 x4        the research energy and ore cost, then the build pair
    int32             byte offset into TRF7, this item's short code
    int32             byte offset into TRF8, this item's display name
    int32             an id, not the item's own index
    int32             byte offset into TRFA, this item's stat template
    uint16            this item's own entry in TRFB -- the mapping, reversed
    byte x6           six separate fields, one getter each

The last eight bytes were read here as two packed words until the engine's own
accessors said otherwise: ``MisLoad.dll`` hands out a **bounds-checked getter
per byte** for the six at ``+0x22``..``+0x27``, and the data agrees -- each
holds between 4 and 33 distinct values across all 29 archives, which the bytes
of a packed word would not.  What any of the six means is open.

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

#: The same field over ``TRF1``, which the loader keeps as a boolean.  No
#: shipped archive sets it, so what it switches is unknown.
STATE_FLAG = 0

#: The starting state in ``TRF1`` -- the engine copies this stream into a
#: writable buffer, so it is where an item begins rather than what it is.
#: 4 is the bulk of the tree and 7 the small equipment; 2 is the wildlife and
#: the hero, which are not researched.
CATEGORIES = {
    0: "special",
    2: "creature",
    4: "main",
    5: "starting",
    7: "basic",
}


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
    #: none, 0..5 and 0..3.  The first is the ``role``, the fifth the ``size``
    #: and the sixth the ``upgrade_level``; the middle three are open.
    tail: tuple[int, ...] = ()

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
        code_at, name_at, _id, _panel, part_index = fields[4:9]
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
