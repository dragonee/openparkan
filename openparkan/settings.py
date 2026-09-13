"""The engine's own configuration, and the progress it keeps.

Five text files sit outside the data, and each one belongs to a module that
carries its name and its keys:

| file | read by | what it is |
|---|---|---|
| ``Comp.ini`` | ``World3D.dll`` | the component registry |
| ``Behavior.ini`` | ``Behavior.dll`` | the AI's logging and debug switches |
| ``ArealMap.ini`` | ``ArealMap.dll`` | the navigation mesh's, the same shape |
| ``Iron_3D.ini`` | ``iron3d.dll`` | display, input, multiplayer, difficulty |
| ``MISSIONS/dispatcher.ini`` | ``iron3d.dll`` | which missions have been completed |

*Measured*: each module's binary is the only one holding that file's name and
its switch names -- so the pairing is the game's, not a guess.  The last two
are **written by the game**, not shipped: they are settings and progress, and
what they hold is one player's, not a fact about the format.

## ``Comp.ini`` -- the Component Address File

Eight rows, and the engine's own words for them come out of ``World3D.dll``,
whose reader at ``0x10014790`` is ``LoadComponentAddr`` and whose error says
*Component Address File not found*::

    0  terrain.dll  LoadLandscape      // comments...
    3  animesh.dll  LoadAgent          // comments...
    7  misload.dll  LoadResearch       // comments...

The reader opens the file ``"rt"``, **skips any line under five characters and
any line starting with** ``//``, and takes the rest with ``sscanf``'s
``"%d %s %s"`` -- which is why a trailing comment needs no delimiter.  Each
row becomes a 16-byte record: the id, the module handle from ``LoadLibraryA``,
and the entry point from ``GetProcAddress``.  A row whose DLL or function is
missing is dropped, because the table's counter only advances on success.

**All eight functions are real exports of the DLL named beside them**
(``analysis/registry.py``).  The ids are contiguous 0 to 7 and the file's own
comment header names each one -- ``CID_CLASSIC_LANDSCAPE`` 0 through
``CID_RESEARCH`` 7.  Those names appear in **no binary**: the engine dispatches
on the bare number and the names are there for the reader.

## The two debug files

``Behavior.ini`` (14 switches) and ``ArealMap.ini`` (10) share a five-switch
logging preamble -- ``LogFile``, ``SaveLog``, ``MaxErrorLevel``,
``DefErrorLevel``, ``LookBugMode`` -- so the two modules were built on one
framework.  Everything else is a module's own: ``LockBehaviour`` and
``GiveDefaultOrder`` in ``Behavior.dll``, the strategy layer, ``ShowAreals``
and ``HallWay_NoZBuffer`` for the navigation mesh (`08-arealmap.md`).
``ai.dll``, which runs the ``.scr`` scripts (`15-behaviour.md`), has no
configuration file at all.

## ``Iron_3D.ini``

Four sections, 33 keys, all written by the game.  ``[CS]`` is the display and
input, ``[MULTIPLAYER]`` a login, ``[TEMP]`` a pair of ranges the interface
normalises against (``OFFENCE_MIN``/``MAX``, ``DEFENCE_MIN``/``MAX``), and
``[LEVEL_RATIO]`` three difficulty multipliers that ``GAME_LEVEL`` picks
between.

## ``dispatcher.ini``

One ``[COMPLETE]`` section, one key per mission finished, value 1.  **The key
is the mission's own directory path** with every separator and dot flattened
to an underscore, lowercased, keeping the trailing separator:

    MISSIONS\\CAMPAIGN\\CAMPAIGN.00\\Mission.01\\
    missions_campaign_campaign_00_mission_01_

so it round-trips against the directory tree.

Everything above is re-derived by ``uv run openparkan verify``.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path

#: The component registry, in the installation root.
COMPONENTS_FILE = "Comp.ini"

#: The two debug files beside it.
BEHAVIOUR_FILE = "Behavior.ini"
AREALMAP_FILE = "ArealMap.ini"

#: The player's own settings, which the game rewrites.
DISPLAY_FILE = "Iron_3D.ini"

#: And the player's progress.
DISPATCHER_FILE = ("MISSIONS", "dispatcher.ini")

#: How many components the registry declares.
COMPONENTS = 8

#: ``World3D.dll``'s ``LoadObjectFromDisk`` (``0x10007a50``): the object class
#: a caller asks for, and the registry id it loads it through.  Classes 6 and
#: 8 are illegal; the shader (id 6) is reached only by ``LoadComponent``
#: (``0x10014980``), which takes the id itself.  Buildings are class 3 and
#: robots 4 (``ArealMap.dll``'s ``CreateObjectFromScheme``), what a controller
#: emits 9 (``Control.dll``), mission scenery 10 and the research tree 11.
OBJECT_CLASSES = {1: 0, 2: 3, 3: 1, 4: 3, 5: 2, 7: 5, 9: 3, 10: 4, 11: 7}

#: Each loader's allocation and the interface offset it returns, by entry
#: point.  All take ``(library, member, 0, player)``.
LOADERS = {
    "LoadLandscape": (0x7D40, 0),
    "LoadBuilding": (0xFC, 0x8),
    "LoadCamera": (0x1A4, 0x134),
    "LoadAgent": (0x7BC, 0x130),
    "CreateAtmosphere": (0x1AC, 0x138),
    "CreateShader": (0xB8, 0),
    "LoadResearch": (0x138, 0),
}

#: ``Behavior.ini``'s ``DefaultOrderPhase`` is compared with a behaviour's
#: ``+0xa00``, which nothing but its constructor writes (0), before
#: ``GiveDefaultOrder`` hands a battle robot order 13 and the hero order 6
#: (``Behavior.dll:0x10004c80``).
DEFAULT_ORDER_FIELD = 0xA00

#: A ``[CS]`` key of ``Iron_3D.ini`` the shipped file does not carry.  Non-zero
#: silences the research tree's debug-information warning, and four part-list
#: builders of the panels take its inverse as a flag (``iron3d.dll:0x1008ac50``).
FULL_RESEARCH_TREE = "FULL_RESEARCH_TREE"

#: The switches both debug files carry.
LOGGING = ("LogFile", "SaveLog", "MaxErrorLevel", "DefErrorLevel", "LookBugMode")

#: The section a completed mission is recorded in.
COMPLETE = "COMPLETE"

#: What ``dispatcher.ini`` writes against a mission it has seen finished.
DONE = "1"

#: A row is skipped below this length, and a comment opens with this.
MIN_ROW = 5
COMMENT = "//"

#: ``CID_CLASSIC_LANDSCAPE   0`` in the file's own comment header.
_NAMED = re.compile(r"^//\s*(CID_[A-Z0-9_]+)\s+(\d+)\s*$")


@dataclass(frozen=True)
class Component:
    """One registry row: an id, and the entry point that loads it."""

    cid: int
    name: str
    dll: str
    function: str
    comment: str = ""


def _text(path: str | Path) -> list[str]:
    return Path(path).read_bytes().decode("latin-1").splitlines()


def component_names(path: str | Path) -> dict[int, str]:
    """The ``CID_*`` names the registry's comment header declares."""
    out: dict[int, str] = {}
    for line in _text(path):
        match = _NAMED.match(line.strip())
        if match:
            out[int(match.group(2))] = match.group(1)
    return out


def registry(path: str | Path) -> list[Component]:
    """Read ``Comp.ini``, the way the engine reads it.

    Short lines and ``//`` comments are dropped and the rest is taken as
    ``"%d %s %s"``, so whatever follows the function name is free text.
    """
    names = component_names(path)
    out = []
    for line in _text(path):
        if len(line.strip()) < MIN_ROW or line.lstrip().startswith(COMMENT):
            continue
        fields = line.split(None, 3)
        if len(fields) < 3 or not fields[0].lstrip("-").isdigit():
            continue
        cid = int(fields[0])
        out.append(Component(
            cid=cid,
            name=names.get(cid, ""),
            dll=fields[1],
            function=fields[2],
            comment=fields[3].strip() if len(fields) > 3 else "",
        ))
    return out


def switches(path: str | Path) -> dict[str, str]:
    """A flat ``key = value`` file with ``//`` comments."""
    out: dict[str, str] = {}
    for line in _text(path):
        line = line.split(COMMENT, 1)[0].strip()
        if "=" in line:
            key, value = line.split("=", 1)
            out[key.strip()] = value.strip()
    return out


def sections(path: str | Path) -> dict[str, dict[str, str]]:
    """A ``[SECTION]`` file.  Keys before any section head are dropped."""
    out: dict[str, dict[str, str]] = {}
    current: dict[str, str] | None = None
    for line in _text(path):
        line = line.strip()
        if line.startswith("[") and line.endswith("]"):
            current = out.setdefault(line[1:-1], {})
        elif "=" in line and current is not None:
            key, value = line.split("=", 1)
            current[key.strip()] = value.strip()
    return out


def dispatcher_key(game: str | Path, directory: str | Path) -> str:
    """The key ``dispatcher.ini`` writes for a mission directory."""
    relative = Path(directory).resolve().relative_to(Path(game).resolve())
    return (relative.as_posix() + "/").replace("/", "_").replace(".", "_").lower()


def completed(game: str | Path) -> dict[str, str]:
    """The ``[COMPLETE]`` section, or empty if the game has not written one."""
    game = Path(game)
    path = game.joinpath(*DISPATCHER_FILE)
    if not path.exists():
        return {}
    return sections(path).get(COMPLETE, {})
