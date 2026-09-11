"""Resource descriptors, and a PE string table this file builds itself."""

from __future__ import annotations

import struct

import pytest

from openparkan import resources

CFG = """\
# a descriptor and a plain object beside it
object\ttext_resources
 desc\t\t= "resource"
 library\t= "data\\TextRes.dll"
 libtype\t= "multi"
 type\t\t= 6
 T01_T01\t= 8
 T01_T02\t= 9
end

object briefing_sounds
 desc  = "resource"
 library = "voices.lib"
 libtype = "multi"
 type  = 4
 T01_T01 = t01_t01.wav
end

object primary_objectives
 objective1 = "1. Stay alive"
end
"""


def write(tmp_path, name, text):
    p = tmp_path / name
    p.write_bytes(text.replace("\n", "\r\n").encode("latin-1"))
    return p


def test_only_resource_objects_are_descriptors(tmp_path):
    found = resources.descriptors(write(tmp_path, "mission.cfg", CFG))
    assert [d.role for d in found] == ["text_resources", "briefing_sounds"]


def test_the_four_meta_keys_are_not_bindings(tmp_path):
    text, sounds = resources.descriptors(write(tmp_path, "mission.cfg", CFG))
    assert text.bindings == {"T01_T01": "8", "T01_T02": "9"}
    assert text.type == 6 and text.libtype == "multi"
    assert text.kind == "text" and sounds.kind == "sounds"


def test_bindings_are_numbered_or_named(tmp_path):
    text, sounds = resources.descriptors(write(tmp_path, "mission.cfg", CFG))
    assert text.numbered and not sounds.numbered
    assert len(text) == 2
    assert sounds.get("t01_t01") == "t01_t01.wav"
    assert sounds.get("nothing") is None


def test_locate_walks_a_windows_path(tmp_path):
    (tmp_path / "UI").mkdir()
    (tmp_path / "UI" / "ui.lib").write_bytes(b"")
    assert resources.locate(tmp_path, "ui\\\\ui.lib").name == "ui.lib"
    assert resources.locate(tmp_path, "ui\\UI.LIB").name == "ui.lib"
    assert resources.locate(tmp_path, "ui\\missing.lib") is None
    assert resources.locate(tmp_path, "") is None


# --------------------------------------------------------------------------
# A PE image, assembled here so the string reader is tested on bytes whose
# every field this file chose.
# --------------------------------------------------------------------------

SECTION_RVA = 0x1000


def build_pe(blocks: dict[int, list[str]], language: int = resources.LANGUAGE,
             type_id: int = resources.RT_STRING) -> bytes:
    """A minimal PE32 whose only content is one resource directory."""
    bodies = {}
    for block, texts in blocks.items():
        body = b""
        for text in texts:
            body += struct.pack("<H", len(text)) + text.encode("utf-16-le")
        bodies[block] = body

    # Layout: root directory, one type directory, one leaf directory per
    # block, then the data entries, then the bodies.
    root = 16 + 8
    type_dir = root
    name_dirs = type_dir + 16 + 8 * len(bodies)
    data_entries = name_dirs + len(bodies) * (16 + 8)
    payload = data_entries + len(bodies) * 16

    out = bytearray()
    out += struct.pack("<IIHH", 0, 0, 0, 0) + struct.pack("<HH", 0, 1)
    out += struct.pack("<II", type_id, 0x8000_0000 | type_dir)
    out += struct.pack("<IIHH", 0, 0, 0, 0) + struct.pack("<HH", 0, len(bodies))
    off = name_dirs
    for block in sorted(bodies):
        out += struct.pack("<II", block, 0x8000_0000 | off)
        off += 16 + 8
    off = data_entries
    for _ in sorted(bodies):
        out += struct.pack("<IIHH", 0, 0, 0, 0) + struct.pack("<HH", 0, 1)
        out += struct.pack("<II", language, off)
        off += 16
    body_at = payload
    for block in sorted(bodies):
        out += struct.pack("<IIII", SECTION_RVA + body_at, len(bodies[block]), 0, 0)
        body_at += len(bodies[block])
    for block in sorted(bodies):
        out += bodies[block]
    section = bytes(out)

    headers = bytearray(0x200)
    headers[:2] = b"MZ"
    struct.pack_into("<I", headers, 0x3C, 0x80)
    struct.pack_into("<4sHHIIIHH", headers, 0x80, b"PE\0\0", 0x14C, 1, 0, 0, 0, 224, 0)
    optional = 0x80 + 24
    struct.pack_into("<H", headers, optional, 0x10B)
    struct.pack_into("<I", headers, optional + 92, 16)
    struct.pack_into("<II", headers, optional + 92 + 4 + 2 * 8,
                     SECTION_RVA, len(section))
    sections = optional + 224
    struct.pack_into("<8sIIIII", headers, sections, b".rsrc\0\0\0",
                     len(section), SECTION_RVA, len(section), 0x200, 0)
    return bytes(headers) + section


def test_a_block_holds_sixteen_ids():
    data = build_pe({1: ["first", "", "third"]})
    assert resources.strings(data) == {0: "first", 2: "third"}


def test_ids_are_numbered_from_the_block():
    data = build_pe({2: ["seventeen"], 13: ["one hundred and ninety three"]})
    table = resources.strings(data)
    assert table[16] == "seventeen"
    assert table[192] == "one hundred and ninety three"


def test_a_language_can_be_asked_for():
    data = build_pe({1: ["english"]}, language=1033)
    assert resources.strings(data, language=1033) == {0: "english"}
    assert resources.strings(data, language=1049) == {}


def test_another_resource_type_is_not_a_string():
    assert resources.strings(build_pe({1: ["icon"]}, type_id=3)) == {}


def test_not_a_pe():
    with pytest.raises(resources.ResourceFormatError):
        resources.strings(b"NRes" + bytes(60))


def test_text_resources_join_the_index_to_the_table(tmp_path):
    (tmp_path / "DATA").mkdir()
    write(tmp_path, "DATA/TextRes.cfg", CFG)
    (tmp_path / "DATA" / "TextRes.dll").write_bytes(
        build_pe({1: ["", "", "", "", "", "", "", "", "eight", "nine"]}))
    texts = resources.TextResources.open(tmp_path)
    assert len(texts) == 2
    assert texts.get("T01_T01") == "eight"
    assert texts.get("T01_T02") == "nine"
    assert texts.get("T01_T99") is None
