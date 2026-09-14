"""The behaviour scripts, ``MISSIONS/SCRIPTS/*.scr``.

A mission's AI is a set of **named handlers**, and each handler is a flat list
of **nodes**.  The file is a straightforward serialisation of that and nothing
more: a three-word header, then one record per handler, each carrying its own
node list.  All 58 shipped scripts read end to end with nothing left over.

```
int32   73, the length of the function table
int32   handler count
handler x count:
    int32   name length
    char    name[length]
    uint8   always 0
    int32   index, 0 upward in file order
    int32   node count
    node x count:
        int32   head[0]     function id, or -1
        int32   head[1]     destination variable, or -1
        int32   head[2]     source variable, or a number, or -1
        int32   head[3]     kind, -1..6
        int32   opcode      an if's relation 0..5, else 6
        int32   operand count
        int32   operands[count]
        int32   trailer     formula index into the .fml, or -1
```

Nine handlers are in **every** script -- `Init`, `Problems0`, `Mission`, four
`Fort_`/`Mech_` task events and `Hero_Teleported` -- so they are the engine's
own event set rather than anything a mission invents.  The rest are the
mission's AI problems, and those come in pairs: **every** `PBM_*_Start` in a
file has a matching `PBM_*_Continue`, across all 58 files without exception.

A node's operands are **variable indices**, and the variables are declared in
``MISSIONS/SCRIPTS/varset.var`` -- one shared, commented, plain-text symbol
table of 231 entries that every script draws on.  All 9239 operands in the
corpus index it and none falls outside.  ``head[1]`` is the node's
**destination**: it too is always a valid index, and it never names any of the
first 23 declarations, which are the literal constants ``f0``..``f9`` and
``d0``..``d9`` plus the three the engine writes itself.  Operands read those
freely.  A node therefore reads its sources and writes its result, and which
slot is which is settled.

``head[3]`` is the node's **kind**, which ``ai.dll``'s executor switches on
(``0x10012020``): -1 a statement, 0 an ``if``, 1 its ``end``, 2 a label, 3 a
jump to a label, 4 a jump to another handler, 5 ``return``, 6 a constant.  A
statement with ``head[0]`` set is a **call** of function ``head[0]`` -- an index
straight into a 73-slot handler table, the operands its arguments and the
result written to ``head[1]``; without it the statement copies the variable
``head[2]`` or, failing that, evaluates **formula** ``trailer`` of the script's
own ``.fml`` file.  An ``if`` compares its two operands under the relation its
fifth word names (``RELATIONS``); that word is 6 on every other node.  The
kinds' arities are fixed: ``if`` takes two operands, the two jumps one, the
rest none.

A function's signature is fixed: all 57 the scripts use take a single number
of arguments but for five with optional trailing ones, and the handler behind
each reads exactly that many (``ARGUMENTS``, out of the binary) on 55 of 57.
See ``docs/15-behaviour.md``.

Everything above is re-derived by ``uv run openparkan verify``.
"""

from __future__ import annotations

import re
import struct
from dataclasses import dataclass
from pathlib import Path

#: The first word of every script.  It is the length of the function table
#: the script was compiled against: ``ai.dll``'s loader allocates that many
#: slots, its constructor fills 73 and warns "Scripts are not up to date" when
#: the word says otherwise (``0x100014f9``).
MAGIC = 73

#: How many handlers the interpreter's function table holds; a call's
#: ``head[0]`` indexes it directly.
FUNCTION_TABLE = 73

#: The byte between a handler's name and its index; 0 on all 677 records.
NAME_PAD = 0

#: The fifth word.  On an ``IF`` node it is the relation, 0 to 5, and the node
#: takes exactly two operands; on every other node it is 6.
BINARY = range(0, 6)
VARIADIC = 6
OPCODES = range(0, 7)

#: The relation an ``IF`` applies, by its fifth word -- read from the executor
#: (``ai.dll:0x1001203f``).  Both operands are compared as unsigned ``DWORD``
#: when the first is a ``DWORD``, and as floats otherwise.
RELATIONS = ("<", "==", ">", "<=", ">=", "!=")

#: ``head[3]``: what a node is.  ``ai.dll:0x10012038`` jumps on ``kind + 1``
#: through an eight-entry table.
STATEMENT = -1      # a call, or a copy from a variable or a formula
IF = 0              # compare two operands; opens a block
END = 1             # closes the innermost block
LABEL = 2           # a jump target, which does nothing
GOTO = 3            # continue at the label whose node index is the operand
SWITCH = 4          # continue in the handler whose index is the operand
RETURN = 5          # stop running the handler
CONST = 6           # write head[2], as a plain number, to the destination

#: How many operands each handler of the function table reads, by function
#: id -- out of ``ai.dll`` by ``analysis/scrtable.py``, which fetches each
#: operand the same way.  The scripts pass exactly this many on 55 of the 57
#: functions they call; function 14 has an optional fourth the corpus never
#: passes, and function 0 ignores the one it is given.
ARGUMENTS = (
    0, 1, 7, 0, 1, 2, 2, 0, 1, 0, 0, 1, 0, 2, 4, 11, 2, 2, 2, 3,
    4, 2, 1, 3, 1, 5, 2, 3, 11, 1, 2, 2, 2, 1, 1, 2, 2, 1, 2, 1,
    3, 1, 1, 0, 5, 0, 0, 2, 1, 0, 1, 2, 1, 1, 1, 1, 0, 2, 1, 1,
    1, 1, 1, 1, 2, 0, 2, 1, 1, 1, 1, 3, 1,
)

#: The handlers that never write the interpreter's result slot.  A call to one
#: of them never names a destination in the shipped scripts.
VOID_FUNCTIONS = (4, 5, 6, 8, 9, 30, 43, 51, 57, 62)

#: The widest operand list in the shipped scripts.
MAX_OPERANDS = 11

#: How many distinct functions a call selects from, and their range.
FUNCTIONS = 57
FUNCTION_IDS = range(0, 73)

#: ``head[3]`` on an assignment: these two write a destination, the rest
#: (1 to 5) are the forms that write nothing.
ASSIGN_TAGS = (STATEMENT, CONST)

#: A node kind fixes its arity, the way a function id does.  The comparison
#: takes two operands, the two jumps one, the rest none.
TAG_ARITY = {STATEMENT: 0, IF: 2, END: 0, LABEL: 0, GOTO: 1, SWITCH: 1, RETURN: 0,
             CONST: 0}

#: The kind under which ``head[2]`` is a variable index, and the one under
#: which it is a plain number.
REFERENCE_TAG = STATEMENT
LITERAL_TAG = CONST

#: The kind that closes a block a comparison opened.  Across the corpus, 675
#: of 677 handlers hold exactly as many of these as comparisons and bracket
#: cleanly, nesting up to five deep; the engine clamps a spare one at depth 0.
CLOSE_TAG = END

#: The three kinds that leave the code in front of them: a jump to a label, a
#: jump to a handler, and ``return``.  Taken inside a block, each also drops
#: every open block.  319 of their 320 nodes are the last thing in a block.
EXIT_TAGS = (GOTO, SWITCH, RETURN)

#: The label a ``GOTO`` lands on.  It does nothing when run.
MARKER_TAG = LABEL

#: The bit set on 55 of the literals, and on every building's logical id: the
#: ``varset.var`` constant ``CLASS_BUILDING``.  A mission gives it to every
#: object it places that is not a unit.
CLASS_BUILDING = 0x8000_0000
LITERAL_FLAG = CLASS_BUILDING

#: The owner word of a destroyed object.  Every comparison the scripts make
#: against a variable set to 65534 tests it against function 52's answer --
#: the owner clan of a logical id -- and fails an objective when they match.
DESTROYED = 0xFFFE

#: A script's formulas: ``<script>.fml`` beside it, one ``FUNCTION( , expr, )``
#: line per statement that evaluates one.
FORMULAS = ".fml"
FORMULA_HEADER = "//FormulaSet export file"

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

#: The shared symbol table every script draws on, in ``MISSIONS/SCRIPTS``.
VARSET = "varset.var"

#: The two forms ``varset.var`` documents in its own header.  Only ``VAR`` is
#: ever used: the shipped file has no ``STRING`` declaration at all.
DECLARATIONS = ("VAR", "STRING")

#: The types those declarations carry, and how many of each.  ``ai.dll`` holds
#: six, by a type tag it names in a table of its own (``0x10037688``): 0
#: ``void``, 1 ``int``, 2 ``BOOL``, 3 ``float``, 4 ``char*``, 5 ``DWORD``.
TYPES = {"DWORD": 200, "float": 31}
TYPE_TAGS = ("void", "int", "BOOL", "float", "char*", "DWORD")

#: Declarations 0 to 22 are the read-only pool -- the float and integer
#: literals 0..9, and the three values the engine writes itself.  A node's
#: operands read them; a node's destination is never one of them.
READ_ONLY = 23

#: The high bit seen set on some ``head[2]`` values, which otherwise stay
#: small.  Whatever it tags, it is not an index in the operands' space.
TAGGED = -(2**31)


class ScriptFormatError(ValueError):
    pass


@dataclass(frozen=True)
class Node:
    """One node of a handler.

    ``operands`` and ``destination`` are indices into ``varset.var``, except
    on the two jumps, whose one operand is a node or handler index.
    ``head[0]`` is a function id, ``head[2]`` a variable or a number by kind,
    ``head[3]`` the kind, ``opcode`` an ``IF``'s relation and ``trailer`` a
    formula index.
    """

    head: tuple[int, int, int, int]
    opcode: int
    operands: tuple[int, ...]
    trailer: int

    @property
    def binary(self) -> bool:
        """True for a comparison, which always carries two operands."""
        return self.opcode in BINARY

    @property
    def kind(self) -> int:
        """``head[3]``: ``STATEMENT``, ``IF``, ``END``, ``LABEL``, ... ``CONST``."""
        return self.head[3]

    @property
    def relation(self) -> str:
        """An ``IF``'s comparison as its operator, or ``''`` on any other node."""
        if not self.calls and self.kind == IF and self.opcode in BINARY:
            return RELATIONS[self.opcode]
        return ""

    @property
    def target(self) -> int:
        """Where a jump goes: the label's node index for ``GOTO``, the handler's
        index for ``SWITCH``, or ``NULL``."""
        if not self.calls and self.kind in (GOTO, SWITCH) and self.operands:
            return self.operands[0]
        return NULL

    @property
    def formula(self) -> int:
        """The formula a statement evaluates, as an index into its ``.fml``, or
        ``NULL``."""
        return self.trailer if not self.calls else NULL

    @property
    def function(self) -> int:
        """The function this node calls, or ``NULL`` when it is not a call."""
        return self.head[0]

    @property
    def calls(self) -> bool:
        """True when the node is a call: a function, and operands as arguments."""
        return self.head[0] != NULL

    @property
    def source(self) -> int:
        """The formula an assignment evaluates, or ``NULL``; see ``formula``.

        Only ever set on a node that writes, and never together with
        ``immediate``.  It indexes the script's ``.fml``, not ``varset.var``:
        the low indices that dominate made it look like the literal pool.
        """
        return self.trailer

    @property
    def terminates(self) -> bool:
        """True for a tag that ends the block it sits in."""
        return not self.calls and not self.assigns and self.tag in EXIT_TAGS

    @property
    def opens(self) -> bool:
        """True for a comparison, which opens a block."""
        return not self.calls and self.opcode in BINARY

    @property
    def closes(self) -> bool:
        """True for the bare tag that closes one.

        Bare is checked against the raw fields rather than ``reference`` and
        ``literal``, which are gated on their own tags and so would report
        nothing here whatever ``head[2]`` held.
        """
        return (not self.calls and self.tag == CLOSE_TAG
                and self.head[2] == NULL and self.trailer == NULL)

    @property
    def tag(self) -> int:
        """What a node that is not a call does.  Its arity is ``TAG_ARITY``."""
        return self.head[3]

    @property
    def immediate(self) -> int:
        """The value an assignment writes through ``head[2]``, or ``NULL``.

        The tag says how to read it: under ``REFERENCE_TAG`` it is a variable
        index and under ``LITERAL_TAG`` a plain number.  Prefer ``reference``
        and ``literal``.
        """
        return self.head[2]

    @property
    def reference(self) -> int:
        """The variable ``head[2]`` names, or ``NULL`` if it is not one."""
        return self.head[2] if self.tag == REFERENCE_TAG else NULL

    @property
    def literal(self) -> int:
        """The number ``head[2]`` holds, or ``NULL`` if it is not one.

        ``CLASS_BUILDING``, set on 55 of them, is left in place: this returns
        the word as written, sign and all.
        """
        return self.head[2] if self.tag == LITERAL_TAG else NULL

    @property
    def assigns(self) -> bool:
        """True when the node writes its destination from one of the two."""
        return not self.calls and (self.source != NULL or self.immediate != NULL)

    @property
    def destination(self) -> int:
        """The variable this node writes, or ``NULL``.

        Never one of the first ``READ_ONLY`` declarations, where the operands
        draw on them freely -- which is what makes this the write and those
        the reads.
        """
        return self.head[1]


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


@dataclass(frozen=True)
class Variable:
    """One line of ``varset.var``.

    The file documents its own two forms at the top::

        //VAR( Type, Name, DefValue, Minimum, Maximum, Comment)
        //STRING( Size, Name, DefValue, Comment)
    """

    kind: str
    type: str
    name: str
    default: str

    @property
    def literal(self) -> bool:
        """True for the ``f0``..``f9`` / ``d0``..``d9`` constant pool."""
        return len(self.name) == 2 and self.name[0] in "fd" and self.name[1].isdigit()


_DECL = re.compile(
    r"^\s*(VAR|STRING)\(\s*([^,)]+?)\s*,\s*([A-Za-z_][A-Za-z0-9_]*)\s*"
    r"(?:,(.*?))?\)\s*;?\s*$"
)


def variables(game: Path) -> list[Variable]:
    """Read ``varset.var``: the symbol table a node's indices point into.

    Order is the file's, because the index *is* the position.  Comment-only
    lines and the trailing ``;`` some declarations carry are both tolerated;
    dropping either would shift every index after it.
    """
    return variables_file(game / "MISSIONS" / "SCRIPTS" / VARSET)


def variables_file(path: Path) -> list[Variable]:
    """Read the declarations of the ``varset.var`` at ``path``; see ``variables``."""
    out: list[Variable] = []
    for line in path.read_text("latin-1").replace("\r\n", "\n").split("\n"):
        body = "" if line.lstrip().startswith("//") else line.split("//")[0]
        found = _DECL.match(body)
        if found:
            kind, type_, name, default = found.groups()
            out.append(Variable(kind, type_, name, (default or "").strip()))
    if not out:
        raise ScriptFormatError(f"{path.name}: no declarations")
    return out


def name_at(table: list[Variable], index: int) -> str:
    """The name at ``index``, or ``''`` for ``NULL`` and anything out of range."""
    if 0 <= index < len(table):
        return table[index].name
    return ""


_FUNCTION = re.compile(r"^\s*FUNCTION\(\s*[^,]*,(.*),[^,]*\)\s*;?\s*$")


def parse_formulas(text: str, where: str = "<text>") -> list[str]:
    """The expressions of one ``.fml``, in file order.

    Each line is ``FUNCTION( , <expression>,  )`` -- the first and last fields
    are empty on all 1379 shipped lines -- and the index is the position,
    which is what a statement's trailer counts.
    """
    out: list[str] = []
    for line in text.replace("\r\n", "\n").split("\n"):
        if not line.strip() or line.lstrip().startswith("//"):
            continue
        found = _FUNCTION.match(line)
        if not found:
            raise ScriptFormatError(f"{where}: not a formula: {line.strip()!r}")
        out.append(found.group(1).strip())
    return out


def formulas(path: Path) -> list[str]:
    """Read the ``.fml`` beside a script; ``path`` may name either file."""
    fml = path.with_suffix(FORMULAS)
    return parse_formulas(fml.read_text("latin-1"), fml.name)


def render_node(node: Node, table: list[Variable], exprs: list[str] | None = None,
                handlers: tuple[str, ...] = ()) -> str:
    """One node as a line of pseudo-code.

    The control flow is named because it is read from the executor: ``if``
    with its relation, ``end``, ``label``, ``goto``, ``return``.  A function
    stays ``fn15``: its table index is what the file holds.  Variables are
    real names out of ``varset.var``; a formula prints as the expression its
    ``.fml`` gives when ``exprs`` is passed, and a jump to a handler by name
    when ``handlers`` is.
    """
    args = ", ".join(name_at(table, o) or str(o) for o in node.operands)
    into = name_at(table, node.destination) or ""
    lead = f"{into} = " if into else ""

    if node.calls:
        return f"{lead}fn{node.function}({args})"
    if node.relation and len(node.operands) == 2:
        a, b = (name_at(table, o) or str(o) for o in node.operands)
        return f"if {a} {node.relation} {b}"
    if node.source != NULL:
        if exprs is not None and 0 <= node.source < len(exprs):
            return f"{lead}{exprs[node.source]}"
        return f"{lead}formula {node.source}"
    if node.reference != NULL:
        return f"{lead}{name_at(table, node.reference) or node.reference}"
    if node.literal != NULL:
        value = node.literal & 0xFFFF_FFFF    # the word the file holds
        if value & CLASS_BUILDING:
            return f"{lead}CLASS_BUILDING|{value & ~CLASS_BUILDING}"
        return f"{lead}{value}"
    if node.kind == END and not args:
        return "end"
    if node.kind == LABEL and not args:
        return "label"
    if node.kind == RETURN and not args:
        return "return"
    if node.kind == GOTO and node.target != NULL:
        return f"goto {node.target}"
    if node.kind == SWITCH and node.target != NULL:
        if 0 <= node.target < len(handlers):
            return f"goto {handlers[node.target]}"
        return f"goto handler {node.target}"
    return f"{lead}tag{node.tag}({args})" if args else f"{lead}tag{node.tag}"


def render(script: Script, table: list[Variable], only: str = "",
           exprs: list[str] | None = None) -> list[str]:
    """A script as pseudo-code, one handler after another.

    Indented on the bracketing an ``if`` and ``end`` make, which holds on 675
    of the corpus's 677 handlers.  The depth is clamped at zero, as the
    engine clamps it, so the two that carry a spare ``end`` still print.
    """
    names = tuple(h.name for h in script.handlers)
    lines: list[str] = []
    for handler in script.handlers:
        if only and handler.name != only:
            continue
        lines.append(f"{handler.name}:   # {len(handler.nodes)} nodes")
        depth = 0
        for index, node in enumerate(handler.nodes):
            if node.closes:
                depth = max(0, depth - 1)
            text = render_node(node, table, exprs, names)
            lines.append(f"  {index:4}  {'  ' * depth}{text}")
            if node.opens:
                depth += 1
        lines.append("")
    return lines
