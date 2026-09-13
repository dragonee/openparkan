"""``behpsp.res``: the behaviour profiles, and the economy they carry.

An NRes archive of 32 binary ``.var`` files, one per role -- a building type
(``prof_mine``, ``prof_generator``, ``prof_storage``, ``prof_plant``, ...), a
chassis (``chas_*``), a character, or a difficulty level (``diff_*``).  Each
file is a list of named, typed variables with a value, a default and bounds,
the same shape a mission property has::

    uint32   count
    count x  uint32   4          the size of a value
             uint32   type       2 BOOL, 3 float, 5 DWORD
             uint32   0
             uint32   name length
             char[n]  name       no terminator
             uint32   0
             4 x      value, default, minimum, maximum

All 32 members walk to the byte.  ``Behavior.dll`` binds the variables to a
struct by name through ``MVarSet::LinkVar``, checking the type name against
the three above; see ``docs/23-economy.md``.

The economy is in the building profiles.  ``Transfer_Power_Out`` is what a
building puts into its clan's power pool each second, ``Use_Power`` and
``Use_Ore`` what it draws, ``Store_Ore_Maximum`` how much ore it holds, and the
two ``Transfer_Ore_*`` rates how fast ore moves on and off it.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass
from pathlib import Path

from .nres import NResArchive

ARCHIVE = "behpsp.res"

TYPE_BOOL = 2
TYPE_FLOAT = 3
TYPE_DWORD = 5
TYPES = {TYPE_BOOL: "BOOL", TYPE_FLOAT: "float", TYPE_DWORD: "DWORD"}

VALUE_SIZE = 4
RECORD_FIXED = 16 + 20

#: The building-profile variables that make up the economy.
POWER_OUT = "Transfer_Power_Out"
POWER_IN = "Transfer_Power_In"
USE_POWER = "Use_Power"
USE_ORE = "Use_Ore"
ORE_MAXIMUM = "Store_Ore_Maximum"
ORE_NOW = "Store_Ore_Now"
ORE_ON = "Transfer_Ore_OnBoard"
ORE_OFF = "Transfer_Ore_OffBoard"


#: The engine's own economy constants, compiled into ``Behavior.dll`` by the
#: constructor at ``0x10016250`` and bound by name at ``0x10016480``.  No file
#: in the installation names any of them, so these are the values the game
#: runs with.  The mission editor wrote three of them into every placed mine,
#: transport and storage as its ``MaximumOre`` property.
MINE_ORE_PER_SECOND = 50.0
MINE_MAX_ORE = 500.0
STORAGE_MAX_ORE = 4000.0
TRANSPORT_MAX_ORE = 2000.0
TRANSPORT_ORE_ON_PER_SECOND = 100.0
TRANSPORT_ORE_OFF_PER_SECOND = 100.0
TRANSPORT_BUILDING_DIST = 80.0
#: Not a building's price, which is the sum of its parts' build ore
#: (``Behavior.dll:0x10029810``): what a builder must hold to set off for a
#: site, and 1.5 times it is what a mine or storage must hold before a builder
#: fetches ore from it (``0x10028ff0``, ``0x10029110``).
BUILDING_COST = 100.0
#: How long an upgrade takes at the building, in seconds (``0x1003363f``).
UPGRADE_SECONDS = 50.0

#: The HUD's ore bar is ore held in the clan's mines and storages over this --
#: ``iron3d.dll:0x1006d927`` multiplies by 1/4500.  It is one full mine plus
#: one full storage, so a lone full mine reads 11%.
HUD_ORE_FULL = MINE_MAX_ORE + STORAGE_MAX_ORE

#: The four per-building grants a mission can set, by property name, and what
#: ``MBehaviour`` holds in their fields before a mission does
#: (``Behavior.dll:0x10003a3c``).  A factory builds its ``FreeBotNum`` bots at
#: no cost; a research centre researches its ``FreeTechnoNum`` technologies at
#: no cost; and ``FreeResearchTime`` is the time budget of *every* research
#: the building starts, free or not, in seconds.
FREE_BOTS = "FreeBotNum"
FREE_TECHNOLOGIES = "FreeTechnoNum"
FREE_CONSTRUCTION_TIME = "FreeConstructionTime"
FREE_RESEARCH_TIME = "FreeResearchTime"
CONSTRUCTION_TIME_DEFAULT = 5.0
RESEARCH_TIME_DEFAULT = 2.0

#: How long a paid bot takes, in seconds, whatever it is: the construction
#: task starts every build with this budget (``Behavior.dll:0x10029bea``) and
#: only a free bot replaces it.
BUILD_SECONDS = 5.0
#: A size class from a name's size letter.  A chassis carries it third
#: (``R_T_02``) and a building fourth (``fr_l_plant``); a factory builds a
#: chassis no bigger than itself.  ``Behavior.dll:0x10029e10`` for chassis,
#: ``0x1000cee0`` for buildings.
CHASSIS_SIZE = {"t": 1, "l": 2, "h": 2, "m": 3, "b": 4}
BUILDING_SIZE = {"l": 2, "m": 3, "b": 4, "e": 5}
#: A free bot's build time in seconds, by factory size and chassis size
#: (``0x1002a000``); any pair not listed takes 20.
FREE_BOT_SECONDS = {
    (2, 1): 30.0, (2, 2): 60.0,
    (3, 1): 20.0, (3, 2): 35.0, (3, 3): 60.0,
    (4, 1): 10.0, (4, 2): 20.0, (4, 3): 40.0, (4, 4): 60.0,
}
FREE_BOT_SECONDS_OTHERWISE = 20.0
#: What a free bot costs instead of its technology's price.
FREE_BOT_ORE = 0.0
FREE_BOT_POWER = 1.0


#: A chassis profile's ``ChassisType``.  The four ``chas_*.var`` profiles use
#: 1-4; ``chas_worm.var``'s 5 is on no shipped chassis.
CHASSIS_TYPE = {0: "building", 1: "flying", 2: "walking", 3: "wheeled", 4: "tracked",
                5: "worm"}


class ProfileFormatError(ValueError):
    pass


@dataclass(frozen=True)
class Variable:
    name: str
    type: int
    value: float | int
    default: float | int
    minimum: float | int
    maximum: float | int


def parse(blob: bytes, source: str = "<var>") -> dict[str, Variable]:
    """One ``.var`` file, as ``name -> Variable`` in file order."""
    if len(blob) < 4:
        raise ProfileFormatError(f"{source}: too short")
    count = struct.unpack_from("<I", blob, 0)[0]
    pos = 4
    out: dict[str, Variable] = {}
    for _ in range(count):
        if pos + 16 > len(blob):
            raise ProfileFormatError(f"{source}: record header runs past the end")
        size, typ, _zero, length = struct.unpack_from("<4I", blob, pos)
        if size != VALUE_SIZE or typ not in TYPES:
            raise ProfileFormatError(f"{source}: record ({size}, {typ}) at {pos}")
        end = pos + 16 + length + 20
        if end > len(blob):
            raise ProfileFormatError(f"{source}: record at {pos} runs past the end")
        name = blob[pos + 16:pos + 16 + length].decode("latin-1")
        fmt = "<4f" if typ == TYPE_FLOAT else "<4i"
        value, default, low, high = struct.unpack_from(fmt, blob, pos + 16 + length + 4)
        out[name] = Variable(name, typ, value, default, low, high)
        pos = end
    if pos != len(blob):
        raise ProfileFormatError(f"{source}: {len(blob) - pos} bytes left over")
    return out


def load(game: str | Path) -> dict[str, dict[str, Variable]]:
    """Every profile in the installation's ``behpsp.res``, by member name."""
    archive = NResArchive.open(Path(game) / ARCHIVE)
    return {entry.name: parse(archive.read(entry), entry.name) for entry in archive}
