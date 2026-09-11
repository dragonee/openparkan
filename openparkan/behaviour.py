"""The behaviour scripts, ``MISSIONS/SCRIPTS/*.scr``.

A mission's AI is a set of **named handlers**, and each handler is a flat list
of **nodes**.  The file is a straightforward serialisation of that and nothing
more: a three-word header, then one record per handler, each carrying its own
node list.  All 58 shipped scripts read end to end with nothing left over.

```
int32   magic, always 73
int32   handler count
handler x count:
    int32   name length
    char    name[length]
    uint8   always 0
    int32   index, 0 upward in file order
    int32   node count
    node x count:
        int32   head[4]     four reference fields, meanings open
        int32   opcode      0..6
        int32   operand count
        int32   operands[count]
        int32   trailer
```

Nine handlers are in **every** script -- `Init`, `Problems0`, `Mission`, four
`Fort_`/`Mech_` task events and `Hero_Teleported` -- so they are the engine's
own event set rather than anything a mission invents.  The rest are the
mission's AI problems, and those come in pairs: **every** `PBM_*_Start` in a
file has a matching `PBM_*_Continue`, across all 58 files without exception.

What a node *does* is not read here, and reproducing that is the hard part of
the game rather than of the format.  Two structural facts constrain it: the
opcode's arity is fixed -- **0 to 5 take exactly two operands and 6 takes
anything from none to eleven** -- and the operands index a table that is not
in this file, since they run to 228 in scripts holding as few as 17 nodes.
See ``docs/15-behaviour.md``.

Everything above is re-derived by ``uv run openparkan verify``.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass
from pathlib import Path

#: The first word of every script.
MAGIC = 73

#: The byte between a handler's name and its index; 0 on all 677 records.
NAME_PAD = 0

#: Opcodes 0 to 5 take exactly two operands.  Opcode 6 is variadic.
BINARY = range(0, 6)
VARIADIC = 6
OPCODES = range(0, 7)

#: The widest operand list in the shipped scripts.
MAX_OPERANDS = 11

#: The engine's own event handlers: present in all 58 scripts.
EVENTS = (
    "Init",
    "Problems0",
    "Mission",
    "Fort_Task_Complete",
    "Fort_Captured",
    "Mech_GeneratorFound",
    "Mech_Mineral_Found",
    "Mech_Task_Complete",
    "Hero_Teleported",
)

#: The AI problems.  Each appears as a ``_Start``/``_Continue`` pair.
PROBLEMS = (
    "PBM_ATTACK_UNIT",
    "PBM_BASE_DEFENCE",
    "PBM_BUILDING_ATTACK",
    "PBM_BUILDING_CAPTURE",
    "PBM_BUILDING_INF_CAPTURE",
    "PBM_BUILDING_NEEDED",
    "PBM_BUILDING_PROTECT",
    "PBM_MAKE_RESEARCH",
    "PBM_MINE_NEEDED",
    "PBM_N_E_ENERGY",
    "PBM_N_OPTIMAL_TRANSPORT",
    "PBM_PLACE_PROTECT",
    "PBM_ROBOT_NEEDED",
    "PBM_UPGRADE_NEEDED",
)

#: The two halves a problem is always written in.
PHASES = ("Start", "Continue")

#: What a head field or a trailer holds where it holds nothing.
NULL = -1

#: The high bit seen set on some ``head[2]`` values, which otherwise stay
#: small.  Whatever it tags, it is not an index in the operands' space.
TAGGED = -(2**31)


class ScriptFormatError(ValueError):
    pass


@dataclass(frozen=True)
class Node:
    """One node of a handler.

    ``head`` is four fields whose meanings are open.  Their observed ranges
    differ, so they are not four of a kind: slot 0 runs -1..72, slot 1
    -1..228 (the operands' own range), slot 2 is mostly -1 with a handful of
    values carrying the top bit, and slot 3 runs -1..6.
    """

    head: tuple[int, int, int, int]
    opcode: int
    operands: tuple[int, ...]
    trailer: int

    @property
    def binary(self) -> bool:
        """True for the fixed-arity opcodes, which always carry two operands."""
        return self.opcode in BINARY


@dataclass(frozen=True)
class Handler:
    """A named entry point and the nodes behind it."""

    name: str
    index: int
    nodes: tuple[Node, ...]

    @property
    def problem(self) -> str:
        """The ``PBM_`` problem this handler is half of, or ``''``."""
        for phase in PHASES:
            if self.name.endswith(f"_{phase}"):
                return self.name[: -len(phase) - 1]
        return ""

    @property
    def phase(self) -> str:
        """``Start``, ``Continue``, or ``''`` for an event handler."""
        for phase in PHASES:
            if self.name.endswith(f"_{phase}"):
                return phase
        return ""


@dataclass(frozen=True)
class Script:
    """One ``.scr`` file."""

    source: Path
    magic: int
    handlers: tuple[Handler, ...]

    def handler(self, name: str) -> Handler | None:
        """The handler of that name, or None."""
        for h in self.handlers:
            if h.name == name:
                return h
        return None

    @property
    def problems(self) -> tuple[str, ...]:
        """The ``PBM_`` problems this script carries, in file order, once each."""
        seen: list[str] = []
        for h in self.handlers:
            if h.problem and h.problem not in seen:
                seen.append(h.problem)
        return tuple(seen)

    @property
    def events(self) -> tuple[str, ...]:
        """The handlers that are not half of a problem."""
        return tuple(h.name for h in self.handlers if not h.problem)

    @property
    def nodes(self) -> int:
        return sum(len(h.nodes) for h in self.handlers)


class _Reader:
    def __init__(self, data: bytes, where: str) -> None:
        self.data = data
        self.where = where
        self.at = 0

    def i32(self, n: int = 1) -> tuple[int, ...]:
        end = self.at + 4 * n
        if end > len(self.data):
            raise ScriptFormatError(
                f"{self.where}: ran off the end at {self.at} wanting {4 * n} bytes"
            )
        out = struct.unpack_from(f"<{n}i", self.data, self.at)
        self.at = end
        return out

    def u8(self) -> int:
        if self.at >= len(self.data):
            raise ScriptFormatError(f"{self.where}: ran off the end at {self.at}")
        out = self.data[self.at]
        self.at += 1
        return out

    def name(self) -> str:
        (length,) = self.i32()
        if not 0 < length <= 64 or self.at + length > len(self.data):
            raise ScriptFormatError(f"{self.where}: bad name length {length} at {self.at - 4}")
        out = self.data[self.at : self.at + length]
        self.at += length
        return out.decode("latin-1")


def parse(data: bytes, source: Path | None = None) -> Script:
    """Read one script.  Raises unless it is consumed exactly."""
    where = source.name if source is not None else "<bytes>"
    r = _Reader(data, where)
    magic, count = r.i32(2)
    if magic != MAGIC:
        raise ScriptFormatError(f"{where}: magic {magic}, not {MAGIC}")
    if count < 0:
        raise ScriptFormatError(f"{where}: handler count {count}")

    handlers: list[Handler] = []
    for expected in range(count):
        name = r.name()
        pad = r.u8()
        if pad != NAME_PAD:
            raise ScriptFormatError(f"{where}: {name} padded with {pad}, not {NAME_PAD}")
        index, node_count = r.i32(2)
        if index != expected:
            raise ScriptFormatError(f"{where}: {name} indexed {index}, not {expected}")
        if node_count < 0:
            raise ScriptFormatError(f"{where}: {name} declares {node_count} nodes")
        nodes: list[Node] = []
        for _ in range(node_count):
            *head, opcode, operand_count = r.i32(6)
            if opcode not in OPCODES:
                raise ScriptFormatError(f"{where}: {name} node opcode {opcode}")
            if operand_count < 0:
                raise ScriptFormatError(f"{where}: {name} node takes {operand_count} operands")
            operands = r.i32(operand_count) if operand_count else ()
            (trailer,) = r.i32()
            nodes.append(
                Node(
                    head=(head[0], head[1], head[2], head[3]),
                    opcode=opcode,
                    operands=operands,
                    trailer=trailer,
                )
            )
        handlers.append(Handler(name=name, index=index, nodes=tuple(nodes)))

    if r.at != len(data):
        raise ScriptFormatError(
            f"{where}: {len(data) - r.at} bytes left over after {count} handlers"
        )
    return Script(source=source or Path(where), magic=magic, handlers=tuple(handlers))


def read(path: Path) -> Script:
    """Read the script at ``path``."""
    return parse(path.read_bytes(), path)


def scripts(game: Path) -> list[Path]:
    """Every ``.scr`` the installation ships, sorted."""
    return sorted((game / "MISSIONS" / "SCRIPTS").glob("*.scr"))
