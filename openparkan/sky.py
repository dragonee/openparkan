"""Reader for ``sky.ske`` -- the per-mission **atmosphere**.

Not a skybox.  ``Terrain.dll`` calls this an atmosphere file and names the
tool that wrote it: *"SunDll panic : Old version ske file / ReSave in
SunEditor"*.  Inside are ``CAtmosphere``, ``CAtmData``, ``CSun``,
``CreateAtmosphereObject``, "Illegal atmosphere object type", and the
settings ``AtmSkyDetail``, ``AtmStarsOn``, ``AtmCloudsOn``, ``LensFlareOn``.

What the file holds is one or more **day cycles**: lists of keyframes, each
stamped with an hour and a minute, carrying the colours and intensities the
sky takes at that time and the weather event that fires there.

The layout is read off the deserialiser (``Terrain.dll:0x100672d0``, which
calls ``0x100660c0`` per section and ``0x10066230`` per keyframe), and it
accounts for all 29 shipped files to the byte::

    int32   -1                          magic
    int32   5                           version
    int32   section count, 1 or 2
    for each section:
        int32   1                       section version
        int32   keyframe count
        time    23:59                   read, never asked for
        time    the day length          hours and minutes of real time
        keyframe x count
    time    the clock's start           section, hour and minute
    int32   0                           read, never asked for
    int32   0 or 1                      handed to the sky, which never reads it

and one keyframe is::

    int32   3                           keyframe version
    time    the keyframe's clock stamp
    int32   the event opcode            0..9, see EVENT_OPCODES
    88 bytes                            22 slots: BGRA colours, two float32
    6 x string                          the object's name in the first
    float32[4]                          sun sizes, light, weather intensity
    int32 n, n x string                 up to four effect names

A ``time`` is 32 bytes -- six ``uint32`` and eight more bytes
(``0x10086570``): the section at +0, the hour at +12, the minute at +16.  A
string is an ``int32`` length and that many bytes, no terminator.

**An earlier reading of this file was off by one keyframe.**  It took the 84
bytes before the first keyframe's slots as a 124-byte header and each
keyframe's 40-byte preamble as a *trailer* of the keyframe before it, so every
keyframe was stamped with the next one's time and opcode.  That is why no
field of the file seemed to carry the opcode: the right word had been tested,
against the wrong keyframe.

The sections are **played one after another** -- day 0, then day 1, then day
0 again -- not chosen between: ``CAtmosphere`` keeps a cycle as long as all the
sections' days together and walks a ``(section, seconds)`` position through
them (``0x10070040``, ``CAtmData::GetTimeDiffInSec``).  The clock starts at
the trailing time (``0x1006fab0``).

The sibling ``sky.wea`` names the textures, in the same format model wears
use, and **the slot index is the role**: the same nine slots in the same order
in all 29 missions.  They are material names, so they resolve through
``Material.lib`` exactly as the terrain's do.

See ``docs/10-sky.md``.
"""

from __future__ import annotations

import math
import struct
from dataclasses import dataclass, field
from pathlib import Path

from .mesh import read_wea

MAGIC = 0xFFFFFFFF
VERSION = 5
#: Magic, version and the section count.
FILE_HEADER_SIZE = 12
#: ``0x10086570``: six ``uint32`` and eight bytes.
TIME_SIZE = 32
#: A section opens with its version, its keyframe count and two times.
SECTION_VERSION = 1
SECTION_HEADER_SIZE = 4 + 4 + TIME_SIZE + TIME_SIZE
#: The file header and the first section's header: everything before the
#: first keyframe.
HEADER_SIZE = FILE_HEADER_SIZE + SECTION_HEADER_SIZE
#: The clock's start time and two ``int32`` close the file.
TRAILER_SIZE = TIME_SIZE + 4 + 4
#: The keyframe version every shipped keyframe carries.  The reader also takes
#: 2, which has no effect list, and 1, which stores twenty slots and makes
#: slots 20 and 21 from slot 19 at 0.3 (``0x10066230``); no shipped file uses
#: either.
KEYFRAME_VERSION = 3
#: The float32 0.3 a version-1 keyframe's slots 20 and 21 are scaled by.
V1_SCALE = struct.unpack("<f", struct.pack("<f", 0.3))[0]
SLOT_COUNT = 22
NAME_SLOTS = 6
#: The record filler copies at most this many effect names (``0x100692d0``).
EFFECT_SLOTS = 4
#: The first section's day length, as a file offset: its second time's hour.
DAY_LENGTH_AT = FILE_HEADER_SIZE + 8 + TIME_SIZE + 12
#: Offsets inside a ``time``.
TIME_SECTION_AT = 0
TIME_HOUR_AT = 12
TIME_MINUTE_AT = 16

#: Slots of the 88-byte block that hold a float32 rather than a colour: the
#: fog's start and end over ``FOG_SCALE`` (``Terrain.dll:0x1007bbc5``).
FLOAT_SLOTS = (5, 6)
FOG_START_SLOT = 5
FOG_END_SLOT = 6
FOG_SCALE = 700.0

#: What the sky does with the colour slots (``Terrain.dll:0x1006b2bc``).  Each
#: group of four is a compass, property k at k quarter turns of the camera's
#: heading angle: the horizon, which is ring 4 of the dome and the fog colour;
#: ring 3; ring 2.  The apex and ring 1 take one colour.
HORIZON_SLOTS = (2, 3, 1, 4)
RING3_SLOTS = (7, 10, 8, 9)
RING2_SLOTS = (11, 14, 12, 13)
APEX_SLOT = 15
#: The cloud layer's material colour (``0x1007a4de``, drawn only with
#: ``AtmCloudsOn``): property 15.
CLOUD_SLOT = 18
#: Added to every drawn material's emissive (``Terrain.dll:0x100308b8``): the
#: scene's ambient light in all but name.
SCENE_COLOUR_SLOT = 20
#: The sun object's main light: this colour times the third float
#: (``0x1006ac9a``) is the colour of the directional light ``CSun`` makes
#: (``0x1007eb9e``).  Rain and snow take their colour from it too.
SUN_LIGHT_SLOT = 19
#: The colour of the sun object's second directional light (``0x1007ed34``).
SUN_SECOND_LIGHT_SLOT = 21
#: Its alpha scales how far the flare gates brighten the main light
#: (``0x1007ea14``).
SUN_BOOST_SLOT = 17
#: Read with the keyframe and never used: slot 0 is not copied into the event
#: record at all, and slot 16 is copied and no consumer reads it
#: (``0x100692d0``, ``0x1006a970``).  Slot 0 holds heap addresses.
UNUSED_SLOTS = (0, 16)
#: The name this module first gave the horizon group.
DAY_CYCLE_SLOTS = HORIZON_SLOTS
#: Which of the four floats is what.  The sun's on-screen extents are float 1
#: across and float 2 up (``0x1007dfdf``); the third is the light; the fourth is the
#: running weather's intensity, the one value rain, snow and lightning take
#: (``0x1006ce00``).
SUN_WIDTH_FLOAT, SUN_HEIGHT_FLOAT, LIGHT_FLOAT, WEATHER_FLOAT = 0, 1, 2, 3

#: The dome (``Terrain.dll:0x100787f0``): a spherical cap this high, with this
#: cap angle, in this many rings, and ``2 ** AtmSkyDetail`` segments -- 16 at
#: the default of 4.  It is drawn at the camera, so its rim is at eye height.
DOME_HEIGHT = 10000.0
DOME_ANGLE = math.pi / 4
DOME_RINGS = 5
DOME_SEGMENTS = 16

#: Seconds in the clock day a stamp is scaled against.
CLOCK_DAY = 86400
#: What a file says when its sky does not move: a day that lasts a real day.
STATIC_DAY = (24, 0)
#: Every section's first time says 23:59; nothing asks for it.
SECTION_END = (23, 59)

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
#: The one the shipped keyframes use for "nothing happens here".
NOTHING = 7

#: One event record the handler walks: phase, type, and two words of time.
EVENT_RECORD = 20

#: What each starting object is called here, by type.
WEATHER_TYPES = {2: "rain", 3: "snow", 4: "lightning"}

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

#: The ``sky.wea`` slot each starting object draws with, as the start cases of
#: ``GetEvents`` hand it over: 3 or 4 for a body, 7 for snow, 8 for rain.
EVENT_SLOT = {2: 8, 3: 7}

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

#: The effect names a starting event carries.  ``GetEvents`` takes rain's
#: background sound and lightning's effect from the keyframe's effect list,
#: and stops with *"Rain background sound not specified"* or *"Lightning
#: effect not specified"* when the first entry is missing.
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

#: How far the two flare gates lift the sun's main light: from its colour c to
#: ``FLARE_LIGHT_BOOST * c`` (``0x1007ea14``).
FLARE_LIGHT_BOOST = 5.0


def flare_height_gate(height: float) -> float:
    """The second gate, given the height of a unit direction."""
    if height >= FLARE_HEIGHT_FULL:
        return 1.0
    if height <= FLARE_HEIGHT_ZERO:
        return 0.0
    return (height - FLARE_HEIGHT_ZERO) / (FLARE_HEIGHT_FULL - FLARE_HEIGHT_ZERO)


#: **Where the sun stands, and it is not in any file.**  ``CSun``'s two angles
#: are constants in ``Terrain.dll``, picked by whether the starting keyframe's
#: name is exactly ``sun``: an azimuth and a tilt from the zenith, in whole
#: degrees.  See ``docs/10-sky.md``.
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


def body_for(name: str) -> str:
    """Which body a starting ``SUN`` event makes: the engine tests ``== "sun"``."""
    return "sun" if name == "sun" else "moon"


class SkyFormatError(ValueError):
    pass


@dataclass(frozen=True)
class ClockTime:
    """A 32-byte time: six ``uint32`` and eight bytes.

    Only three of them are ever asked for -- the section at +0, the hour at
    +12 and the minute at +16.  The last eight bytes hold 1 and an
    uninitialised word in a section header, and zero everywhere else.
    """

    words: tuple[int, ...]

    @property
    def section(self) -> int:
        return self.words[0]

    @property
    def hour(self) -> int:
        return self.words[3]

    @property
    def minute(self) -> int:
        return self.words[4]

    @property
    def seconds(self) -> int:
        """Seconds since midnight on the 24-hour clock."""
        return self.hour * 3600 + self.minute * 60

    @classmethod
    def of(cls, hour: int, minute: int, section: int = 0) -> ClockTime:
        return cls((section, 0, 0, hour, minute, 0, 0, 0))


@dataclass
class Keyframe:
    """The sky at one time of day, and the event that fires there."""

    hour: int
    minute: int
    #: 22 four-byte slots exactly as stored.
    slots: list[bytes]
    #: The object this keyframe belongs to -- ``sun`` or ``moon`` -- or an
    #: empty string.  The first of the six name strings; the other five are
    #: empty on every shipped keyframe.
    name: str = ""
    #: The counted string list: rain's background sound or lightning's effect,
    #: as stored, empty entries included.
    effects: list[str] = field(default_factory=list)
    #: Four float32: the sun sprite's width and height, the light, and the
    #: running weather's intensity.
    intensity: tuple[float, float, float, float] = (0.0, 0.0, 0.0, 0.0)
    #: The event opcode: what ``GetEvents`` does when the clock passes here.
    opcode: int = NOTHING
    #: The keyframe version word, 3 on every shipped keyframe.
    version: int = KEYFRAME_VERSION
    #: Which section of the file this came from.
    section: int = 0
    #: The six name strings as stored.
    names: list[str] = field(default_factory=list)
    #: The keyframe's own time as stored.
    time: ClockTime | None = None

    @property
    def minutes(self) -> int:
        """Time of day in minutes, for ordering and interpolation."""
        return self.hour * 60 + self.minute

    @property
    def clock_seconds(self) -> int:
        return self.hour * 3600 + self.minute * 60

    @property
    def sounds(self) -> list[str]:
        """The non-empty effect names."""
        return [e for e in self.effects if e]

    @property
    def markers(self) -> list[str]:
        """Every name this keyframe carries, the effect list included."""
        return [n for n in ([self.name] + self.sounds) if n]

    @property
    def event(self) -> tuple[int, int] | None:
        """``(phase, type)``, or None where the opcode does nothing."""
        return EVENT_OPCODES.get(self.opcode)

    @property
    def weather(self) -> str | None:
        """``"rain"``, ``"snow"`` or ``"lightning"`` if this keyframe starts it."""
        event = self.event
        if event is None or event[0] != 0:
            return None
        return WEATHER_TYPES.get(event[1])

    @property
    def stops(self) -> str | None:
        """The object type name this keyframe stops, or None."""
        event = self.event
        if event is None or event[0] != 1:
            return None
        return OBJECT_TYPES[event[1]]

    @property
    def body(self) -> str | None:
        """``"sun"`` or ``"moon"`` if this keyframe starts a body."""
        if self.event != (0, 0):
            return None
        return body_for(self.name)

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
        """The horizon at heading zero: property 0, the fog colour there."""
        return self.colour(HORIZON_SLOTS[0])

    @property
    def horizon(self) -> list[tuple[int, int, int, int]]:
        """The horizon at heading quarters 0, 1, 2 and 3."""
        return [self.colour(s) for s in HORIZON_SLOTS]

    @property
    def rings(self) -> list[list[tuple[int, int, int, int]]]:
        """The dome's colours from the top down: apex, ring 2, ring 3, horizon."""
        return [[self.colour(APEX_SLOT)] * 4, [self.colour(s) for s in RING2_SLOTS],
                [self.colour(s) for s in RING3_SLOTS], self.horizon]

    @property
    def apex(self) -> tuple[int, int, int, int]:
        return self.colour(APEX_SLOT)

    @property
    def cloud_colour(self) -> tuple[int, int, int, int]:
        """The cloud layer's material colour."""
        return self.colour(CLOUD_SLOT)

    @property
    def scene_colour(self) -> tuple[int, int, int, int]:
        """Added to every material's emissive."""
        return self.colour(SCENE_COLOUR_SLOT)

    @property
    def sun_light(self) -> tuple[float, float, float]:
        """The main sun light's colour, 0..1 a channel times the light."""
        r, g, b, _a = self.colour(SUN_LIGHT_SLOT)
        return tuple(c / 255.0 * self.light for c in (r, g, b))

    @property
    def fog_start(self) -> float:
        return FOG_SCALE * self.number(FOG_START_SLOT)

    @property
    def fog_end(self) -> float:
        return FOG_SCALE * self.number(FOG_END_SLOT)

    def fog_colour(self, heading: float) -> tuple[int, int, int, int]:
        """The horizon at the camera's heading angle, in radians.

        The angle's quarter and its fraction through it blend the two nearest
        horizon colours (``Terrain.dll:0x10079730``), at full alpha.  The angle
        is ``atan2(m[0], m[4])`` of the camera's matrix (``0x100850f0``); where
        that points in the world is not established.
        """
        quarter = (heading / (math.pi / 2)) % 4.0
        k = int(quarter) % 4
        f = quarter - int(quarter)
        a, b = self.horizon[k], self.horizon[(k + 1) % 4]
        r, g, bl = (round(x + f * (y - x)) for x, y in zip(a[:3], b[:3], strict=True))
        return r, g, bl, 255

    @property
    def light(self) -> float:
        return self.intensity[LIGHT_FLOAT]

    @property
    def weather_intensity(self) -> float:
        return self.intensity[WEATHER_FLOAT]


def dome(segments: int = DOME_SEGMENTS, rings: int = DOME_RINGS,
         height: float = DOME_HEIGHT, angle: float = DOME_ANGLE
         ) -> list[tuple[float, float, float]]:
    """The dome's vertices around the camera: the apex, then ring by ring.

    A ring *r* vertex *j* sits at theta = r / rings x angle and phi = j x 2 pi /
    segments on a sphere of radius height / (2 sin^2(angle / 2)), dropped so
    the apex is at ``height`` and the rim at 0.
    """
    radius = height / (2.0 * math.sin(angle / 2.0) ** 2)
    out = [(0.0, 0.0, height)]
    for r in range(1, rings + 1):
        theta = r / rings * angle
        for j in range(segments):
            phi = j * 2.0 * math.pi / segments
            out.append((radius * math.sin(theta) * math.sin(phi),
                        radius * math.sin(theta) * math.cos(phi),
                        radius * math.cos(theta) + height - radius))
    return out


@dataclass
class Section:
    """One day cycle's header: its keyframe count and how long its day lasts."""

    index: int
    version: int
    count: int
    #: The first time: 23:59 in all 35 shipped sections.  Read, never used.
    end: ClockTime
    #: The second time: how long this day lasts in real time.
    day: ClockTime

    @property
    def day_seconds(self) -> int:
        """What ``CAtmData`` keeps per section (``0x1006a070``)."""
        return self.day.hour * 3600 + self.day.minute * 60


@dataclass(frozen=True)
class Event:
    """One event as ``GetEvents`` produces it."""

    section: int
    #: Seconds into the section's day, as the engine scales the clock stamp.
    seconds: int
    phase: int
    type: int
    keyframe: Keyframe

    @property
    def kind(self) -> str:
        if self.type == 0:
            return body_for(self.keyframe.name)
        return OBJECT_TYPES[self.type].lower()


@dataclass
class Atmosphere:
    source: Path
    version: int
    sections: int
    keyframes: list[Keyframe]
    #: The bytes before the first keyframe: the file header and the first
    #: section's header.
    header: bytes = b""
    #: Texture names from the sibling ``sky.wea``.
    textures: list[str] = field(default_factory=list)
    #: Every section's header, in file order.
    section_headers: list[Section] = field(default_factory=list)
    #: Where the clock starts as the mission loads: section, hour and minute.
    start: ClockTime = ClockTime.of(0, 0)
    #: The ``int32`` after it, 0 in every shipped file and never asked for.
    trailer_word: int = 0
    #: The last ``int32``: passed to the sky as its sixth parameter, which the
    #: sky stores and never reads.
    sky_flag: int = 0

    def __len__(self) -> int:
        return len(self.keyframes)

    # --- how long a day lasts -------------------------------------------
    @property
    def day(self) -> tuple[int, int]:
        """How long the first section's day lasts in real time, (hours, minutes)."""
        if self.section_headers:
            day = self.section_headers[0].day
            return day.hour, day.minute
        if len(self.header) < DAY_LENGTH_AT + 8:
            return (0, 0)
        return struct.unpack_from("<2I", self.header, DAY_LENGTH_AT)

    @property
    def day_seconds(self) -> int:
        """The same, in seconds -- what the engine keeps."""
        hours, minutes = self.day
        return hours * 3600 + minutes * 60

    def section_day_seconds(self, section: int) -> int:
        """How long one section's day lasts, in seconds."""
        if self.section_headers:
            return self.section_headers[section].day_seconds
        return self.day_seconds

    @property
    def section_count(self) -> int:
        return len(self.section_headers) or max(1, self.sections)

    @property
    def cycle_seconds(self) -> int:
        """All the sections' days end to end: the length of the whole cycle."""
        return sum(self.section_day_seconds(i) for i in range(self.section_count))

    def real_seconds(self, hour: int, minute: int, section: int = 0) -> int:
        """When in its section's day a clock stamp falls, in seconds.

        The engine's own arithmetic (``0x1006d460``): the stamp's seconds since
        midnight, times the section's day length, over a 24-hour clock, in
        integers.
        """
        return (hour * 3600 + minute * 60) * self.section_day_seconds(section) // CLOCK_DAY

    # --- the clock -------------------------------------------------------
    def between(self, a: tuple[int, int], b: tuple[int, int]) -> int:
        """Seconds from position ``a`` to position ``b``, each ``(section, s)``.

        ``CAtmData::GetTimeDiffInSec``: forward only, running out of ``a``'s
        section, through every section between, and into ``b``'s -- wrapping
        round the whole cycle when ``b`` is behind ``a``.
        """
        (sa, ta), (sb, tb) = a, b
        if sa == sb and tb >= ta:
            return tb - ta
        total = self.section_day_seconds(sa) - ta
        s = (sa + 1) % self.section_count
        while s != sb:
            total += self.section_day_seconds(s)
            s = (s + 1) % self.section_count
        return total + tb

    @property
    def start_position(self) -> tuple[int, int]:
        """Where the clock starts, as ``(section, seconds into its day)``."""
        section = self.start.section
        return section, self.real_seconds(self.start.hour, self.start.minute, section)

    @property
    def start_offset(self) -> int:
        """Seconds from the start of the cycle to the clock's start (``0x1006fab0``)."""
        return self.between((0, 0), self.start_position)

    def position(self, elapsed: float) -> tuple[int, float]:
        """The cycle position ``elapsed`` real seconds after the mission loads.

        ``CAtmosphere`` sets its epoch so the position at load is the start
        offset, then takes the time since the epoch modulo the whole cycle and
        walks it through the sections from section 0 (``0x10070040``).
        """
        cycle = self.cycle_seconds
        if cycle <= 0:
            return 0, 0.0
        t = (self.start_offset + elapsed) % cycle
        section = 0
        while t >= self.section_day_seconds(section):
            t -= self.section_day_seconds(section)
            section = (section + 1) % self.section_count
        return section, t

    def clock(self, elapsed: float) -> tuple[int, int, int]:
        """``(section, hour, minute)`` on the 24-hour clock at ``elapsed``."""
        section, t = self.position(elapsed)
        day = self.section_day_seconds(section)
        seconds = t * CLOCK_DAY / day if day else 0.0
        return section, int(seconds // 3600), int(seconds % 3600 // 60)

    # --- the keyframes ---------------------------------------------------
    def section_keyframes(self, section: int = 0) -> list[Keyframe]:
        """One section's keyframes in time order, as the engine sorts them.

        ``0x10067500`` bubble-sorts each section by ``hour * 60 + minute``
        after loading; all 35 shipped sections are already in order.
        """
        return sorted((k for k in self.keyframes if k.section == section),
                      key=lambda k: k.minutes)

    def at(self, hour: int, minute: int = 0, section: int = 0) -> Keyframe | None:
        """The keyframe in force at a time of day in one section.

        The latest keyframe at or before the time, wrapping to the last one of
        the day before the first stamp.
        """
        ordered = self.section_keyframes(section)
        if not ordered:
            return None
        want = hour * 60 + minute
        best = ordered[-1]
        for k in ordered:
            if k.minutes <= want:
                best = k
        return best

    def events(self) -> list[Event]:
        """Every event in the cycle, in order: section, then time.

        Opcodes 2 and 7 are left out, as ``GetEvents`` leaves them out.
        """
        out = []
        for section in range(self.section_count):
            for k in self.section_keyframes(section):
                event = k.event
                if event is None:
                    continue
                seconds = self.real_seconds(k.hour, k.minute, section)
                out.append(Event(section, seconds, event[0], event[1], k))
        return out

    def events_between(self, a: tuple[int, int], b: tuple[int, int]) -> list[Event]:
        """The events that fire as the clock runs from ``a`` to ``b``.

        ``0x1006d740``: a keyframe fires when its scaled time ``t`` is
        ``a <= t < b`` within a section; a span that leaves its section is
        split at the section's end, and the rest taken from the start of
        ``b``'s section.  (A span over three sections or more is taken
        differently, and no shipped file has three.)
        """
        (sa, ta), (sb, tb) = a, b
        spans = []
        if sb == sa and tb >= ta:
            spans.append((sa, ta, tb))
        else:
            spans.append((sa, ta, self.section_day_seconds(sa)))
            spans.append((sb, 0, tb))
        out = []
        for section, lo, hi in spans:
            out.extend(e for e in self.events()
                       if e.section == section and lo <= e.seconds < hi)
        return out

    def lifetime(self, start: Event) -> int | None:
        """How long a body started by ``start`` is given, in seconds.

        ``GetEvents``' start-``SUN`` case (``0x1006dcb7``) looks for the first
        stop-``SUN`` keyframe at or after the start, in its section and then
        the later ones -- it never wraps back to section 0 -- and before the
        end of the cycle, the last section's full day.  A stop stamped 24:00
        is at that end and does not count.  None when no stop is found, where
        the engine keeps whatever the previous search left.
        """
        last = self.section_count - 1
        end = (last, self.section_day_seconds(last))
        begin = (start.section, start.seconds)
        for e in self.events():
            if e.phase != 1 or e.type != 0:
                continue
            here = (e.section, e.seconds)
            if here < begin or here >= end:
                continue
            return self.between(begin, here)
        return None

    def windows(self, section: int = 0) -> list[tuple[str, Keyframe, Keyframe | None]]:
        """What runs when in one section: ``(kind, start, stop)``.

        A body or a shower runs from its start keyframe to the next keyframe
        that stops its object type; ``stop`` is None when none follows in the
        section.
        """
        out = []
        frames = self.section_keyframes(section)
        for i, k in enumerate(frames):
            event = k.event
            if event is None or event[0] != 0:
                continue
            stop = next((j for j in frames[i + 1:] if j.event == (1, event[1])), None)
            kind = body_for(k.name) if event[1] == 0 else WEATHER_TYPES[event[1]]
            out.append((kind, k, stop))
        return out

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


def _read_time(data: bytes, pos: int) -> tuple[ClockTime, int]:
    if pos + TIME_SIZE > len(data):
        raise SkyFormatError(f"time runs past the end at {pos}")
    return ClockTime(struct.unpack_from("<8I", data, pos)), pos + TIME_SIZE


def _read_keyframe(data: bytes, pos: int, section: int) -> tuple[Keyframe, int]:
    if pos + 4 + TIME_SIZE + 4 > len(data):
        raise SkyFormatError(f"keyframe runs past the end at {pos}")
    version = struct.unpack_from("<I", data, pos)[0]
    if version not in (1, 2, 3):
        raise SkyFormatError(f"keyframe version {version} at {pos}")
    time, pos = _read_time(data, pos + 4)
    opcode = struct.unpack_from("<I", data, pos)[0]
    pos += 4

    stored = SLOT_COUNT if version >= 2 else SLOT_COUNT - 2
    if pos + stored * 4 > len(data):
        raise SkyFormatError(f"keyframe slots run past the end at {pos}")
    slots = [data[pos + i * 4 : pos + i * 4 + 4] for i in range(stored)]
    pos += stored * 4
    if version == 1:
        # Slots 20 and 21 are made from slot 19's colour at 0.3 a channel,
        # rounded to nearest, with no alpha.
        b, g, r, _a = slots[19]
        made = bytes(round(c * V1_SCALE) for c in (b, g, r)) + b"\0"
        slots += [made, made]

    names = []
    for _ in range(NAME_SLOTS):
        name, pos = _read_string(data, pos)
        names.append(name)

    if pos + 16 > len(data):
        raise SkyFormatError(f"keyframe floats run past the end at {pos}")
    intensity = struct.unpack_from("<4f", data, pos)
    pos += 16

    effects = []
    if version == 3:
        count = struct.unpack_from("<I", data, pos)[0]
        pos += 4
        if count > 64:
            raise SkyFormatError(f"implausible effect count {count} at {pos - 4}")
        for _ in range(count):
            effect, pos = _read_string(data, pos)
            effects.append(effect)

    return (
        Keyframe(
            hour=time.hour,
            minute=time.minute,
            slots=slots,
            name=names[0],
            effects=effects,
            intensity=intensity,
            opcode=opcode,
            version=version,
            section=section,
            names=names,
            time=time,
        ),
        pos,
    )


def parse(data: bytes, source: Path = Path("sky.ske")) -> Atmosphere:
    """Read an atmosphere from bytes, the way ``Terrain.dll:0x100672d0`` does.

    Raises SkyFormatError unless the file is consumed exactly, which is the
    same consistency check the other readers make.
    """
    if len(data) < FILE_HEADER_SIZE:
        raise SkyFormatError(f"{source}: too short to be an atmosphere file")
    magic, version, count = struct.unpack_from("<3I", data, 0)
    if magic != MAGIC:
        raise SkyFormatError(f"{source}: expected magic {MAGIC:#x}, got {magic:#x}")
    if version != VERSION:
        raise SkyFormatError(f"{source}: version {version}, expected {VERSION}")
    if count > 16:
        raise SkyFormatError(f"{source}: implausible section count {count}")

    pos = FILE_HEADER_SIZE
    sections: list[Section] = []
    keyframes: list[Keyframe] = []
    for index in range(count):
        if pos + 8 > len(data):
            raise SkyFormatError(f"{source}: section {index} runs past the end")
        section_version, frames = struct.unpack_from("<2I", data, pos)
        if section_version != SECTION_VERSION:
            raise SkyFormatError(
                f"{source}: section {index} version {section_version} at {pos}")
        end, pos = _read_time(data, pos + 8)
        day, pos = _read_time(data, pos)
        sections.append(Section(index, section_version, frames, end, day))
        for _ in range(frames):
            frame, pos = _read_keyframe(data, pos, index)
            keyframes.append(frame)

    if pos + TRAILER_SIZE != len(data):
        raise SkyFormatError(
            f"{source}: {len(data) - pos} bytes after the keyframes, "
            f"expected {TRAILER_SIZE}")
    start, pos = _read_time(data, pos)
    trailer_word, sky_flag = struct.unpack_from("<2I", data, pos)

    return Atmosphere(
        source, version, count, keyframes, bytes(data[:HEADER_SIZE]),
        section_headers=sections, start=start, trailer_word=trailer_word,
        sky_flag=sky_flag,
    )


def load(path: str | Path) -> Atmosphere:
    """Parse a ``sky.ske``, picking up the sibling ``sky.wea`` when present."""
    path = Path(path)
    atmosphere = parse(path.read_bytes(), path)
    wea = path.parent / "sky.wea"
    if wea.exists():
        atmosphere.textures = read_wea(wea.read_bytes())
    return atmosphere
