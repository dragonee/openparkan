"""The engine's configuration files, on text this file writes itself."""

from __future__ import annotations

from openparkan import settings

COMP = """\
// Current constants for component identification:
//
//     CID_CLASSIC_LANDSCAPE   0
//     CID_SHADER              6
//
// Format:
//     CID  DLL-Name  Function_Name   Comments...
//

0  terrain.dll  LoadLandscape      // comments...
6  terrain.dll  CreateShader
"""

SWITCHES = """\
//***********************************************
//Behaviour initialization and configuration file
//***********************************************

//Logging
LogFile = Behavior.log
SaveLog = 0

//Debugging
LockBehaviour = 0   // trailing comment
"""

SECTIONS = """\
DANGLING=1
[CS]
DISPLAY_WIDTH=1920
FORCE_CD_SOUND=".\\MUSIC\\"

[LEVEL_RATIO]
EASY=0.5
HARD=1.0
"""

DISPATCHER = """\
[COMPLETE]
missions_single_02_=1
missions_campaign_campaign_00_mission_01_=1
"""


def write(tmp_path, name, text):
    p = tmp_path / name
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_bytes(text.replace("\n", "\r\n").encode("latin-1"))
    return p


def test_a_row_is_an_int_and_two_words(tmp_path):
    rows = settings.registry(write(tmp_path, settings.COMPONENTS_FILE, COMP))
    assert [r.cid for r in rows] == [0, 6]
    assert rows[0].dll == "terrain.dll" and rows[0].function == "LoadLandscape"
    assert rows[0].comment == "// comments..."
    assert rows[1].comment == ""


def test_the_header_names_the_ids(tmp_path):
    path = write(tmp_path, settings.COMPONENTS_FILE, COMP)
    assert settings.component_names(path) == {0: "CID_CLASSIC_LANDSCAPE",
                                              6: "CID_SHADER"}
    assert [r.name for r in settings.registry(path)] == ["CID_CLASSIC_LANDSCAPE",
                                                        "CID_SHADER"]


def test_comments_and_short_lines_are_not_rows(tmp_path):
    text = COMP + "//9  ghost.dll  Nope\nx\n\n1  a.dll  B\n"
    rows = settings.registry(write(tmp_path, settings.COMPONENTS_FILE, text))
    assert [r.cid for r in rows] == [0, 6, 1]


def test_a_line_the_scanf_would_reject_is_dropped(tmp_path):
    text = COMP + "seven  misload.dll  LoadResearch\n8  onlytwo.dll\n"
    rows = settings.registry(write(tmp_path, settings.COMPONENTS_FILE, text))
    assert [r.cid for r in rows] == [0, 6]


def test_switches_drop_their_comments(tmp_path):
    table = settings.switches(write(tmp_path, settings.BEHAVIOUR_FILE, SWITCHES))
    assert table == {"LogFile": "Behavior.log", "SaveLog": "0",
                     "LockBehaviour": "0"}


def test_sections_start_at_the_first_head(tmp_path):
    blocks = settings.sections(write(tmp_path, settings.DISPLAY_FILE, SECTIONS))
    assert set(blocks) == {"CS", "LEVEL_RATIO"}
    assert blocks["CS"]["DISPLAY_WIDTH"] == "1920"
    assert blocks["CS"]["FORCE_CD_SOUND"] == '".\\MUSIC\\"'
    assert blocks["LEVEL_RATIO"] == {"EASY": "0.5", "HARD": "1.0"}


def test_a_dispatcher_key_is_a_flattened_path(tmp_path):
    directory = tmp_path / "MISSIONS" / "CAMPAIGN" / "CAMPAIGN.00" / "Mission.01"
    directory.mkdir(parents=True)
    assert settings.dispatcher_key(tmp_path, directory) == \
        "missions_campaign_campaign_00_mission_01_"


def test_completed_reads_the_one_section(tmp_path):
    write(tmp_path, "MISSIONS/dispatcher.ini", DISPATCHER)
    done = settings.completed(tmp_path)
    assert set(done) == {"missions_single_02_",
                         "missions_campaign_campaign_00_mission_01_"}
    assert set(done.values()) == {settings.DONE}


def test_no_dispatcher_is_not_an_error(tmp_path):
    assert settings.completed(tmp_path) == {}
