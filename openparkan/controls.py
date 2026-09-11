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

#: The controller's own message dispatch is a 16-way jump table on the command
#: number, so these are the commands it handles itself.  Walking and locking
#: are outside it and go somewhere else.
DISPATCHED = range(1, 17)

#: What a resolver returns for a name it does not know.
UNRESOLVED = -1
UNKNOWN_CLASS = 0

#: The "no modifier" key, and the two devices a row can come from.
NO_MODIFIER = "SCAN_NULL"
DEVICES = ("KEY", "MOUSE")


# --- Values recovered from World3D.dll -------------------------------------
#
# The engine parses these tables as text, so it carries a resolver for every
# family of names: a chain of string compares, each case returning a
# constant, ending in a branchless idiom that yields the constant on a match
# and the family's default otherwise (-1 for most, 0 for the classes).
# The names are the files'; the numbers are the engine's.

#: Movement command -> the number the engine dispatches on.  The
#: controller's own jump table covers 1 to 16; ``MCMD_WALK_F``,
#: ``MCMD_WALK_B`` and ``MCMD_LOCK`` fall outside it.
MCMD = {
    "MCMD_DUMMY": 0,
    "MCMD_STATE": 1,
    "MCMD_ROTATE_X": 2,
    "MCMD_ROTATE_Y": 3,
    "MCMD_ROTATE_Z": 4,
    "MCMD_ANGLE_X": 5,
    "MCMD_ANGLE_Y": 6,
    "MCMD_FORWARD": 7,
    "MCMD_BACK": 8,
    "MCMD_LEFT": 9,
    "MCMD_RIGHT": 10,
    "MCMD_UP": 11,
    "MCMD_DOWN": 12,
    "MCMD_SELECT": 13,
    "MCMD_SELECT_NEXT": 14,
    "MCMD_TABLE": 15,
    "MCMD_ANGLE_Z": 16,
    "MCMD_MISSILE": 17,
    "MCMD_FIRE_ALL": 18,
    "MCMD_WALK_F": 19,
    "MCMD_WALK_B": 20,
    "MCMD_LOCK": 21,
}

#: Component class -> its id.  6, 7, 14, 16, 17 and 18 name nothing.
#: An unrecognised class resolves to 0, which is what ``CICLS_UNKNOWN``
#: in the shipped tables amounts to.
CICLS = {
    "CICLS_TURRET": 1,
    "CICLS_MULTIGUN": 2,
    "CICLS_SIMPLE": 3,
    "CICLS_CAMERA": 4,
    "CICLS_ENGINE": 5,
    "CICLS_RADAR": 8,
    "CICLS_FIGHTSHIELD": 9,
    "CICLS_DETECTSHIELD": 10,
    "CICLS_ELEVATOR": 11,
    "CICLS_DOOR": 12,
    "CICLS_COMPUTER": 13,
    "CICLS_REPAIRSYS": 15,
    "CICLS_POWERSTOR": 19,
}

#: Component state -> its bit.  The same bit means different things to
#: different classes: 256 is ``CIS_CONTINUEFIGHT`` to a gun,
#: ``CIS_TURRETCONTROL`` to a turret and ``CIS_ANGLETRACE`` to a tracker.
CIS = {
    "CIS_SWITCHOFF": 0,
    "CIS_SWITCHON": 32,
    "CIS_SWITCH_INV": 64,
    "CIS_ANGLETRACE": 256,
    "CIS_CONTINUEFIGHT": 256,
    "CIS_TURRETCONTROL": 256,
    "CIS_MANUALCONTROL": 512,
    "CIS_MANUALTRACE": 512,
    "CIS_SINGLEFIGHT": 512,
    "CIS_GROUPFIGHT": 1024,
    "CIS_POINTTRACE": 1024,
    "CIS_INFRARED_ON": 4096,
    "CIS_INFRARED_OFF": 8192,
    "CIS_CHAMELEON_INV": 16384,
    "CIS_INFRARED_INV": 16384,
}

#: Whether an angle wraps at a whole turn.
MAN = {
    "MAN_WRAP": 0,
    "MAN_NOTWRAP": 1,
}

#: Key -> scan code.  Every one of the 174 names in ``ScanCode.dsc`` is
#: here and there are no others.  These are the real IBM PC set-1 codes:
#: ``SCAN_A`` is 30, ``SCAN_ESC`` is 1, ``SCAN_F1`` is 59.  The mouse and
#: joystick continue the numbering past the keyboard.
SCAN = {
    "SCAN_NULL": 0,
    "SCAN_ESC": 1,
    "SCAN_W_1": 2,
    "SCAN_W_2": 3,
    "SCAN_W_3": 4,
    "SCAN_W_4": 5,
    "SCAN_W_5": 6,
    "SCAN_W_6": 7,
    "SCAN_W_7": 8,
    "SCAN_W_8": 9,
    "SCAN_W_9": 10,
    "SCAN_W_0": 11,
    "SCAN_W_SUB": 12,
    "SCAN_W_PLUS": 13,
    "SCAN_BS": 14,
    "SCAN_TAB": 15,
    "SCAN_Q": 16,
    "SCAN_W": 17,
    "SCAN_E": 18,
    "SCAN_R": 19,
    "SCAN_T": 20,
    "SCAN_Y": 21,
    "SCAN_U": 22,
    "SCAN_I": 23,
    "SCAN_O": 24,
    "SCAN_P": 25,
    "SCAN_LBRACKET": 26,
    "SCAN_RBRACKET": 27,
    "SCAN_W_ENTER": 28,
    "SCAN_LCTRL": 29,
    "SCAN_A": 30,
    "SCAN_S": 31,
    "SCAN_D": 32,
    "SCAN_F": 33,
    "SCAN_G": 34,
    "SCAN_H": 35,
    "SCAN_J": 36,
    "SCAN_K": 37,
    "SCAN_L": 38,
    "SCAN_SEMIDOT": 39,
    "SCAN_QUOTE": 40,
    "SCAN_TILDA": 41,
    "SCAN_LSHIFT": 42,
    "SCAN_BSLASH": 43,
    "SCAN_Z": 44,
    "SCAN_X": 45,
    "SCAN_C": 46,
    "SCAN_V": 47,
    "SCAN_B": 48,
    "SCAN_N": 49,
    "SCAN_M": 50,
    "SCAN_COMMA": 51,
    "SCAN_DOT": 52,
    "SCAN_SLASH": 53,
    "SCAN_RSHIFT": 54,
    "SCAN_G_ASTERISK": 55,
    "SCAN_BLANK": 57,
    "SCAN_CAPS": 58,
    "SCAN_F1": 59,
    "SCAN_F2": 60,
    "SCAN_F3": 61,
    "SCAN_F4": 62,
    "SCAN_F5": 63,
    "SCAN_F6": 64,
    "SCAN_F7": 65,
    "SCAN_F8": 66,
    "SCAN_F9": 67,
    "SCAN_F10": 68,
    "SCAN_PAUSE": 69,
    "SCAN_SLOCK": 70,
    "SCAN_G_7": 71,
    "SCAN_G_8": 72,
    "SCAN_G_9": 73,
    "SCAN_G_SUB": 74,
    "SCAN_G_4": 75,
    "SCAN_G_5": 76,
    "SCAN_G_6": 77,
    "SCAN_G_PLUS": 78,
    "SCAN_G_1": 79,
    "SCAN_G_2": 80,
    "SCAN_G_3": 81,
    "SCAN_G_0": 82,
    "SCAN_W_DEL": 83,
    "SCAN_F11": 87,
    "SCAN_F12": 88,
    "SCAN_G_ENTER": 284,
    "SCAN_RCTRL": 285,
    "SCAN_G_SLASH": 309,
    "SCAN_NUMLOCK": 325,
    "SCAN_G_HOME": 327,
    "SCAN_G_UP": 328,
    "SCAN_G_PGUP": 329,
    "SCAN_G_LEFT": 331,
    "SCAN_G_RIGHT": 333,
    "SCAN_G_END": 335,
    "SCAN_G_DOWN": 336,
    "SCAN_G_PGDN": 337,
    "SCAN_G_INS": 338,
    "SCAN_G_DEL": 339,
    "SCAN_MOUSE_X": 510,
    "SCAN_MOUSE_Y": 511,
    "SCAN_LMOUSE": 512,
    "SCAN_RMOUSE": 513,
    "SCAN_MMOUSE": 514,
    "SCAN_JOY1": 516,
    "SCAN_JOY2": 517,
    "SCAN_JOY3": 518,
    "SCAN_JOY4": 519,
    "SCAN_JOY5": 520,
    "SCAN_JOY6": 521,
    "SCAN_JOY7": 522,
    "SCAN_JOY8": 523,
    "SCAN_JOY9": 524,
    "SCAN_JOY10": 525,
    "SCAN_JOY11": 526,
    "SCAN_JOY12": 527,
    "SCAN_JOY13": 528,
    "SCAN_JOY14": 529,
    "SCAN_JOY15": 530,
    "SCAN_JOY16": 531,
    "SCAN_JOY17": 532,
    "SCAN_JOY18": 533,
    "SCAN_JOY19": 534,
    "SCAN_JOY20": 535,
    "SCAN_JOY21": 536,
    "SCAN_JOY22": 537,
    "SCAN_JOY23": 538,
    "SCAN_JOY24": 539,
    "SCAN_JOY25": 540,
    "SCAN_JOY26": 541,
    "SCAN_JOY27": 542,
    "SCAN_JOY28": 543,
    "SCAN_JOY29": 544,
    "SCAN_JOY30": 545,
    "SCAN_JOY31": 546,
    "SCAN_JOY32": 547,
    "SCAN_JPOV1_0": 550,
    "SCAN_JPOV1_45": 551,
    "SCAN_JPOV1_90": 552,
    "SCAN_JPOV1_135": 553,
    "SCAN_JPOV1_180": 554,
    "SCAN_JPOV1_225": 555,
    "SCAN_JPOV1_270": 556,
    "SCAN_JPOV1_315": 557,
    "SCAN_JPOV2_0": 560,
    "SCAN_JPOV2_45": 561,
    "SCAN_JPOV2_90": 562,
    "SCAN_JPOV2_135": 563,
    "SCAN_JPOV2_180": 564,
    "SCAN_JPOV2_225": 565,
    "SCAN_JPOV2_270": 566,
    "SCAN_JPOV2_315": 567,
    "SCAN_JPOV3_0": 570,
    "SCAN_JPOV3_45": 571,
    "SCAN_JPOV3_90": 572,
    "SCAN_JPOV3_135": 573,
    "SCAN_JPOV3_180": 574,
    "SCAN_JPOV3_225": 575,
    "SCAN_JPOV3_270": 576,
    "SCAN_JPOV3_315": 577,
    "SCAN_JPOV4_0": 580,
    "SCAN_JPOV4_45": 581,
    "SCAN_JPOV4_90": 582,
    "SCAN_JPOV4_135": 583,
    "SCAN_JPOV4_180": 584,
    "SCAN_JPOV4_225": 585,
    "SCAN_JPOV4_270": 586,
    "SCAN_JPOV4_315": 587,
    "SCAN_JOY_X": 590,
    "SCAN_JOY_Y": 591,
    "SCAN_JOY_Z": 592,
    "SCAN_JOY_R_X": 593,
    "SCAN_JOY_R_Y": 594,
    "SCAN_JOY_R_Z": 595,
}


# --- Values recovered from World3D.dll and iron3d.dll ----------------------
#
# The `CMD_` names of `Command.dsc` and the `.man` binding files are resolved
# by **two** binaries, and they divide the vocabulary cleanly between them:
# `World3D.dll` answers for the commands aimed at the object you are
# controlling, `iron3d.dll` for the ones aimed at the game itself.  The two
# chains share exactly one name, `CMD_CAMERA_INFRARED`, and they agree on it.
#
# The split is not an inference from the names.  Ten of the twelve `.man`
# files draw on one binary only -- `hero.man` and the two `table_*.man` from
# World3D, `addition.man` and `ui_hq*.man` from iron3d -- and no `.tbl` row
# anywhere names an `iron3d.dll` command.  Only `ui_other.man` mixes, which is
# what a file of that name should do.

#: Commands to the controlled object, from ``World3D.dll``.  The values are
#: banded by subsystem: 1-14 the hull, 20-24 the turret, 30-35 the camera,
#: 40-49 weapon selection, 50-51 firing the selected weapon, 60-66 the rest.
CMD_OBJECT = {
    "CMD_OBJ_MOVE_LEFT": 1,
    "CMD_OBJ_MOVE_RIGHT": 2,
    "CMD_OBJ_MOVE_FORWARD": 3,
    "CMD_OBJ_MOVE_BACKWARD": 4,
    "CMD_OBJ_MOVE_DOWN": 5,
    "CMD_OBJ_MOVE_UP": 6,
    "CMD_OBJ_TURN_LEFT": 7,
    "CMD_OBJ_TURN_RIGHT": 8,
    "CMD_OBJ_TURN_UP": 9,
    "CMD_OBJ_TURN_DOWN": 10,
    "CMD_OBJ_SPEED_MAX": 11,
    "CMD_OBJ_SPEED_MORE": 12,
    "CMD_OBJ_SPEED_LESS": 13,
    "CMD_OBJ_STOP": 14,
    "CMD_TURRET_LEFT": 20,
    "CMD_TURRET_RIGHT": 21,
    "CMD_TURRET_UP": 22,
    "CMD_TURRET_DOWN": 23,
    "CMD_TURRET_CENTER": 24,
    "CMD_CAMERA_LEFT": 30,
    "CMD_CAMERA_RIGHT": 31,
    "CMD_CAMERA_UP": 32,
    "CMD_CAMERA_DOWN": 33,
    "CMD_CAMERA_CENTER": 34,
    "CMD_CAMERA_INFRARED": 35,
    "CMD_SELECT_ALL_WEAPON": 40,
    "CMD_SELECT_WEAPON_1": 41,
    "CMD_SELECT_WEAPON_2": 42,
    "CMD_SELECT_WEAPON_3": 43,
    "CMD_SELECT_WEAPON_4": 44,
    "CMD_SELECT_WEAPON_5": 45,
    "CMD_SELECT_WEAPON_6": 46,
    "CMD_SELECT_WEAPON_7": 47,
    "CMD_SELECT_WEAPON_8": 48,
    "CMD_SELECT_WEAPON_9": 49,
    "CMD_FIRE_SELECTED_CONT": 50,
    "CMD_FIRE_SELECTED": 51,
    "CMD_CAMOUFLAGE_WEAR": 60,
    "CMD_REPAIRSYS_ON": 61,
    "CMD_CHANGE_TABLE": 63,
    "CMD_SPOTLIGHT": 64,
    "CMD_FIRE_MISSILE": 65,
    "CMD_FIRE_ALL": 66,
}

#: Commands to the game, from ``iron3d.dll``: the commander's camera, the
#: menus, the terminal, saving and loading.  A flat run of 723..754 with 743
#: and 745 unused, plus the one camera command it shares with the object set.
CMD_GAME = {
    "CMD_CAMERA_INFRARED": 35,
    "CMD_JAMES_HQ_MOVE_LEFT": 723,
    "CMD_JAMES_HQ_MOVE_RIGHT": 724,
    "CMD_JAMES_HQ_MOVE_UP": 725,
    "CMD_JAMES_HQ_MOVE_DOWN": 726,
    "CMD_JAMES_HQ_MOVE_FORWARD": 727,
    "CMD_JAMES_HQ_MOVE_BACKWARD": 728,
    "CMD_PAGER": 729,
    "CMD_ENTER_STATE": 730,
    "CMD_JAMES_MISSION_OBJ": 731,
    "CMD_JAMES_SELECT_TARGET": 732,
    "CMD_JAMES_SELECT_ENEMY": 733,
    "CMD_JAMES_SELECT_FRIEND": 734,
    "CMD_ROLLBACK_STATE": 735,
    "CMD_JAMES_OUTER_CAMERA": 736,
    "CMD_JAMES_COCKPIT_OFF": 737,
    "CMD_JAMES_ZOOM_MODE": 738,
    "CMD_JAMES_SATELLITE_MAP": 739,
    "CMD_JAMES_WINGMAN_MENU": 740,
    "CMD_JAMES_BASE_ROTLEFT": 741,
    "CMD_JAMES_BASE_ROTRIGHT": 742,
    "CMD_JAMES_AUTO_DRIVER": 744,
    "CMD_TERMINAL": 746,
    "CMD_TERMINAL_NEW_MESSAGE": 747,
    "CMD_GAME_MENU": 748,
    "CMD_HELP": 749,
    "CMD_JAMES_AIM_TARGET": 750,
    "CMD_INC_MAP_ALPHA": 751,
    "CMD_DEC_MAP_ALPHA": 752,
    "CMD_QUICK_SAVE": 753,
    "CMD_QUICK_LOAD": 754,
}

#: Every command the engine knows, from whichever binary resolves it.  The
#: union is **73** names against the **72** in ``Command.dsc``: the extra one
#: is ``CMD_FIRE_SELECTED`` (51), which the engine resolves but no shipped
#: file names.  Nothing in ``Command.dsc`` is missing from here.
CMD = {**CMD_OBJECT, **CMD_GAME}

#: The one name both binaries resolve, and the value they agree on.
CMD_SHARED = "CMD_CAMERA_INFRARED"

#: Resolved by the engine but named by no shipped file.  Its neighbour
#: ``CMD_FIRE_SELECTED_CONT`` (50) is the one ``Command.dsc`` exposes, so
#: this looks like the single-shot half of a pair that was never bound.
CMD_UNNAMED = "CMD_FIRE_SELECTED"

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
        fields = line.strip().split(None, 1)
        if fields and fields[0].startswith(prefix):
            out[fields[0]] = fields[1].strip() if len(fields) > 1 else ""
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

    @property
    def codes(self) -> tuple[int, int]:
        """The chord as the engine numbers it: (modifier, key) scan codes."""
        return SCAN.get(self.modifier, UNRESOLVED), SCAN.get(self.key, UNRESOLVED)

    @property
    def code(self) -> int:
        """The command as the engine numbers it, from whichever binary knows it."""
        return CMD.get(self.command, UNRESOLVED)

    @property
    def handler(self) -> str:
        """Which binary resolves this command: ``World3D.dll`` or ``iron3d.dll``.

        ``CMD_CAMERA_INFRARED`` is in both chains; it is reported as the
        object command it also is.
        """
        if self.command in CMD_OBJECT:
            return "World3D.dll"
        if self.command in CMD_GAME:
            return "iron3d.dll"
        return ""


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

    @property
    def code(self) -> int:
        """The command as the engine numbers it."""
        return MCMD.get(self.command, UNRESOLVED)

    @property
    def class_id(self) -> int:
        """The target class as the engine numbers it.  ``CICLS_UNKNOWN`` is 0."""
        return CICLS.get(self.target, UNKNOWN_CLASS)

    @property
    def bits(self) -> int:
        """The state field's value: a ``CIS_`` bit, a ``MAN_`` flag, or 0."""
        if self.state in CIS:
            return CIS[self.state]
        return MAN.get(self.state, 0)

    @property
    def dispatched(self) -> bool:
        """True when the movement controller handles this command itself."""
        return self.code in DISPATCHED


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
