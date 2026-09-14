"""The golden dumps of a ``.cfg``, a string table and a mission's progression resources."""

from __future__ import annotations

import pytest

from openparkan import dump, gamedir, resources
from tests.conftest import build_nres
from tests.test_resources import build_pe

MISSION_CFG = """\
object briefing_sounds
 desc = "resource"
 library = "voices.lib"
 libtype = "multi"
 type = 4
 T01_T01 = "t01_t01.wav"
end

object tutorial_voices
 desc = "resource"
 library = "voices.lib"
 libtype = "multi"
 type = 4
 T01_I01 = "T01_I01.wav"
 T01_T01 = "shadowed.wav"
end

object ambient_music_loop
 desc = "resource"
 library = "sounds.lib"
 libtype = "multi"
 type = 5
 THEME = "theme.wav"
end

object ambient_music_variation
 desc = "resource"
 library = "sounds.lib"
 libtype = "multi"
 type = 4
 DAY_VARIATION1 = "bird.wav"
 DAY_VARIATION3 = "unreached.wav"
end

object primary_objectives
 objective1 = "1. Destroy the targets"
end

object bonus_objectives
 extra = "Keep the bridge"
end
"""

MESSAGES_CFG = """\
object message1
 message_index = 0
 text_resource = "T01_T01"
 voice_resource = "T01_T01"
end

object message12
 message_index = 11
 text_resource = "T01_I01"
 voice_resource = "t01_i01"
 info_system = true
end

object message13
 message_index = 12
 text_resource = "NOWHERE"
 voice_resource = "NOWHERE"
end
"""


def write(path, text):
    path.write_bytes(text.replace("\n", "\r\n").encode("latin-1"))
    return path


def test_bound_takes_the_first_descriptor_that_binds_a_name(tmp_path):
    found = resources.descriptors(write(tmp_path / "mission.cfg", MISSION_CFG))
    d, member = resources.bound(found, "t01_t01")
    assert (d.role, member) == ("briefing_sounds", "t01_t01.wav")
    assert resources.bound(found, "nothing") is None


def test_a_cfg_dumps_its_blocks_descriptors_objectives_and_messages(tmp_path):
    out = dump.cfg_file(write(tmp_path / "mission.cfg", MISSION_CFG))
    assert [o["name"] for o in out["objects"]][:2] == ["briefing_sounds", "tutorial_voices"]
    assert out["descriptors"][0]["bindings"] == [["T01_T01", "t01_t01.wav"]]
    assert out["descriptors"][2]["type"] == 5 and not out["descriptors"][2]["numbered"]
    assert out["objectives"] == [{"text": "1. Destroy the targets", "exempt": False},
                                 {"text": "Keep the bridge", "exempt": True}]
    assert {m["index"] for m in out["messages"]} == {-1}


def test_a_string_table_dumps_in_id_order(tmp_path):
    path = tmp_path / "TextRes.dll"
    path.write_bytes(build_pe({2: ["seventeen"], 1: ["", "one"]}))
    assert dump.pe_strings(path)["strings"] == [[1, "one"], [16, "seventeen"]]


@pytest.fixture
def install(tmp_path):
    for marker in gamedir.MARKERS:
        (tmp_path / marker).mkdir(exist_ok=True)
    write(tmp_path / "DATA" / "TextRes.cfg",
          'object text_resources\n desc = "resource"\n library = "data\\\\TextRes.dll"\n'
          ' libtype = "multi"\n type = 6\n T01_T01 = 8\n T01_I01 = 9\nend\n')
    (tmp_path / "DATA" / "TextRes.dll").write_bytes(build_pe({1: [""] * 8 + ["Tara.", "Welcome."]}))
    wave = b"RIFF" + b"\0" * 40
    (tmp_path / "voices.lib").write_bytes(
        build_nres([("WAVE", "t01_t01.wav", wave), ("WAVE", "T01_I01.wav", wave)]))
    (tmp_path / "sounds.lib").write_bytes(build_nres([("WAVE", "theme.wav", wave)]))
    mission = tmp_path / "MISSIONS" / "Mission.01"
    mission.mkdir(parents=True)
    write(mission / "mission.cfg", MISSION_CFG)
    write(mission / "messages.cfg", MESSAGES_CFG)
    return mission


def test_a_missions_progression_resolves_against_the_install_around_it(install):
    out = dump.progression(install)
    assert [o["exempt"] for o in out["objectives"]] == [False, True]
    first, welcome, missing = out["messages"]
    assert first["text"] == "Tara." and first["voice"] == {
        "library": "voices.lib", "member": "t01_t01.wav", "found": True}
    assert welcome["text"] == "Welcome." and welcome["info_system"]
    assert welcome["voice"]["member"] == "T01_I01.wav" and welcome["voice"]["found"]
    assert missing["text"] is None and missing["voice"] is None
    ambient = out["ambient"]
    assert ambient["theme"] == {"library": "sounds.lib", "member": "theme.wav", "found": True}
    assert [s["member"] for s in ambient["day"]] == ["bird.wav"]
    assert not ambient["day"][0]["found"] and ambient["default"] == ambient["night"] == []
