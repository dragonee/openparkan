"""The input layer, which the game ships as commented plain text.

Seventeen files in the installation root are not archives at all.  They are
tables the developers wrote and annotated in English, and together they are the
whole path from a key to a command:

```
ScanCode.dsc   SCAN_A                    a key, and what to call it on screen
Command.dsc    CMD_OBJ_MOVE_LEFT         an action, and what it does
*.man          CMD_OBJ_MOVE_LEFT  SCAN_NULL SCAN_A      which key runs it
*.tbl          SCAN_NULL SCAN_A 1 CICLS_UNKNOWN MCMD_LEFT 1.0 ...  what it sends
```

The `.tbl` row is the interesting one: it names the **target class** the
command goes to (`CICLS_TURRET`, `CICLS_CAMERA`, `CICLS_MULTIGUN`) and the
**movement command** it sends (`MCMD_LEFT`, `MCMD_ANGLE_X`), with a magnitude
that is 1.0 while a key is down and 0.0 when it comes up.  That is the input
half of the movement controller in `control.py`, and it is the engine's own
vocabulary rather than anything recovered from a disassembly.

Nothing here is decoded, guessed or reverse-engineered.  It is read.

Everything below is re-derived by ``uv run openparkan verify``.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

#: What the game calls its three input tables.  ``hero.tbl`` drives the pilot
#: on foot; ``m1.tbl`` and ``m2.tbl`` are two machine schemes.
TABLES = ("hero.tbl", "m1.tbl", "m2.tbl")

#: The two descriptor files: every key and every action, with a label for each.
SCANCODES = "ScanCode.dsc"
COMMANDS = "Command.dsc"

#: The behaviour system's building schemes.
BUILD_SCHEMES = "BuildDat.lst"
#: The file's own header says "There must be 11 schemes".  It ships **12**.
#: Whether the engine reads the twelfth is not established here.
BUILD_SCHEME_DECLARED = 11

#: A ``.tbl`` row has eleven fields before the trailing text.
TABLE_FIELDS = 11

#: The "no modifier" key, and the two devices a row can come from.
NO_MODIFIER = "SCAN_NULL"
DEVICES = ("KEY", "MOUSE")


class ControlsFormatError(ValueError):
    pass


def _lines(path: Path) -> list[str]:
    """Text lines with the CRLF and the comment-only lines taken off."""
    text = path.read_text("latin-1").replace("\r\n", "\n")
    return [line.rstrip() for line in text.split("\n")]


def _pairs(path: Path, prefix: str) -> dict[str, str]:
    """A ``.dsc``: one identifier per line, then its label, tabs and all."""
    out: dict[str, str] = {}
    for line in _lines(path):
        head, _, rest = line.strip().partition(" ")
        if head.startswith(prefix):
            out[head] = rest.strip()
    return out


def scancodes(game: Path) -> dict[str, str]:
    """Every key the engine knows, and what it prints for it."""
    return _pairs(game / SCANCODES, "SCAN_")


def commands(game: Path) -> dict[str, str]:
    """Every action the engine knows, and the sentence describing it."""
    return _pairs(game / COMMANDS, "CMD_")


@dataclass(frozen=True)
class Binding:
    """One line of a ``.man``: an action, and the key chord that runs it."""

    command: str
    modifier: str
    key: str

    @property
    def chord(self) -> str:
        return self.key if self.modifier == NO_MODIFIER else f"{self.modifier}+{self.key}"


def bindings(path: Path) -> list[Binding]:
    """Read one ``.man``.  Raises unless every line is a three-field binding."""
    out: list[Binding] = []
    for number, line in enumerate(_lines(path), 1):
        if not line.strip():
            continue
        fields = line.split()
        if len(fields) != 3:
            raise ControlsFormatError(f"{path.name}:{number}: {len(fields)} fields, not 3")
        out.append(Binding(*fields))
    return out


@dataclass(frozen=True)
class Action:
    """One row of a ``.tbl``: a key event, and the command it sends where.

    ``value`` is the magnitude the command carries -- 1.0 as a key goes down
    and 0.0 as it comes up for a movement, negative for the opposite
    direction, and a fraction for a rate.  ``ramp`` and ``ramp_time`` are set
    on six rows only, the speed-step keys among them.
    """

    #: ``KEY`` or ``MOUSE``.
    device: str
    #: The chord: a modifier (usually ``SCAN_NULL``) and the key itself.
    modifier: str
    key: str
    #: True on the press row, False on the release row.
    pressed: bool
    #: ``CICLS_*`` -- which class of component the command is aimed at.
    target: str
    #: ``MCMD_*`` -- the movement command itself.
    command: str
    value: float
    index: int
    #: ``0``, a ``MAN_*`` wrap flag, or a ``CIS_*`` state.
    state: str
    ramp: float
    ramp_time: int
    #: The text after ``//``.  On a press row it is usually the ``CMD_`` name
    #: without its prefix; on a release row it is prose.
    note: str

    @property
    def action(self) -> str:
        """The ``Command.dsc`` identifier this row names, if it names one."""
        return f"CMD_{self.note}" if self.note else ""


def table(path: Path) -> list[Action]:
    """Read one ``.tbl``.  Raises unless every row has its eleven fields."""
    out: list[Action] = []
    for number, line in enumerate(_lines(path), 1):
        body, _, note = line.partition("//")
        fields = body.split()
        if not fields:
            continue
        if len(fields) != TABLE_FIELDS:
            raise ControlsFormatError(
                f"{path.name}:{number}: {len(fields)} fields, not {TABLE_FIELDS}"
            )
        (device, modifier, key, pressed, target, command,
         value, index, state, ramp, ramp_time) = fields
        out.append(
            Action(
                device=device,
                modifier=modifier,
                key=key,
                pressed=pressed == "1",
                target=target,
                command=command,
                value=float(value),
                index=int(index),
                state=state,
                ramp=float(ramp),
                ramp_time=int(ramp_time),
                note=note.strip(),
            )
        )
    return out


@dataclass(frozen=True)
class BuildScheme:
    """One scheme the behaviour system builds from: a role, and the
    assemblies that fill it at each size."""

    name: str
    members: tuple[str, ...]


def build_schemes(game: Path) -> list[BuildScheme]:
    """Read ``BuildDat.lst``.  Raises unless each scheme has the count it declares."""
    path = game / BUILD_SCHEMES
    out: list[BuildScheme] = []
    pending: list[str] = []
    name = ""
    want = 0
    for number, line in enumerate(_lines(path), 1):
        text = line.split("//")[0].strip()
        if not text:
            continue
        if text.startswith('"'):
            pending.append(text.strip('"'))
            continue
        if name and len(pending) != want:
            raise ControlsFormatError(
                f"{path.name}:{number}: {name} declared {want}, got {len(pending)}"
            )
        if name:
            out.append(BuildScheme(name, tuple(pending)))
        head, _, count = text.rpartition(" ")
        name, pending, want = head.strip(), [], int(count)
    if name:
        if len(pending) != want:
            raise ControlsFormatError(f"{path.name}: {name} declared {want}, got {len(pending)}")
        out.append(BuildScheme(name, tuple(pending)))
    return out
