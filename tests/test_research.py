"""The .trf research tree, on an archive this file builds itself."""

from __future__ import annotations

import struct

import pytest

from openparkan import research
from tests.conftest import build_nres


def build_trf(items, edges=True, parts=None) -> bytes:
    """``items`` is a list of ``(name, code, category, values, requires)``.

    ``parts`` is an optional list of ``(part_id, item_index)`` for TRFB/TRF6.
    """
    n = len(items)
    names, codes = bytearray(), bytearray()
    name_at, code_at = [], []
    for name, code, *_ in items:
        name_at.append(len(names))
        names += name.encode() + b"\0"
        code_at.append(len(codes))
        codes += code.encode() + b"\0"

    requires = [tuple(i[4]) for i in items]
    unlocks = [tuple(j for j, r in enumerate(requires) if i in r) for i in range(n)]

    trf6, trfb = bytearray(), bytearray()
    part_index = [0] * n
    for entry, (pid, item) in enumerate(parts or []):
        at = len(trf6)
        trf6 += pid.encode() + b"\0"
        trfb += struct.pack("<HH", at, item)
        if item < n:
            part_index[item] = entry

    trf0 = bytearray()
    for i, (_, _, _, values, _) in enumerate(items):
        trf0 += struct.pack("<4f4iH6B", *values, code_at[i], name_at[i], i, 0,
                            part_index[i], 1, 2, 3, 4, 5, 6)

    members = [
        ("TRF0", research.MEMBER, bytes(trf0)),
        ("TRF1", research.MEMBER, bytes(i[2] for i in items)),
    ]
    if edges:
        members += [
            ("TRF2", research.MEMBER, struct.pack(f"<{n}i", *(len(r) for r in requires))),
            ("TRF3", research.MEMBER,
             struct.pack(f"<{sum(len(r) for r in requires)}i",
                         *[x for r in requires for x in r])),
            ("TRF4", research.MEMBER, struct.pack(f"<{n}i", *(len(u) for u in unlocks))),
            ("TRF5", research.MEMBER,
             struct.pack(f"<{sum(len(u) for u in unlocks)}i",
                         *[x for u in unlocks for x in u])),
        ]
    members += [
        ("TRF7", research.MEMBER, bytes(codes)),
        ("TRF8", research.MEMBER, bytes(names)),
    ]
    if parts:
        members += [
            ("TRF6", research.MEMBER, bytes(trf6)),
            ("TRFB", research.MEMBER, bytes(trfb)),
        ]
    return build_nres(members)


ITEMS = [
    ("Small Lab", "RC-17", 4, (1.0, 2.0, 3.0, 4.0), ()),
    ("Large Lab", "RC-47", 4, (5.0, 6.0, 7.0, 8.0), (0,)),
    ("Big Gun", "BG", 7, (0.0, 0.0, 1.0, 1.0), (1,)),
]


def test_a_tree_reads_its_items():
    t = research.parse(build_trf(ITEMS))
    assert len(t) == 3
    assert [i.name for i in t.items] == ["Small Lab", "Large Lab", "Big Gun"]
    assert [i.code for i in t.items] == ["RC-17", "RC-47", "BG"]
    assert t[0].values == (1.0, 2.0, 3.0, 4.0)
    assert t[2].kind == "basic"
    assert t[0].kind == "main"


def test_prerequisites_and_unlocks_are_transposes():
    t = research.parse(build_trf(ITEMS))
    assert t[0].requires == ()
    assert t[1].requires == (0,)
    assert t[2].requires == (1,)
    assert t[0].unlocks == (1,)
    assert t[1].unlocks == (2,)
    assert t[2].unlocks == ()
    assert t.edges == 2


def test_roots_and_leaves():
    t = research.parse(build_trf(ITEMS))
    assert [i.name for i in t.roots] == ["Small Lab"]
    assert t[0].root and not t[0].leaf
    assert t[2].leaf and not t[2].root


def test_an_archive_without_edges_still_reads():
    t = research.parse(build_trf(ITEMS, edges=False))
    assert len(t) == 3
    assert t.edges == 0
    assert all(i.root and i.leaf for i in t.items)


def test_find_matches_case_insensitively():
    t = research.parse(build_trf(ITEMS))
    assert [i.index for i in t.find("lab")] == [0, 1]
    assert t.find("nothing here") == ()


def test_render_indents_what_each_item_opens():
    lines = [x for x in research.render(research.parse(build_trf(ITEMS))) if x]
    assert lines == ["Small Lab  [RC-17]", "    Large Lab  [RC-47]", "        Big Gun  [BG]"]


def build_broken(trf0_records=3, trf2=(0, 1, 1), trf3=(0, 1)) -> bytes:
    """An archive whose columns deliberately disagree."""
    n = 3
    return build_nres([
        ("TRF0", research.MEMBER, b"\0" * (trf0_records * research.RECORD)),
        ("TRF1", research.MEMBER, bytes(n)),
        ("TRF2", research.MEMBER, struct.pack(f"<{len(trf2)}i", *trf2)),
        ("TRF3", research.MEMBER, struct.pack(f"<{len(trf3)}i", *trf3)),
        ("TRF7", research.MEMBER, b"\0"),
        ("TRF8", research.MEMBER, b"\0"),
    ])


def test_a_mismatched_count_is_refused():
    """TRF2 promises three prerequisites; TRF3 only holds two."""
    with pytest.raises(research.ResearchFormatError, match="TRF2 counts 3"):
        research.parse(build_broken(trf2=(1, 1, 1), trf3=(0, 1)))


def test_a_short_record_stream_is_refused():
    with pytest.raises(research.ResearchFormatError, match="not 3 x 40"):
        research.parse(build_broken(trf0_records=2))


def test_a_missing_stream_is_refused():
    data = build_nres([("TRF0", research.MEMBER, b"\0" * research.RECORD)])
    with pytest.raises(research.ResearchFormatError, match="no TRF1"):
        research.parse(data)


def test_the_loader_order_is_the_twelve_streams():
    """A typo in READ_ORDER would silently weaken the checks that use it."""
    assert sorted(research.READ_ORDER) == sorted(research.STREAMS)
    assert set(research.OPTIONAL) < set(research.STREAMS)


PARTS = [("e_gun_bc_05", 0), ("e_tur_bb_01", 2), ("e_tur_bt_01", 2)]


def test_trfb_maps_parts_onto_items():
    t = research.parse(build_trf(ITEMS, parts=PARTS))
    assert t.parts == {"e_gun_bc_05": 0, "e_tur_bb_01": 2, "e_tur_bt_01": 2}
    assert t[0].parts == ("e_gun_bc_05",)
    assert t[2].parts == ("e_tur_bb_01", "e_tur_bt_01")
    assert t[1].parts == ()


def test_a_part_index_is_a_trfb_index_not_an_item_one():
    t = research.parse(build_trf(ITEMS, parts=PARTS))
    assert t.part_ids == ("e_gun_bc_05", "e_tur_bb_01", "e_tur_bt_01")
    assert t.part_at(t[2].part_index) in t[2].parts
    assert t.part_at(99) == ""


def test_an_item_is_found_by_its_part():
    t = research.parse(build_trf(ITEMS, parts=PARTS))
    assert t.item_for("E_TUR_BT_01").name == "Big Gun"
    assert t.item_for("nothing") is None


def test_a_trfb_entry_that_names_no_item_is_refused():
    with pytest.raises(research.ResearchFormatError):
        research.parse(build_trf(ITEMS, parts=[("ghost", 9)]))


def test_the_record_tail_is_six_separate_bytes():
    t = research.parse(build_trf(ITEMS, parts=PARTS))
    assert t[0].tail == (1, 2, 3, 4, 5, 6)


def test_a_tree_without_trfb_still_reads():
    t = research.parse(build_trf(ITEMS))
    assert t.part_ids == () and t[0].parts == ()


def test_an_items_tail_names_its_role_size_and_level():
    item = research.Item(0, "Small Builder", "Bs1", 1, (3.0, 5.0, 3.0, 5.0), (), (),
                         tail=(research.ROLE_BUILDER, 9, 33, 255, 1, 2))
    assert (item.role, item.size, item.upgrade_level) == (research.ROLE_BUILDER, 1, 2)
    bare = research.Item(1, "", "", 0, (0.0, 0.0, 0.0, 0.0), (), ())
    assert bare.role == research.ROLE_NONE
