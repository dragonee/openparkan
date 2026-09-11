"""The briefing flythrough, ``briefing.cfg``, and the messages beside it.

A campaign mission opens on a camera flying over the map while a voice talks.
That sequence is a plain-text list of waypoints in the mission's own
directory, written in the same ``object … end`` syntax as ``mission.cfg``::

    object WayPoint0
        CameraX = 767.965        where the camera is
        CameraY = 164.519
        CameraZ = 118.899
        TargetX = 967.457        where it looks
        TargetY = 610.348
        TargetZ = 11.945
        EdgeType = "spline"      how it travels
        WaitType = "continuous"  whether it stops here
        EdgeTime = 3.70
        WaypointTime = 0.000
        RotateTime = 0.000
        FadeTime = 0.000
        ZoomTime = 0.000
        WaitForText = false      what holds the sequence here
        WaitForSound = true
        WaitForTime = true
        WaitForClick = false
        TextResID = "T01_T01"    the subtitle
        SoundResID = "T01_T01"   the voice
        NoisePercent = 0         and the picture
        FadePercent = 100
        ZoomOn = false
        NightVisionOn = false
        LoopIndex = -1
    end

**All 20 campaign missions carry one and no other mission does** -- the same
20 whose ``mission.cfg`` declares a ``briefing_sounds`` resource descriptor
(`20-resources.md`), which is where ``SoundResID`` resolves.  Between them
they hold **378 waypoints, and every one carries all 24 fields**.

The coordinates are world coordinates, in the same frame as everything the
mission places: **all 378 camera positions and all 378 targets fall inside
their own map's XY extent**, the closest 31 units from an edge, and **all 378
cameras sit above the terrain** -- a median of 17.3 units of clearance and
never less than 1.0.  A wrong frame or scale would scatter them.

``EdgeType`` is ``linear`` (243), ``spline`` (89) or ``jump`` (46);
``WaitType`` is ``continuous`` (362), ``flyaround`` (11) or ``stay`` (5).
``LoopIndex`` is -1 on all 378 -- a field the engine reads and the authors
never used.

``messages.cfg`` sits beside it in 16 of the 20, and is the in-mission
dialogue rather than the briefing::

    object  message1
      message_index   = 0
      text_resource   = "T01_T01"
      voice_resource  = "T01_T01"
    end

**99 messages, and every text resource resolves.**  ``message_index`` is an
id, not a position: three files skip a number and one jumps to 100.  Those 16
are exactly the missions whose ``mission.cfg`` declares ``tutorial_voices``.

Everything above is re-derived by ``uv run openparkan verify``.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from .mission import load_cfg

#: The two files, in a mission's own directory.
BRIEFING = "briefing.cfg"
MESSAGES = "messages.cfg"

#: The descriptor roles the two draw their voices from, in ``mission.cfg``.
BRIEFING_ROLE = "briefing_sounds"
MESSAGE_ROLE = "tutorial_voices"

#: How the camera travels into a waypoint.
EDGES = ("linear", "spline", "jump")

#: Whether it stops there.
WAITS = ("continuous", "stay", "flyaround")

#: Every field a waypoint carries.  All 378 carry all 24.
FIELDS = (
    "CameraX", "CameraY", "CameraZ", "TargetX", "TargetY", "TargetZ",
    "EdgeType", "WaitType", "EdgeTime", "WaypointTime", "RotateTime",
    "FadeTime", "ZoomTime", "WaitForText", "WaitForSound", "WaitForTime",
    "WaitForClick", "TextResID", "SoundResID", "NoisePercent", "FadePercent",
    "ZoomOn", "NightVisionOn", "LoopIndex",
)

#: What ``LoopIndex`` holds in every shipped waypoint.
NO_LOOP = -1

#: What the shipped installation carries.
BRIEFINGS = 20
WAYPOINTS = 378
MESSAGE_FILES = 16
MESSAGES_TOTAL = 99


def _flag(text: str) -> bool:
    return text.strip().lower() == "true"


def _number(text: str) -> float:
    try:
        return float(text)
    except ValueError:
        return 0.0


@dataclass(frozen=True)
class Waypoint:
    """One stop on the flythrough."""

    name: str
    camera: tuple[float, float, float]
    target: tuple[float, float, float]
    edge: str
    wait: str
    edge_time: float
    dwell: float
    rotate_time: float
    fade_time: float
    zoom_time: float
    wait_for_text: bool
    wait_for_sound: bool
    wait_for_time: bool
    wait_for_click: bool
    text_id: str
    sound_id: str
    noise: float
    fade: float
    zoom: bool
    night_vision: bool
    loop: int

    @property
    def seconds(self) -> float:
        """Travel plus dwell.  A floor, not a duration: a waypoint that waits
        for its voice runs as long as the voice does, which is not in the
        file."""
        return self.edge_time + self.dwell

    @property
    def speaks(self) -> bool:
        return bool(self.text_id or self.sound_id)


@dataclass(frozen=True)
class Message:
    """One line of in-mission dialogue."""

    name: str
    index: int
    text_id: str
    voice_id: str


def waypoints(path: str | Path) -> list[Waypoint]:
    """Read a ``briefing.cfg``, in file order."""
    out = []
    for name, p in load_cfg(path).items():
        out.append(Waypoint(
            name=name,
            camera=(_number(p.get("CameraX", "")), _number(p.get("CameraY", "")),
                    _number(p.get("CameraZ", ""))),
            target=(_number(p.get("TargetX", "")), _number(p.get("TargetY", "")),
                    _number(p.get("TargetZ", ""))),
            edge=p.get("EdgeType", ""),
            wait=p.get("WaitType", ""),
            edge_time=_number(p.get("EdgeTime", "")),
            dwell=_number(p.get("WaypointTime", "")),
            rotate_time=_number(p.get("RotateTime", "")),
            fade_time=_number(p.get("FadeTime", "")),
            zoom_time=_number(p.get("ZoomTime", "")),
            wait_for_text=_flag(p.get("WaitForText", "")),
            wait_for_sound=_flag(p.get("WaitForSound", "")),
            wait_for_time=_flag(p.get("WaitForTime", "")),
            wait_for_click=_flag(p.get("WaitForClick", "")),
            text_id=p.get("TextResID", ""),
            sound_id=p.get("SoundResID", ""),
            noise=_number(p.get("NoisePercent", "")),
            fade=_number(p.get("FadePercent", "")),
            zoom=_flag(p.get("ZoomOn", "")),
            night_vision=_flag(p.get("NightVisionOn", "")),
            loop=int(_number(p.get("LoopIndex", str(NO_LOOP)))),
        ))
    return out


def messages(path: str | Path) -> list[Message]:
    """Read a ``messages.cfg``, in file order."""
    out = []
    for name, p in load_cfg(path).items():
        out.append(Message(
            name=name,
            index=int(_number(p.get("message_index", "-1"))),
            text_id=p.get("text_resource", ""),
            voice_id=p.get("voice_resource", ""),
        ))
    return out


def briefings(game: str | Path) -> list[Path]:
    """Every mission directory with a briefing, in path order."""
    return sorted(Path(game).rglob(BRIEFING))
