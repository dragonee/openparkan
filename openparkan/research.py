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
transposes of each other**, which is what pins the direction.  A ``TRF0``
record is::

    float32 x4        the research energy and ore cost, then the build pair
    int32             byte offset into TRF7, this item's short code
    int32             byte offset into TRF8, this item's display name
    int32             an id, not the item's own index
    int32             byte offset into TRFA, this item's stat template
    uint32            (class << 16) | counter
    uint32            four packed bytes

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

#: The category byte in ``TRF1``.  4 is the bulk of the tree and 7 the small
#: equipment; 2 is the wildlife and the hero, which are not researched.
CATEGORIES = {
    0: "special",
    2: "creature",
    4: "main",
    5: "starting",
    7: "basic",
}


class ResearchFormatError(ValueError):
    pass


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

    items: list[Item] = []
    for index in range(count):
        *values, code_at, name_at, _id, _panel, _tag, _packed = struct.unpack_from(
            "<4f6i", blob["TRF0"], index * RECORD
        )
        items.append(
            Item(
                index=index,
                name=_text(blob["TRF8"], name_at),
                code=_text(blob["TRF7"], code_at),
                category=blob["TRF1"][index],
                values=(values[0], values[1], values[2], values[3]),
                requires=requires[index],
                unlocks=unlocks[index],
            )
        )
    return Tree(source=source or Path(where), items=tuple(items))


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
