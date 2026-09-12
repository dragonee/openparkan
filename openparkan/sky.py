"""Reader for ``sky.ske`` -- the per-mission **atmosphere**.

Not a skybox.  ``Terrain.dll`` calls this an atmosphere file and names the
tool that wrote it: *"SunDll panic : Old version ske file / ReSave in
SunEditor"*.  Inside are ``CAtmosphere``, ``CAtmData``, ``CSun``,
``CreateAtmosphereObject``, "Illegal atmosphere object type", and the
settings ``AtmSkyDetail``, ``AtmStarsOn``, ``AtmCloudsOn``, ``LensFlareOn``.

What the file holds is a **day cycle**: a list of keyframes, each stamped with
an hour and a minute, carrying the colours and intensities the sky takes at
that time.  On ``CAMPAIGN.04/Mission.01`` the twelve keyframes run 00:20,
01:24, 06:48, 12:34, 13:58, 15:00, 15:48, 19:20, 22:39, 23:20, 23:59, and the
first colour group goes from ``#000025`` at twenty past midnight to ``#563868``
at half past twelve while the light intensity climbs from 0.1 to 5.0.

Layout, which accounts for all 29 shipped files to the byte::

    file header, 124 bytes
        int32   -1                     magic
        int32   5                      version
        int32   section count, 1 or 2
        int32   1
        int32   keyframes in the first section
        ...     see FileHeader below
    keyframe x count
    for each further section:
        72 bytes                       the same block as bytes 52..123
        keyframes to the end of the file

and one keyframe is::

    88 bytes    22 slots: mostly BGRA colours, three of them float32
    6 x string  the object's name in one slot, empty in the rest
    float32[4]  intensities; the third tracks the sun through the day
    int32 n
    n x string  sound files -- "atm_rain1.wav" is the only one shipped
    uint32[10]  a kind word, then the hour and minute

A string is an ``int32`` length followed by that many bytes and no
terminator, which is what ``MFile``'s string reader does.

The engine builds five kinds of atmosphere object -- **SUN, SKY, RAIN, SNOW
and LIGHTNING** -- and they are numbered, out of the jump table the type-name
switch uses: ``SUN`` 0, ``SKY`` 1, ``RAIN`` 2, ``SNOW`` 3, ``LIGHTNING`` 4.

The ten-valued opcode ``CAtmData::GetEvents`` dispatches on decodes
completely, and not by the obvious rule: each case *writes* a phase and a type
into a 20-byte event record, and read back off those writes the table is

    0 start SUN    1 stop SUN    2 nothing
    3 start RAIN   4 stop RAIN
    5 start SNOW   6 stop SNOW   7 nothing
    8 start LIGHTNING   9 stop LIGHTNING

**2 and 7 do nothing** -- they share the switch's default -- and **SKY has no
case at all**, which agrees with the sky being created outside the switch with
a hardcoded id.  Phase 0 is the create side: the handler that takes it looks
the object up and warns *"Atmosphere object already exists"*.

Which field of a keyframe carries that opcode is still **not established**,
and the last word of the trailer -- the only field that spans 0 to 9 -- is
ruled out for a sharper reason than before: 7 means *nothing happens*, and
that word puts **all 59 ``moon`` keyframes and 60 of the 75 ``sun`` ones**
there, which cannot be right for bodies that must be started and stopped.
See ``docs/10-sky.md``.

The sibling ``sky.wea`` names the textures, in the same format model wears
use, and **the slot index is the role**: the same nine slots in the same order
in all 29 missions, with slots 1, 5, 6 and 8 naming the identical texture
every time.  They are material names, so they resolve through ``Material.lib``
exactly as the terrain's do -- and the material picks a cell of a sprite sheet
as well as a texture, which is how one 2 x 2 ``SUN.0`` provides both the sun
and the moon.
"""

from __future__ import annotations

import math
import struct
from dataclasses import dataclass, field
from pathlib import Path

from .mesh import read_wea

MAGIC = 0xFFFFFFFF
VERSION = 5
HEADER_SIZE = 124
#: Bytes 52..123 of the file header, repeated ahead of every later section.
SECTION_HEADER_SIZE = 72
SLOT_COUNT = 22
NAME_SLOTS = 6
#: Slots of the 88-byte block that hold a float32 rather than a colour.
FLOAT_SLOTS = (6,)
#: Slots whose colour tracks the time of day; see the module docstring.
DAY_CYCLE_SLOTS = (1, 2, 3, 4)

#: The five kinds of atmosphere object, numbered as ``Terrain.dll``'s
#: type-name switch numbers them (the jump table at ``0x10070024``).
OBJECT_TYPES = {0: "SUN", 1: "SKY", 2: "RAIN", 3: "SNOW", 4: "LIGHTNING"}

#: What an event does to its object.  Phase 0 is the create side.
PHASES = {0: "start", 1: "stop"}

#: ``CAtmData::GetEvents``' ten opcodes, as ``(phase, type)`` -- read off what
#: each case of the switch at ``0x1006e829`` writes into the event record.
#: 2 and 7 are absent: they share the switch's default and do nothing, and
#: ``SKY`` never appears because the sky is created outside the switch.
EVENT_OPCODES = {
    0: (0, 0), 1: (1, 0),
    3: (0, 2), 4: (1, 2),
    5: (0, 3), 6: (1, 3),
    8: (0, 4), 9: (1, 4),
}

#: The opcodes that reach the default and do nothing.
NO_EVENT = (2, 7)

#: One event record the handler walks: phase, type, and two words of time.
EVENT_RECORD = 20

#: The trailer word that spans the opcode range but is **not** the opcode;
#: see the module docstring.
OPCODE_CANDIDATE = 9

#: ``sky.wea`` slot -> what it is.  Fixed across all 29 missions.
SLOT_ROLES = (
    "nebula",
    "stars",
    "clouds",
    "sun",
    "moon",
    "flare",
    "flare2",
    "snow",
    "rain",
)

#: The lens flare, as ``Terrain.dll``'s ``CSun::RenderFlare`` draws it: twelve
#: sprites strung along the line from the sun's position on screen through the
#: centre of the screen, each ``(position, size, colour, texture)``.
#:
#: ``position`` is the fraction of that line -- 1 is the sun itself, 0 the
#: middle of the screen, negative the far side, and the first element sits at
#: 1.2, past the sun.  ``size`` is scaled by ``FLARE_SCALE`` and half the
#: viewport width, so the largest ghost is an eighth of the screen across.
#: ``colour`` is a D3D ARGB constant whose alpha the engine multiplies by the
#: flare's own intensity.  ``texture`` picks between the two flare slots of
#: ``sky.wea`` -- which of the pair the engine calls 0 is not established, so
#: this takes 0 as ``flare`` and 1 as ``flare2``.
FLARE_ELEMENTS = (
    (1.2, 0.2, 0xFFB090A3, 0),
    (0.7, 0.3, 0xFF5A58BB, 0),
    (0.5, 0.2, 0x9630BE52, 1),
    (0.2, 0.1, 0x96C93432, 1),
    (0.0, 0.1, 0xFF30BE52, 0),
    (-0.2, 0.3, 0x96969664, 1),
    (-0.3, 0.3, 0xFFB090A3, 0),
    (-0.5, 0.7, 0xFF7C6BC9, 0),
    (-0.6, 0.4, 0x96306452, 1),
    (-0.8, 1.0, 0xFF0B17B9, 0),
    (-1.0, 0.3, 0xFFB626B1, 0),
    (-1.1, 0.2, 0xFF7CC5C9, 0),
)

#: A keyframe names the atmosphere object it belongs to.  Four names appear:
#: ``sun`` and ``moon``, which come in a start/stop pair, and these two, which
#: appear once each in a section.  No shipped mission names snow.
RAIN_MARKER = "atm_rain1.wav"
LIGHTNING_MARKER = "env_lightning"

#: A ghost's half-size is ``FLARE_SCALE * (viewport width / 2) * size``.
FLARE_SCALE = 0.25

#: The flare is off when the sun is more than this many degrees off the view
#: axis, and ramps linearly to full on-axis; the engine then squares the ramp.
FLARE_CONE_DEGREES = 15.0

#: The second gate is a ramp on the **height** of the body's own direction --
#: the engine negates the vector's third component and compares it against
#: these two cosines, so the flare is full at 60 degrees of elevation and above
#: and out at 30 degrees and below.  Written as heights rather than as angles
#: because that is what the comparison is against, and because writing them as
#: elevations inverts the two.
FLARE_HEIGHT_FULL = math.cos(math.radians(30.0))
FLARE_HEIGHT_ZERO = math.cos(math.radians(60.0))


def flare_height_gate(height: float) -> float:
    """The second gate, given the height of a unit direction."""
    if height >= FLARE_HEIGHT_FULL:
        return 1.0
    if height <= FLARE_HEIGHT_ZERO:
        return 0.0
    return (height - FLARE_HEIGHT_ZERO) / (FLARE_HEIGHT_FULL - FLARE_HEIGHT_ZERO)


#: **Where the sun stands, and it is not in any file.**  ``CSun``'s two angles
#: are constants in ``Terrain.dll``, picked by whether the keyframe's name is
#: exactly ``sun``: an azimuth and a tilt from the zenith, in whole degrees.
#: See ``docs/10-sky.md``.
BODY_ANGLES = {"sun": (90.0, 30.0), "moon": (0.0, 50.0)}

#: The same three-int block's fourth field is the ``sky.wea`` slot the body
#: draws with, and it is 3 for the sun and 4 for anything else -- which is
#: ``SLOT_ROLES`` exactly.
BODY_SLOT = {"sun": 3, "moon": 4}


def body_direction(name: str) -> tuple[float, float, float]:
    """The unit direction to ``sun`` or ``moon``, in game axes, z up.

    ``CSun::Render`` builds ``Rz(azimuth) . Rx(tilt)`` every frame from the two
    constants above and nothing ever changes them, so this is fixed for the
    whole mission.  The direction is that matrix's third column.
    """
    azimuth, tilt = (math.radians(v) for v in BODY_ANGLES[name])
    return (
        math.sin(azimuth) * math.sin(tilt),
        -math.cos(azimuth) * math.sin(tilt),
        math.cos(tilt),
    )


def body_elevation(name: str) -> float:
    """How far above the horizon a body stands, in degrees."""
    return math.degrees(math.asin(body_direction(name)[2]))

#: The trailer opens with a kind word: 3 on 621 of the 656 shipped keyframes,
#: 1 on 6, and 0 on the 29 that close a section.  The hour and minute follow
#: it, one word later when the kind is 3.  Reading it that way gives a valid
#: time on every keyframe and leaves all 29 first sections sorted by time;
#: reading a fixed offset breaks on 18.
KIND_WITH_PADDING = 3


class SkyFormatError(ValueError):
    pass


@dataclass
class Keyframe:
    """The sky at one time of day."""

    hour: int
    minute: int
    #: 22 four-byte slots exactly as stored.
    slots: list[bytes]
    #: The object this keyframe belongs to -- ``sun``, ``moon``,
    #: ``env_lightning`` -- or an empty string.
    name: str
    #: Sound files the keyframe triggers.
    sounds: list[str] = field(default_factory=list)
    #: Four float32.  The third runs 0.1 at night to 5.0 at midday, so it
    #: reads as a light intensity; the first two are 2.2 and 2.0 almost
    #: everywhere.
    intensity: tuple[float, float, float, float] = (0.0, 0.0, 0.0, 0.0)
    #: The ten trailing uint32, with the time still in place.
    trailer: tuple[int, ...] = ()

    @property
    def kind(self) -> int:
        return self.trailer[0] if self.trailer else 0
    #: Which section of the file this came from.
    section: int = 0

    @property
    def minutes(self) -> int:
        """Time of day in minutes, for ordering and interpolation."""
        return self.hour * 60 + self.minute

    @property
    def markers(self) -> list[str]:
        """Every name this keyframe carries, the sound slots included."""
        return [n for n in ([self.name] + list(self.sounds)) if n]

    @property
    def weather(self) -> str | None:
        """``"rain"``, ``"lightning"``, or None if this keyframe starts neither.

        The engine turns an atmosphere object on and off from a keyframe, and
        the sun and the moon come in start/stop pairs.  Rain and lightning
        appear once in a section, so where they stop is not established; see
        ``docs/10-sky.md``.
        """
        for marker in self.markers:
            if marker == RAIN_MARKER:
                return "rain"
            if marker == LIGHTNING_MARKER:
                return "lightning"
        return None

    def colour(self, slot: int) -> tuple[int, int, int, int]:
        """One slot as ``(r, g, b, a)``.

        Stored BGRA, the same DirectDraw convention the textures use.
        """
        b, g, r, a = self.slots[slot]
        return r, g, b, a

    def number(self, slot: int) -> float:
        return struct.unpack("<f", self.slots[slot])[0]

    @property
    def sky(self) -> tuple[int, int, int, int]:
        """The first day-cycle colour.

        Which of the three colour groups is the dome and which are fog and
        ambient is not established -- this is the group that most clearly
        tracks the day, going near-black at midnight.
        """
        return self.colour(DAY_CYCLE_SLOTS[0])

    @property
    def light(self) -> float:
        return self.intensity[2]


@dataclass
class Atmosphere:
    source: Path
    version: int
    sections: int
    keyframes: list[Keyframe]
    #: The file header, as it stands, for anything not yet named.
    header: bytes = b""
    #: Texture names from the sibling ``sky.wea``.
    textures: list[str] = field(default_factory=list)

    def __len__(self) -> int:
        return len(self.keyframes)

    def at(self, hour: int, minute: int = 0) -> Keyframe | None:
        """The keyframe in force at a time of day.

        The latest keyframe at or before the time, wrapping to the last one of
        the day before midnight.  Keyframes are not stored in time order in
        every file, so they are sorted here.
        """
        if not self.keyframes:
            return None
        want = hour * 60 + minute
        ordered = sorted(self.keyframes, key=lambda k: k.minutes)
        best = ordered[-1]
        for k in ordered:
            if k.minutes <= want:
                best = k
        return best

    def texture(self, role: str) -> str | None:
        """The material named for one of the nine ``sky.wea`` roles."""
        if role not in SLOT_ROLES:
            raise KeyError(f"unknown sky role {role!r}")
        index = SLOT_ROLES.index(role)
        if index >= len(self.textures):
            return None
        return self.textures[index] or None

    def weather(self) -> dict[str, list[Keyframe]]:
        """The keyframes that start each kind of weather, by kind."""
        out: dict[str, list[Keyframe]] = {}
        for frame in self.keyframes:
            kind = frame.weather
            if kind:
                out.setdefault(kind, []).append(frame)
        return out

    def brightest(self) -> Keyframe | None:
        """The keyframe with the most light -- the middle of the day."""
        return max(self.keyframes, key=lambda k: k.light, default=None)


def _read_string(data: bytes, pos: int) -> tuple[str, int]:
    if pos + 4 > len(data):
        raise SkyFormatError(f"string length runs past the end at {pos}")
    n = struct.unpack_from("<I", data, pos)[0]
    if n > 4096 or pos + 4 + n > len(data):
        raise SkyFormatError(f"implausible string length {n} at {pos}")
    return data[pos + 4 : pos + 4 + n].decode("latin-1"), pos + 4 + n


def _read_keyframe(data: bytes, pos: int, section: int) -> tuple[Keyframe, int]:
    if pos + 88 > len(data):
        raise SkyFormatError(f"keyframe runs past the end at {pos}")
    slots = [data[pos + i * 4 : pos + i * 4 + 4] for i in range(SLOT_COUNT)]
    pos += 88

    names = []
    for _ in range(NAME_SLOTS):
        name, pos = _read_string(data, pos)
        names.append(name)

    intensity = struct.unpack_from("<4f", data, pos)
    pos += 16

    count = struct.unpack_from("<I", data, pos)[0]
    pos += 4
    if count > 64:
        raise SkyFormatError(f"implausible sound count {count} at {pos - 4}")
    sounds = []
    for _ in range(count):
        sound, pos = _read_string(data, pos)
        sounds.append(sound)

    if pos + 40 > len(data):
        raise SkyFormatError(f"keyframe trailer runs past the end at {pos}")
    trailer = struct.unpack_from("<10I", data, pos)
    pos += 40
    at = 4 if trailer[0] == KIND_WITH_PADDING else 3

    return (
        Keyframe(
            hour=trailer[at],
            minute=trailer[at + 1],
            slots=slots,
            name=next((n for n in names if n), ""),
            sounds=[s for s in sounds if s],
            intensity=intensity,
            trailer=trailer,
            section=section,
        ),
        pos,
    )


def load(path: str | Path) -> Atmosphere:
    """Parse a ``sky.ske``, picking up the sibling ``sky.wea`` when present.

    Raises SkyFormatError unless the file is consumed exactly, which is the
    same consistency check the other readers make.
    """
    path = Path(path)
    data = path.read_bytes()
    if len(data) < HEADER_SIZE:
        raise SkyFormatError(f"{path}: too short to be an atmosphere file")
    magic, version, sections, _one, count = struct.unpack_from("<5I", data, 0)
    if magic != MAGIC:
        raise SkyFormatError(f"{path}: expected magic {MAGIC:#x}, got {magic:#x}")
    if version != VERSION:
        raise SkyFormatError(f"{path}: version {version}, expected {VERSION}")

    keyframes = []
    pos = HEADER_SIZE
    for _ in range(count):
        frame, pos = _read_keyframe(data, pos, 0)
        keyframes.append(frame)

    # Later sections repeat the header's own 72-byte tail and then run to the
    # end of the file.  Their keyframe count is not in either header -- the
    # six shipped two-section files carry byte-identical section headers but
    # 27 and 20 keyframes -- so the list is read until the bytes are gone.
    section = 1
    while pos < len(data):
        pos += SECTION_HEADER_SIZE
        while pos < len(data):
            frame, pos = _read_keyframe(data, pos, section)
            keyframes.append(frame)
        section += 1

    if pos != len(data):
        raise SkyFormatError(f"{path}: parsed {pos} bytes of a {len(data)}-byte file")

    wea = path.parent / "sky.wea"
    textures = read_wea(wea.read_bytes()) if wea.exists() else []
    return Atmosphere(path, version, sections, keyframes, data[:HEADER_SIZE], textures)
