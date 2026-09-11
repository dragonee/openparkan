"""Save games, on bytes this file builds itself."""

from __future__ import annotations

import struct

import pytest

from openparkan import save


def build_save(mission="missions/campaign/campaign.01/mission.01/",
               magic=save.MAGIC, version=save.VERSION, kind=save.CAMPAIGN,
               body=b"", length=None) -> bytes:
    raw = mission.encode("latin-1")
    out = bytearray(magic)
    out += bytes([version, kind])
    out += struct.pack("<I", len(raw) if length is None else length)
    out += raw
    return bytes(out + body)


def record(archive: str, member: str) -> bytes:
    """The engine's two-string record: a 32-byte field, then another."""
    return (archive.encode() + b"\0").ljust(save.MEMBER_AT, b"\0") + \
           (member.encode() + b"\0").ljust(save.MEMBER_AT, b"\0")


def test_a_save_reads_its_header():
    s = save.parse(build_save())
    assert s.version == save.VERSION
    assert s.kind == save.CAMPAIGN
    assert s.campaign
    assert s.mission == "missions/campaign/campaign.01/mission.01/"
    assert s.references == ()


def test_a_single_mission_save_is_marked():
    s = save.parse(build_save(mission="missions/single.02/", kind=save.SINGLE))
    assert not s.campaign
    assert s.kind == save.SINGLE


def test_references_are_recovered_from_the_two_string_record():
    body = record("objects.rlb", "i_arm_b_05") + record("effects.rlb", "fx_smoke")
    s = save.parse(build_save(body=body))
    assert [(r.archive, r.member) for r in s.references] == [
        ("objects.rlb", "i_arm_b_05"), ("effects.rlb", "fx_smoke"),
    ]
    assert s.members == (("objects.rlb", "i_arm_b_05"), ("effects.rlb", "fx_smoke"))


def test_repeated_members_are_listed_once():
    body = record("objects.rlb", "i_eng_b_03") * 3
    s = save.parse(build_save(body=body))
    assert len(s.references) == 3
    assert s.members == (("objects.rlb", "i_eng_b_03"),)


def test_the_map_and_trees_are_found():
    body = b"\0DATA\\MAPS\\KM_14\\land\0" + b"MISSIONS\\SCRIPTS\\data.trf\0" \
           + b"MISSIONS\\SCRIPTS\\data.trf\0" + b"MISSIONS\\SCRIPTS\\c3m2p.trf\0"
    s = save.parse(build_save(body=body))
    assert s.map == "KM_14"
    assert s.trees == ("data.trf", "c3m2p.trf")


def test_a_save_with_neither_map_nor_tree_is_still_read():
    s = save.parse(build_save())
    assert s.map == ""
    assert s.trees == ()


def test_a_bad_magic_is_refused():
    with pytest.raises(save.SaveFormatError, match="not a save"):
        save.parse(build_save(magic=b"SAVE"))


def test_a_bad_version_is_refused():
    with pytest.raises(save.SaveFormatError, match="version 2"):
        save.parse(build_save(version=2))


def test_a_path_running_past_the_file_is_refused():
    with pytest.raises(save.SaveFormatError, match="mission path length"):
        save.parse(build_save(length=9999))


def test_a_truncated_header_is_refused():
    with pytest.raises(save.SaveFormatError, match="too short"):
        save.parse(save.MAGIC + b"\x01\x00")


SLOTS = """\
#############################################
# 14/6/2026 21:38
#############################################

OBJECT saveslots
\tquantity\t\t=\t7
END

OBJECT slot1
\tname\t\t=\t"my game"
\tfilename\t\t=\t"slot1.sav"
\tempty\t\t=\tFALSE
END

OBJECT slot7
\tname\t\t=\t"empty"
\tfilename\t\t=\t"slot7.sav"
\tempty\t\t=\tTRUE
END
"""


def test_the_slot_index_reads(tmp_path):
    d = tmp_path / save.DIRECTORY
    d.mkdir()
    (d / save.SLOTS).write_text(SLOTS.replace("\n", "\r\n"), "latin-1")
    got = save.slots(tmp_path)
    # the 'saveslots' object declares a quantity and has no filename, so it is
    # not a slot and must not be counted as one
    assert [s.slot for s in got] == ["slot1", "slot7"]
    assert got[0].name == "my game"
    assert got[0].filename == "slot1.sav"
    assert not got[0].empty
    assert got[1].empty
