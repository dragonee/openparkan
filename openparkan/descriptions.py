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

#: The size letter of the classification line, as in a part id.  ``A``, ``N``
#: and ``E`` are letters no part id uses; the research tree files ``A``, ``H``
#: and ``N`` as size 4 and ``E`` as 5.  ``N`` is an animal -- the five ``ANM``
#: parts, the same five the research tree marks with role 7 -- and ``A`` is a
#: fortification's fittings: 27 parts, 21 of them fitted only into assemblies
#: with an ``fr_*`` building at the root, and every line in the file that says
#: "fortification" belongs to one.  The words themselves are not recoverable;
#: see ``docs/19-descriptions.md``.
SIZES = {"B": "large", "M": "medium", "L": "small", "T": "tiny", "H": "huge"}

#: The ``A<n>`` token that closes the classification line, by size letter.  It
#: follows the part's own letter on 358 of 395, and on all 275 parts that are
#: neither ``WPN`` nor ``AMM``.  On a weapon it is instead **the size of the
#: round it fires** -- which covers 25 of the 37 that differ.  Eight more keep
#: their own letter, seven of them because the round they fire is ``f``-sized
#: and no grade covers that; the 12 clip-less built-in guns are unexplained.
#: See ``round_grade`` and ``docs/19-descriptions.md``.
GRADES = {"T": "A0", "L": "A1", "N": "A1", "M": "A2", "B": "A3", "E": "A4",
          "A": "A5", "H": "A5"}

#: The armament kinds whose grade the round they fire decides.
ARMAMENT_KINDS = ("WPN", "AMM")

#: Nothing in the game reads this file's text.  ``iron3d.dll`` opens it once
#: (``0x100487a4``) and only asks whether a member exists for a part id; the
#: tree's ``TRF0`` carries the kind, sub-kinds and size as numbers, and has no
#: field for the group, the level, the mark or the grade.

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
#: A stat row is ``@G@<label> @B,<field>,G,<unit>,<width>,<decimals>@``.  The
#: third column is ``G`` on every row that carries a unit and empty on the 99
#: hanger rows, which print a quoted count and nothing else (*measured*).
_STAT = re.compile(r"^@G@(.*?)\s*@B,([^,]*),G?,([^,]*),")
#: An ``objects.rlb`` member's size letter -- ``<family>_<size>_<index>``.
_MEMBER = re.compile(r"^[A-Za-z]{2,4}_([A-Za-z])_")


def round_grade(member: str) -> str:
    """The grade the round ``member``'s own size letter implies, or ''.

    A weapon's or ammunition pack's ``A<n>`` is the size of the **round it
    fires**, not its own: ``e_gun_bl_14`` is a large launcher loaded with
    ``bm_l_01``, a small missile, and carries ``A1``, as does ``i_c14_b_df``,
    the large pack that feeds it.  A round whose size letter is ``f`` grades
    nothing -- ``f`` is outside ``GRADES`` -- which is why the fortification
    guns keep their own ``A3``.  See ``docs/19-descriptions.md``.
    """
    found = _MEMBER.match(member)
    return GRADES.get(found.group(1).upper(), "") if found else ""


class DescriptionFormatError(ValueError):
    pass


@dataclass(frozen=True)
class Stat:
    """One row of the part's stat panel: a label, the field behind it, a unit.

    A row whose field is quoted prints that text instead of reading a field: a
    turret's ``Cannon hanger "2"``, a laser's printed damage.
    """

    label: str
    field: str
    unit: str

    @property
    def literal(self) -> str:
        """The quoted text a row prints, or '' where the row names a field."""
        return self.field[1:-1] if self.field.startswith('"') and self.field.endswith('"') else ""


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
    #: Whatever follows the mark -- one ``A<n>`` token throughout, a size
    #: grade; see ``GRADES``.
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
    def grade(self) -> str:
        """The ``A<n>`` token, or '' when the line has none."""
        return self.tail[0] if self.tail else ""

    @property
    def graded(self) -> bool:
        """True when the grade is the one the size letter implies."""
        return bool(self.grade) and GRADES.get(self.size) == self.grade

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
