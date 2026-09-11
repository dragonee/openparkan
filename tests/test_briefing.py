"""The briefing flythrough, on text this file writes itself."""

from __future__ import annotations

from openparkan import briefing

BRIEFING = """\
object WayPoint0
\tCameraX = 767.965
\tCameraY = 164.519
\tCameraZ = 118.899
\tTargetX = 967.457
\tTargetY = 610.348
\tTargetZ = 11.945
\tEdgeType = "spline"
\tWaitType = "continuous"
\tEdgeTime = 3.70
\tWaypointTime = 1.500
\tRotateTime = 0.000
\tFadeTime = 0.000
\tZoomTime = 0.000
\tWaitForText = false
\tWaitForSound = true
\tWaitForTime = true
\tWaitForClick = false
\tTextResID = "T01_T01"
\tSoundResID = "T01_T01"
\tNoisePercent = 0
\tFadePercent = 100
\tZoomOn = false
\tNightVisionOn = true
\tLoopIndex = -1
end

object WayPoint1
\tCameraX = 1.0
\tCameraY = 2.0
\tCameraZ = 3.0
\tTargetX = 4.0
\tTargetY = 5.0
\tTargetZ = 6.0
\tEdgeType = "jump"
\tWaitType = "stay"
\tEdgeTime = 2.0
\tWaypointTime = 0.0
\tRotateTime = 0.0
\tFadeTime = 0.0
\tZoomTime = 0.0
\tWaitForText = true
\tWaitForSound = false
\tWaitForTime = false
\tWaitForClick = true
\tTextResID = ""
\tSoundResID = ""
\tNoisePercent = 50
\tFadePercent = 0
\tZoomOn = true
\tNightVisionOn = false
\tLoopIndex = -1
end
"""

MESSAGES = """\
object  message1
  message_index   = 0
  text_resource   = "T01_T01"
  voice_resource  = "T01_T01"
end

object  message2
  message_index   = 100
  text_resource   = "T01_T02"
  voice_resource  = "T01_T02"
end
"""


def write(tmp_path, name, text):
    p = tmp_path / name
    p.write_bytes(text.replace("\n", "\r\n").encode("latin-1"))
    return p


def test_a_waypoint_keeps_every_field(tmp_path):
    first, second = briefing.waypoints(write(tmp_path, briefing.BRIEFING, BRIEFING))
    assert first.name == "WayPoint0"
    assert first.camera == (767.965, 164.519, 118.899)
    assert first.target == (967.457, 610.348, 11.945)
    assert first.edge == "spline" and first.wait == "continuous"
    assert first.edge_time == 3.7 and first.dwell == 1.5
    assert first.text_id == "T01_T01" and first.sound_id == "T01_T01"
    assert second.edge == "jump" and second.wait == "stay"


def test_the_flags_are_lowercase_words(tmp_path):
    first, second = briefing.waypoints(write(tmp_path, briefing.BRIEFING, BRIEFING))
    assert first.wait_for_sound and first.wait_for_time
    assert not first.wait_for_text and not first.wait_for_click
    assert first.night_vision and not first.zoom
    assert second.wait_for_text and second.wait_for_click and second.zoom


def test_seconds_is_travel_plus_dwell(tmp_path):
    first, second = briefing.waypoints(write(tmp_path, briefing.BRIEFING, BRIEFING))
    assert first.seconds == 3.7 + 1.5
    assert second.seconds == 2.0


def test_a_waypoint_without_text_or_voice_is_silent(tmp_path):
    first, second = briefing.waypoints(write(tmp_path, briefing.BRIEFING, BRIEFING))
    assert first.speaks and not second.speaks


def test_the_vocabulary_covers_what_the_file_says(tmp_path):
    stops = briefing.waypoints(write(tmp_path, briefing.BRIEFING, BRIEFING))
    assert all(w.edge in briefing.EDGES and w.wait in briefing.WAITS for w in stops)
    assert all(w.loop == briefing.NO_LOOP for w in stops)


def test_message_index_is_an_id_not_a_position(tmp_path):
    lines = briefing.messages(write(tmp_path, briefing.MESSAGES, MESSAGES))
    assert [m.index for m in lines] == [0, 100]
    assert [m.text_id for m in lines] == ["T01_T01", "T01_T02"]
    assert lines[1].voice_id == "T01_T02"


def test_briefings_are_found_by_name(tmp_path):
    (tmp_path / "Mission.01").mkdir()
    write(tmp_path, f"Mission.01/{briefing.BRIEFING}", BRIEFING)
    assert [p.parent.name for p in briefing.briefings(tmp_path)] == ["Mission.01"]
