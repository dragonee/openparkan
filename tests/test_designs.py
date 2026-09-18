"""A warbot design: the catalogue's rule, the unit box's figures, the name and the file."""

from __future__ import annotations

import struct
from pathlib import Path

import pytest

from openparkan import designs, objects, research


def labels_blob(labels: list[str]) -> bytes:
    out = b""
    for text in labels:
        raw = text.encode("latin-1")
        out += struct.pack("<I", len(raw)) + (raw + b"\0" if raw else b"")
    return out


def item(index: int, name: str, code: str, parts: tuple[str, ...], category: int) -> research.Item:
    return research.Item(index=index, name=name, code=code, category=category,
                         values=(0.0, 0.0, 10.0 + index, 20.0 + index), requires=(),
                         unlocks=(), parts=parts, tail=(2, 9, 33, 255, 3, 0))


def tree() -> research.Tree:
    granted = research.IN_TREE | research.RESEARCHED
    items = (
        item(0, "Large engine", "LEng1", ("i_eng_b_df",), granted),
        item(1, "Large engine", "LEng2", ("i_eng_b_01",), research.IN_TREE),
        item(2, "Large Flying Chs", "L-2f", ("R_B_02",), granted),
        item(3, "Small Wheel Chs", "S-31", ("R_L_03",), granted),
    )
    return research.Tree(source=Path("t.trf"), items=items,
                         part_ids=("R_L_03", "i_eng_b_df", "i_eng_b_01", "R_B_02"))


def test_socket_labels_are_one_length_prefixed_string_a_node():
    blob = labels_blob(["", "", "e_tur_bb", "universal_bl"])
    assert designs.socket_labels(blob) == ["", "", "e_tur_bb", "universal_bl"]
    with pytest.raises(ValueError):
        designs.socket_labels(blob[:-1])


def test_a_gun_socket_offers_its_two_letters_and_r_offers_both():
    assert designs.gun_prefixes("central_bc") == ("e_gun_bc",)
    assert designs.gun_prefixes("universal_bl") == ("e_gun_bl",)
    assert designs.gun_prefixes("universal_mr") == ("e_gun_mc", "e_gun_ml")
    assert designs.gun_prefixes("universal_ls") == ("e_gun_ls",)


def test_the_chassis_page_adds_a_size_a_grade():
    assert designs.chassis_prefixes(2) == ("r_t", "r_l")
    assert designs.chassis_prefixes(4) == ("r_t", "r_l", "r_m", "r_b")
    assert designs.chassis_prefixes(0) == ()


def test_the_catalogue_offers_researched_parts_in_tree_order():
    catalogue = designs.Catalogue(tree())
    assert catalogue.page(designs.chassis_prefixes(4)) == ["R_L_03", "R_B_02"]
    assert catalogue.page(("i_eng_b",)) == ["i_eng_b_df"]
    assert catalogue.default("i_eng_b") == "i_eng_b_df"
    assert catalogue.default("i_pws_b") is None
    assert designs.Catalogue(tree(), full=True).page(("i_eng_b",)) == ["i_eng_b_df", "i_eng_b_01"]
    assert catalogue.spelling("r_b_02") == "R_B_02"


def test_the_catalogue_tests_researched_and_in_tree_and_not_available():
    """``iron3d.dll:0x1008a896`` and ``0x1008a89e`` read bits 2 and 4 of the item's
    category and nothing else: an item open to research (5) is not offered, and one
    researched outside the tree (2) is not either."""
    states = (research.IN_TREE | research.AVAILABLE, research.RESEARCHED,
              research.IN_TREE, 0,
              research.IN_TREE | research.RESEARCHED | research.AVAILABLE)
    items = tuple(item(i, "Chassis", "C", (f"R_B_0{i}",), c) for i, c in enumerate(states))
    parts = tuple(f"R_B_0{i}" for i in range(len(states)))
    catalogue = designs.Catalogue(research.Tree(source=Path("t.trf"), items=items,
                                                part_ids=parts))
    assert catalogue.page(("r_b",)) == ["R_B_04"]
    assert designs.Catalogue(research.Tree(source=Path("t.trf"), items=items, part_ids=parts),
                             full=True).page(("r_b",)) == list(parts)


def test_percent_normalises_over_the_temp_ranges():
    assert designs.percent(0.0, designs.DEFENCE_RANGE) == 0
    assert designs.percent(2941.0, designs.DEFENCE_RANGE) == 12
    assert designs.percent(1127.0, designs.OFFENCE_RANGE) == 17
    assert designs.percent(99999.0, designs.OFFENCE_RANGE) == 100


def test_the_unit_box_lines_and_the_red_full_payload():
    rating = designs.Rating(mass=59420.0, spare=5580.0, speed=13.46, defence=6802.0,
                            offence=1127.0, sensor_range=400.0)
    assert rating.lines() == ["59 / 6 t", "48 kph", "28 %", "17 %", "400 m"]
    assert not rating.full
    assert designs.Rating(66030.0, 0.0, 12.2, 6627.0, 1653.0, 350.0).full


def test_a_design_is_named_by_size_chassis_and_class():
    assert designs.code(4, 1, objects.TYPE_WARRIOR) == "LFW"
    assert designs.code(4, 1, 0) == "LF?"
    assert designs.name("LFW", 0, "Warrior") == "LFW-X Warrior"
    assert designs.name("LFW", 2, "Warrior") == "LFW-2 Warrior"


def test_a_written_design_reads_back_into_its_tree(tmp_path):
    ref = objects.ResourceRef
    unit = objects.UnitDefinition(source=Path("d.dat"), kind=objects.TYPE_WARRIOR, components=[
        objects.Component(ref("objects.rlb", "R_B_02"), "Large Flying Chs (L-2f)", 1, -1, 0, 2),
        objects.Component(ref("objects.rlb", "i_eng_b_df"), "Large engine (LEng1)", 1, 0, 3, 0),
        objects.Component(ref("objects.rlb", "e_tur_bb_01"), "Large Battle Turret (4L1)",
                          1, 7, 1, 0),
    ])
    path = tmp_path / "d.dat"
    path.write_bytes(designs.dat_bytes(unit))
    back = objects.load_unit(path)
    assert [(c.ref.member, c.attach_node, c.class_id, c.child_count) for c in back.components] \
        == [("R_B_02", -1, 0, 2), ("i_eng_b_df", 0, 3, 0), ("e_tur_bb_01", 7, 1, 0)]
    design = designs.Part.from_unit(back)
    assert [c.part for c in design.children] == ["i_eng_b_df", "e_tur_bb_01"]
    assert [p.part for p in design.walk()] == ["R_B_02", "i_eng_b_df", "e_tur_bb_01"]
