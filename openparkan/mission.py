"""Reader for ``data.tma`` -- a Parkan mission: clans, object placement, routes.

A mission directory holds ``data.tma`` alongside plain-text ``mission.cfg``,
``descr`` and briefing files.  ``data.tma`` is the binary part and carries
everything the engine needs to populate a map.

Strings are length-prefixed: a ``uint32`` byte count followed by that many
bytes, with no NUL terminator.

The trailer's description field is the one exception, and it is a genuine wart
in the shipped data rather than something still to be decoded: its length word
is the *capacity* of a fixed-size buffer, and everything past the real text is
whatever was in memory at save time -- sometimes MSVC's 0xCD debug fill,
sometimes fragments of other strings.  ``Mission.description`` therefore
carries junk for several missions and there is no in-band way to find the real
end.  Use the mission directory's plain-text ``descr`` file for display;
``Mission.descr_file`` reads it.

File layout::

    uint32   version, always 1
    uint32   route count
    routes   { uint32 id; uint32 point count; float32[3] * count }
    uint32   always 6
    uint32   clan count
    clans    (see _read_clan)
    uint32   always 10; uint32 object count
    objects  (see _read_object)
    trailer  map path, description, per-clan viewpoints

See ``docs/04-missions.md``.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field
from pathlib import Path

#: Second word of every object record; absent at the start of the trailer.
OBJECT_MARKER = 0x80000002

#: Property value type tags.
TYPE_FLOAT = 0
TYPE_INT = 1

#: MSVC uninitialised-memory fill, found in the tail of fixed-size string buffers.
DEBUG_FILL = 0xCD

#: An object record's first word says what kind of thing is being placed, and
#: with it how the ``path`` field resolves.  Kinds 0 and 1 name a ``.dat``
#: definition file under ``UNITS/``; kinds 2 and 3 name a ``STAT`` member of
#: ``objects.rlb``.  In the shipped missions kind 2 is always vegetation and
#: kind 3 always rock, but nothing in the format says it must be.
KIND_BUILDING = 0
KIND_UNIT = 1
KIND_VEGETATION = 2
KIND_ROCK = 3

KIND_NAMES = {
    KIND_BUILDING: "building",
    KIND_UNIT: "unit",
    KIND_VEGETATION: "vegetation",
    KIND_ROCK: "rock",
}


class MissionFormatError(ValueError):
    pass


class _Reader:
    """Cursor over a mission file, in the file's own little-endian encoding."""

    def __init__(self, data: bytes, source: str):
        self.data = data
        self.source = source
        self.pos = 0

    def _unpack(self, fmt: str, size: int):
        if self.pos + size > len(self.data):
            raise MissionFormatError(
                f"{self.source}: read past end of file at offset {self.pos}"
            )
        out = struct.unpack_from(fmt, self.data, self.pos)
        self.pos += size
        return out

    def u32(self) -> int:
        return self._unpack("<I", 4)[0]

    def i32(self) -> int:
        return self._unpack("<i", 4)[0]

    def f32(self) -> float:
        return self._unpack("<f", 4)[0]

    def vec3(self) -> tuple[float, float, float]:
        return self._unpack("<3f", 12)

    def string(self) -> str:
        n = self.u32()
        if n > 4096 or self.pos + n > len(self.data):
            raise MissionFormatError(
                f"{self.source}: implausible string length {n} at offset {self.pos - 4}"
            )
        raw = self.data[self.pos : self.pos + n]
        self.pos += n
        # Trim the debug fill where it is present; see the module docstring for
        # why this cannot be relied on to remove every uninitialised tail.
        cut = raw.find(bytes([DEBUG_FILL]))
        if cut != -1:
            raw = raw[:cut]
        return raw.rstrip(b"\0").decode("latin-1")

    def peek_u32(self, words_ahead: int = 0) -> int | None:
        at = self.pos + words_ahead * 4
        if at + 4 > len(self.data):
            return None
        return struct.unpack_from("<I", self.data, at)[0]

    @property
    def exhausted(self) -> bool:
        return self.pos >= len(self.data)


#: An int property's maximum when it has none -- ``ClanID`` uses it.
NO_MAXIMUM = -1


@dataclass
class Property:
    """One entry of an object's property table.

    A value and its bounds: ``b`` is the **minimum** and ``c`` the
    **maximum**.  Every instance of fourteen of the fifteen shipped property
    names keeps its value inside them, and the bounds read as what they are --
    ``0..1`` for a fraction, ``0..INT_MAX`` for a count, ``2..1000`` for a
    time.  Three conventions sit on top:

    * ``c == -1`` on an int means no maximum (``ClanID``);
    * ``b == c == value`` locks the value (``LogicalID``, ``Type``,
      ``ChargeRadius``);
    * ``CurrentOre``'s maximum is the same object's ``MaximumOre`` value, on
      all 463, and its minimum was never initialised -- denormal floats such
      as 6.45e-39 where it is not zero.
    """

    name: str
    type: int
    value: float | int
    b: float | int
    c: float | int

    @property
    def is_float(self) -> bool:
        return self.type == TYPE_FLOAT

    @property
    def minimum(self) -> float | int:
        return self.b

    @property
    def maximum(self) -> float | int | None:
        """The upper bound, or None where an int property says it has none."""
        return None if (not self.is_float and self.c == NO_MAXIMUM) else self.c

    @property
    def locked(self) -> bool:
        """Bounds collapsed onto the value: not something to edit."""
        return self.b == self.c == self.value


@dataclass
class Zone:
    """A circular zone attached to a clan: a centre and two radii.

    Present only on campaign missions, where the radii come in pairs such as
    20/40 and 10/30, so they read as an inner and an outer bound.
    """

    position: tuple[float, float, float]
    inner: float
    outer: float
    kind: int = 1


@dataclass
class Clan:
    name: str
    index: int
    base: tuple[float, float]
    ai_script: str
    behaviour: str
    zones: list[Zone] = field(default_factory=list)
    #: clan name -> relation word (1 towards itself, 0 towards the others in
    #: every shipped mission, so it reads as an alliance matrix)
    relations: dict[str, int] = field(default_factory=dict)
    #: ``(parent, minds)`` as the file holds them; see ``minds``.
    unknown: tuple[int, int] = (0, 0)

    @property
    def minds(self) -> int:
        """How many bots the clan can have at once -- the game's "CPUs".

        The word after the behaviour-tree path.  ``iron3d.dll:0x10039266``
        fills the clan SuperAI's mind list with this many free slots; a factory
        will not start a bot without one (``Behavior.dll:0x10029ba0``), a bot
        under construction already holds one, and a bot's is given back when it
        is destroyed or captured.  No placed clan exceeds it.
        """
        return self.unknown[1]


@dataclass
class MissionObject:
    path: str
    name: str
    logical_id: int
    position: tuple[float, float, float]
    rotation: float
    scale: tuple[float, float, float]
    properties: dict[str, Property] = field(default_factory=dict)
    #: See KIND_* above.
    kind: int = KIND_BUILDING
    unknown: tuple = ()

    @property
    def kind_name(self) -> str:
        return KIND_NAMES.get(self.kind, "?")

    @property
    def is_static(self) -> bool:
        """True when ``path`` names a STAT member of objects.rlb, not a file."""
        return self.kind in (KIND_VEGETATION, KIND_ROCK)

    @property
    def clan_id(self) -> int | None:
        p = self.properties.get("ClanID")
        return int(p.value) if p else None

    @property
    def type_id(self) -> int | None:
        p = self.properties.get("Type")
        return int(p.value) if p else None

    @property
    def category(self) -> str:
        """``BUILDS``/``UNITS``/... from the definition path; ``STATIC`` for scenery."""
        if self.is_static:
            return "STATIC"
        parts = self.path.replace("\\", "/").split("/")
        return parts[1].upper() if len(parts) > 2 else "?"


@dataclass
class Route:
    id: int
    points: list[tuple[float, float, float]]


@dataclass
class Viewpoint:
    position: tuple[float, float, float]
    unknown: tuple[int, int, int, int]


@dataclass
class Mission:
    source: Path
    version: int
    routes: list[Route]
    clans: list[Clan]
    objects: list[MissionObject]
    map_path: str
    description: str
    viewpoints: list[Viewpoint]
    unknown_pre_objects: int

    @property
    def descr_file(self) -> str:
        """The mission's plain-text ``descr`` file, which is clean; '' if absent."""
        f = self.source.parent / "descr"
        if not f.exists():
            return ""
        return f.read_bytes().decode("latin-1").strip()

    @property
    def title(self) -> str:
        """Best available human-readable description."""
        return self.descr_file or self.description

    @property
    def map_name(self) -> str:
        """``DATA\\MAPS\\SC_3\\land`` -> ``SC_3``."""
        parts = self.map_path.replace("\\", "/").rstrip("/").split("/")
        return parts[-2] if len(parts) >= 2 else ""

    def objects_of_clan(self, clan_id: int) -> list[MissionObject]:
        return [o for o in self.objects if o.clan_id == clan_id]

    def clan_by_index(self, index: int) -> Clan | None:
        return next((c for c in self.clans if c.index == index), None)


def _read_clan(r: _Reader) -> Clan:
    name = r.string()
    parent = r.i32()
    x, y = r.f32(), r.f32()
    index = r.u32()
    ai = r.string()
    zones = []
    for _ in range(r.u32()):
        kind = r.u32()
        zones.append(Zone(r.vec3(), r.f32(), r.f32(), kind))
    behaviour = r.string()
    k2 = r.u32()
    relations = {}
    for _ in range(r.u32()):
        other = r.string()
        relations[other] = r.u32()
    return Clan(name, index, (x, y), ai, behaviour, zones, relations, (parent, k2))


def _read_object(r: _Reader) -> MissionObject:
    kind = r.u32()
    marker = r.u32()
    if marker != OBJECT_MARKER:
        raise MissionFormatError(
            f"{r.source}: expected object marker at {r.pos - 4}, found 0x{marker:08x}"
        )
    path = r.string()
    q = r.u32()
    logical_id = r.i32()
    position = r.vec3()
    pad = (r.u32(), r.u32())
    rotation = r.f32()
    scale = r.vec3()
    name = r.string()
    tail = (r.u32(), r.i32(), r.i32(), r.u32())

    properties: dict[str, Property] = {}
    for _ in range(r.u32()):
        ptype = r.u32()
        raw = (r.u32(), r.u32(), r.u32())
        pname = r.string()
        if ptype == TYPE_FLOAT:
            a, b, c = (struct.unpack("<f", struct.pack("<I", v))[0] for v in raw)
        else:
            a, b, c = (v - (1 << 32) if v > 0x7FFFFFFF else v for v in raw)
        properties[pname] = Property(pname, ptype, a, b, c)

    return MissionObject(
        path=path,
        name=name,
        logical_id=logical_id,
        position=position,
        rotation=rotation,
        scale=scale,
        properties=properties,
        kind=kind,
        unknown=(q, pad, tail),
    )


def load(path: str | Path) -> Mission:
    """Parse a ``data.tma``.  Raises MissionFormatError if the layout does not hold."""
    path = Path(path)
    r = _Reader(path.read_bytes(), str(path))

    version = r.u32()
    routes = []
    for _ in range(r.u32()):
        rid = r.u32()
        routes.append(Route(rid, [r.vec3() for _ in range(r.u32())]))

    r.u32()  # always 6 in every shipped mission
    clans = [_read_clan(r) for _ in range(r.u32())]

    pre = r.u32()  # always 10
    objects = [_read_object(r) for _ in range(r.u32())]

    map_path = r.string()
    r.u32()
    description = r.string()
    r.u32()
    viewpoints = [
        Viewpoint(r.vec3(), (r.u32(), r.u32(), r.u32(), r.u32()))
        for _ in range(r.u32())
    ]

    if not r.exhausted:
        raise MissionFormatError(
            f"{path}: {len(r.data) - r.pos} bytes left over after the trailer"
        )

    return Mission(
        source=path,
        version=version,
        routes=routes,
        clans=clans,
        objects=objects,
        map_path=map_path,
        description=description,
        viewpoints=viewpoints,
        unknown_pre_objects=pre,
    )


# --------------------------------------------------------------------------
# mission.cfg -- the plain-text half of a mission
# --------------------------------------------------------------------------


def load_cfg(path: str | Path) -> dict[str, dict[str, str]]:
    """Parse ``mission.cfg``'s ``object NAME ... end`` blocks.

    Returns ``{object_name: {property: value}}``.  Values keep their source
    form minus surrounding quotes.  A shipped comment in the file notes that
    the object names are what matter, not the property names.
    """
    out: dict[str, dict[str, str]] = {}
    current: dict[str, str] | None = None
    for raw in Path(path).read_bytes().decode("latin-1").splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        if line.lower() == "end":
            current = None
        elif line.lower().startswith("object"):
            name = line.split(None, 1)[1].strip() if len(line.split(None, 1)) > 1 else ""
            current = out.setdefault(name, {})
        elif current is not None and "=" in line:
            key, value = line.split("=", 1)
            current[key.strip()] = value.strip().strip('"')
    return out
