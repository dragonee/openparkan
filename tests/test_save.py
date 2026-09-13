"""Save games, on bytes this file builds itself."""

from __future__ import annotations

import struct

import pytest

from openparkan import save


def build_save(mission="missions/campaign/campaign.01/mission.01/",
               magic=save.MAGIC, version=save.VERSION, level=save.EASY,
               body=b"", length=None) -> bytes:
    raw = mission.encode("latin-1")
    out = bytearray(magic)
    out += bytes([version, level])
    out += struct.pack("<I", len(raw) if length is None else length)
    out += raw
    return bytes(out + body)


def record(archive: str, member: str, width: int = save.MEMBER_AT[0]) -> bytes:
    """The engine's two-string record: an archive field, then a member one.

    ``width`` is how much the archive name gets -- the engine writes both 32
    and 128, and a reader that knows only one silently drops the other.
    """
    return (archive.encode() + b"\0").ljust(width, b"\0") + \
           (member.encode() + b"\0").ljust(32, b"\0")


def test_a_save_reads_its_header():
    s = save.parse(build_save())
    assert s.version == save.VERSION
    assert s.level == save.EASY
    assert s.level_name == "EASY"
    assert s.mission == "missions/campaign/campaign.01/mission.01/"
    assert s.references == ()


def test_the_second_byte_is_the_difficulty():
    s = save.parse(build_save(mission="missions/single.02/", level=save.MEDIUM))
    assert s.level == save.MEDIUM
    assert s.level_name == "MEDIUM"
    assert save.parse(build_save(level=save.HARD)).level_name == "HARD"


def in_world(raw: bytes) -> bytes:
    """Bytes a scan should find, carried in a world the parse accepts."""
    world = bytes(save.WORLD_HEADER) + world_record(0x11000000, "", "land", [raw])
    return struct.pack("<I", len(world)) + world


def test_references_are_recovered_from_the_two_string_record():
    body = in_world(record("objects.rlb", "i_arm_b_05") + record("effects.rlb", "fx_smoke"))
    s = save.parse(build_save(body=body))
    assert [(r.archive, r.member) for r in s.references] == [
        ("objects.rlb", "i_arm_b_05"), ("effects.rlb", "fx_smoke"),
    ]
    assert s.members == (("objects.rlb", "i_arm_b_05"), ("effects.rlb", "fx_smoke"))


def test_repeated_members_are_listed_once():
    body = in_world(record("objects.rlb", "i_eng_b_03") * 3)
    s = save.parse(build_save(body=body))
    assert len(s.references) == 3
    assert s.members == (("objects.rlb", "i_eng_b_03"),)


def test_the_map_and_trees_are_found():
    body = in_world(b"\0DATA\\MAPS\\KM_14\\land\0" + b"MISSIONS\\SCRIPTS\\data.trf\0"
                    + b"MISSIONS\\SCRIPTS\\data.trf\0" + b"MISSIONS\\SCRIPTS\\c3m2p.trf\0")
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


def test_the_wide_record_is_found_too():
    body = in_world(record("objects.rlb", "fr_l_gener", width=save.MEMBER_AT[1]))
    s = save.parse(build_save(body=body))
    assert [(r.member, r.field) for r in s.references] == [("fr_l_gener", 128)]


def test_a_member_name_is_never_read_out_of_another_archive_name():
    """Landing 128 bytes on can fall inside a neighbouring archive name."""
    body = in_world(record("objects.rlb", "i_arm_b_05")
                    + record("objects.rlb", "i_arm_b_06")
                    + record("objects.rlb", "i_arm_b_07"))
    s = save.parse(build_save(body=body))
    assert all(r.member != "rlb" for r in s.references)
    assert {r.member for r in s.references} == {
        "i_arm_b_05", "i_arm_b_06", "i_arm_b_07"}


def name(text: str, width: int) -> bytes:
    return (text.encode() + b"\0").ljust(width, b"\0")


def world_record(ident: int, archive: str, member: str, chunks: list[bytes],
                 parent: int = 0, slot: int = -1) -> bytes:
    """One world object: the 276-byte head, the chunk table, the chunks."""
    table = struct.pack("<I", len(chunks)) + b"".join(
        struct.pack("<I", len(c)) for c in chunks) + b"".join(chunks)
    head = struct.pack("<I", ident) + name(archive, 128) + name(member, 128) \
        + struct.pack("<iiIi", parent, slot, len(table), 0)
    assert len(head) == save.RECORD_HEAD
    return head + table


def part(member: str, parent: int, attach: int, ident: int) -> bytes:
    return name("objects.rlb", 32) + name(member, 32) + struct.pack("<3i", parent, attach, ident)


def unit_chunks(position=(10.0, 20.0, 30.0)) -> list[bytes]:
    counts = bytes([1, 1, 1, 0, 0])  # parts, mesh, control; no wizard or behaviour
    parts = part("t_turret", 0, 3, 1) + part("e_gun", 1, 2, 2)
    control = struct.pack("<I4f3f", 0x10000FF0, 1.0, 0.0, 0.0, 0.0, *position) + bytes(8)
    return [counts, parts, struct.pack("<3f", 2.0, 2.0, 2.0), control]


def design_component(member: str, children: int) -> bytes:
    return name("objects.rlb", 32) + name(member, 32) + struct.pack("<Ii", 1, -1) \
        + name("Label", 32) + struct.pack("<Ii", 0, children)


def full_body(minds=(2, 1)) -> bytes:
    world = struct.pack("<II", 1, 12345) + bytes(40)
    world += world_record(0x11000000, "", "DATA\\MAPS\\X\\land", [])
    world += world_record(0x14000001, "objects.rlb", "R_L_01", unit_chunks())
    out = struct.pack("<I", len(world)) + world
    out += struct.pack("<i", len(minds))
    out += struct.pack("<i", 1) + name("1. Win", 255) + struct.pack("<ii", 1, 0)
    for count in minds:
        out += struct.pack("<i", 7) + struct.pack(f"<{count}i", *([5] + [-1] * (count - 1)))
    out += struct.pack("<i", 1) + bytes(save.EXTRA_RECORD)
    out += struct.pack("<i", 1) + struct.pack("<II", 0xF0F1, 0x1008000)
    out += design_component("R_L_01", 1) + design_component("t_turret", 0)
    out += struct.pack("<3i", 1, -1, -1) + struct.pack("<i", len(minds))
    for _ in minds:
        out += blob(bytes(save.AI_FIXED))
    return out


def test_the_whole_container_parses_given_the_minds():
    s = save.parse(build_save(body=full_body()), minds=[2, 1])
    assert s.complete
    assert s.clock == 12345
    assert [o.kind for o in s.objects] == [0x11, 0x14]
    unit = s.objects[1]
    assert unit.member == "R_L_01" and unit.serial == 1
    assert unit.counts == bytes([1, 1, 1, 0, 0])
    assert [(p.member, p.parent, p.attach, p.id) for p in unit.parts] == [
        ("t_turret", 0, 3, 1), ("e_gun", 1, 2, 2)]
    assert unit.scale == (2.0, 2.0, 2.0)
    assert unit.position == (10.0, 20.0, 30.0)
    assert unit.orientation == (1.0, 0.0, 0.0, 0.0)
    assert s.objectives == (save.Objective("1. Win", save.OBJECTIVE_DONE, 0),)
    assert s.clan_words == (7, 7)
    assert s.minds == ((5, -1), (5,))
    assert len(s.extras) == 1
    assert [c.ref.member for c in s.designs[0].components] == ["R_L_01", "t_turret"]
    assert s.tail == (1, -1, -1)
    assert [c.size for c in s.ai] == [save.AI_FIXED, save.AI_FIXED]


def test_without_the_minds_only_the_world_is_read():
    s = save.parse(build_save(body=full_body()))
    assert not s.complete
    assert len(s.objects) == 2
    assert s.objectives == ()


def test_the_wrong_mind_counts_do_not_end_on_the_last_byte():
    with pytest.raises(save.SaveFormatError):
        save.parse(build_save(body=full_body()), minds=[3, 1])


def test_a_world_record_that_lies_about_its_size_is_refused():
    body = bytearray(full_body())
    # the landscape record's "bytes that follow" word, inside the world
    at = 4 + save.WORLD_HEADER + save.RECORD_PARENT + 8
    body[at:at + 4] = struct.pack("<I", 9)
    with pytest.raises(save.SaveFormatError):
        save.parse(build_save(body=bytes(body)))


def blob(payload: bytes) -> bytes:
    """A length then that many bytes."""
    return struct.pack("<I", len(payload)) + payload
