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
BUILDING_COST = 100.0


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
