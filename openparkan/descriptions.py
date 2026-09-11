"""The parts database, ``objects.dlb``.

An NRes archive of **395 `DSCR` members**, one per part id, and they are the
same 395 ids in the same order as the research tree's ``TRF6``
(`16-research.md`).  Each member is plain text with tagged lines, and it is
the authority on what a part is: the game wrote its own field names here.

```
//G3:L14                       group, and a level
//B:WPN:GUN:MK2:A3             size, kind, sub-kind, mark, and one more slot
#1L152mmC                      short code
#2Large Cannon                 display name
#6UpgradeLevel=2
#7ResearchEnergyCost=18
#8ResearchOreCost=50
#9BuildEnergyCost=18
#ABuildOreCost=50
@G@Weight       @B,weight,G,t,5,1@        a row of the UI's stat panel
@G@Rate of fire @B,Frate,G,1/s,5,@
```

**This names the four float32 of a research-tree record.**  They are
``ResearchEnergyCost``, ``ResearchOreCost``, ``BuildEnergyCost`` and
``BuildOreCost`` -- two resources, each charged twice, once to research a
thing and once to build it.  Joined on the short code, **311 of 329 items
match all four exactly**, and the rest are short codes the file reuses.  So
they are costs and none of them is a time.

``#5`` names the weapon an ammunition clip belongs to (`Small Autocannon
S75Can`), which is a second, independent statement of the gun-to-clip link
that the assembly tree gives (`18-vocabulary.md`).

Everything above is re-derived by ``uv run openparkan verify``.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path

from .nres import NResArchive

#: The archive, in the installation root.
LIBRARY = "objects.dlb"

#: The tag every member carries.
TAG = "DSCR"

#: How many parts it describes.
PARTS = 395

#: The size letter of the classification line, as in a part id.
SIZES = {"B": "large", "M": "medium", "L": "small", "T": "tiny", "H": "huge"}

#: The group in the ``//G<n>:L<n>`` line: the catalogue's top-level tab, and
#: the kinds that belong to it.  Armament is one tab holding both the weapons
#: and the ammunition they take.
GROUPS = {1: "buildings", 2: "chassis", 3: "armament", 4: "devices"}
GROUP_KINDS = {1: {"BLD"}, 2: {"SHS", "ANM"}, 3: {"WPN", "AMM"}, 4: {"DVC"}}

#: What ``UpgradeLevel`` bands the tech level into.  The bands rise and touch
#: only at their edges; see ``docs/19-descriptions.md``.
BANDS = {0: (0, 1), 1: (2, 8), 2: (9, 16), 3: (14, 21)}

#: The kind slot.
KINDS = {
    "DVC": "device",
    "SHS": "chassis",
    "WPN": "weapon",
    "BLD": "building",
    "AMM": "ammunition",
    "ANM": "creature",
}

#: The five ``key=value`` slots, in the order the file writes them.
NUMBERS = (
    ("6", "UpgradeLevel"),
    ("7", "ResearchEnergyCost"),
    ("8", "ResearchOreCost"),
    ("9", "BuildEnergyCost"),
    ("A", "BuildOreCost"),
)

_GROUP = re.compile(r"^//G(\d+):L(\d+)\s*$")
#: The classification line is variable: the mark is the ``MK<n>`` token and
#: anything between the kind and it is a chain of sub-kinds.
_CLASS = re.compile(r"^//([A-Z]+):([A-Z]+):(.+?)\s*$")
_MARK = re.compile(r"^MK\d+$")
_SLOT = re.compile(r"^#(\w)(.*)$")
_STAT = re.compile(r"^@G@(.*?)\s*@B,([^,]*),G,([^,]*),")


class DescriptionFormatError(ValueError):
    pass


@dataclass(frozen=True)
class Stat:
    """One row of the part's stat panel: a label, the field behind it, a unit."""

    label: str
    field: str
    unit: str


@dataclass(frozen=True)
class Description:
    """One ``DSCR`` member."""

    part: str
    code: str
    name: str
    #: For ammunition, the weapon it belongs to.
    belongs_to: str
    #: The free-text lines, in file order.
    text: tuple[str, ...]
    group: int
    level: int
    size: str
    kind: str
    #: The sub-kind chain, ``:``-joined where there is more than one.
    sub: str
    mark: str
    #: Whatever follows the mark -- one ``A<n>`` token throughout.  Unread.
    tail: tuple[str, ...]
    upgrade: int
    research_energy: float
    research_ore: float
    build_energy: float
    build_ore: float
    stats: tuple[Stat, ...]

    @property
    def research_cost(self) -> tuple[float, float]:
        """What researching it costs: (energy, ore)."""
        return self.research_energy, self.research_ore

    @property
    def build_cost(self) -> tuple[float, float]:
        """What building it costs: (energy, ore)."""
        return self.build_energy, self.build_ore

    @property
    def researched(self) -> bool:
        """True when it costs something to research."""
        return bool(self.research_energy or self.research_ore)

    @property
    def group_word(self) -> str:
        return GROUPS.get(self.group, str(self.group))

    @property
    def banded(self) -> bool:
        """True when the tech level sits in the band its upgrade level implies."""
        if not self.level:
            return True
        low, high = BANDS.get(self.upgrade, (0, 0))
        return low <= self.level <= high

    @property
    def size_word(self) -> str:
        return SIZES.get(self.size, self.size)

    @property
    def kind_word(self) -> str:
        return KINDS.get(self.kind, self.kind)


def parse_entry(text: str, part: str = "") -> Description:
    """Read one ``DSCR`` member's text."""
    group = level = 0
    size = kind = sub = mark = ""
    tail: tuple[str, ...] = ()
    code = name = belongs = ""
    free: list[str] = []
    numbers: dict[str, float] = {}
    stats: list[Stat] = []
    for line in text.replace("\r\n", "\n").split("\n"):
        line = line.rstrip()
        if not line:
            continue
        found = _GROUP.match(line)
        if found:
            group, level = int(found.group(1)), int(found.group(2))
            continue
        found = _CLASS.match(line)
        if found:
            size, kind = found.group(1), found.group(2)
            rest = found.group(3).split(":")
            marks = [i for i, x in enumerate(rest) if _MARK.match(x)]
            at = marks[0] if marks else len(rest)
            sub = ":".join(rest[:at])
            mark = rest[at] if marks else ""
            tail = tuple(rest[at + 1:]) if marks else ()
            continue
        found = _STAT.match(line)
        if found:
            stats.append(Stat(found.group(1).strip(), found.group(2), found.group(3)))
            continue
        found = _SLOT.match(line)
        if found:
            slot, body = found.group(1), found.group(2)
            key, sep, value = body.partition("=")
            if sep and any(slot == s and key.strip() == k for s, k in NUMBERS):
                try:
                    numbers[key.strip()] = float(value.strip())
                except ValueError:
                    pass
            elif slot == "1":
                code = body.strip()
            elif slot == "2":
                name = body.strip()
            elif slot == "5":
                belongs = body.strip()
            else:
                free.append(body.strip())
    return Description(
        part=part,
        code=code,
        name=name,
        belongs_to=belongs,
        text=tuple(free),
        group=group,
        level=level,
        size=size,
        kind=kind,
        sub=sub,
        mark=mark,
        tail=tail,
        upgrade=int(numbers.get("UpgradeLevel", 0)),
        research_energy=numbers.get("ResearchEnergyCost", 0.0),
        research_ore=numbers.get("ResearchOreCost", 0.0),
        build_energy=numbers.get("BuildEnergyCost", 0.0),
        build_ore=numbers.get("BuildOreCost", 0.0),
        stats=tuple(stats),
    )


def parse(data: bytes, source: Path | None = None) -> dict[str, Description]:
    """Read ``objects.dlb``: part id -> description, in file order."""
    where = source.name if source is not None else "<bytes>"
    archive = NResArchive(data)
    out: dict[str, Description] = {}
    for entry in archive.entries:
        if entry.tag != TAG:
            raise DescriptionFormatError(f"{where}: member tagged {entry.tag!r}, not {TAG!r}")
        out[entry.name] = parse_entry(archive.read(entry).decode("latin-1"), entry.name)
    if not out:
        raise DescriptionFormatError(f"{where}: no {TAG} members")
    return out


def read(path: Path) -> dict[str, Description]:
    """Read the library at ``path``."""
    return parse(path.read_bytes(), path)


def library(game: Path) -> dict[str, Description]:
    """Read the installation's ``objects.dlb``."""
    return read(game / LIBRARY)
