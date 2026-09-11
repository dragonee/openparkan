"""The objects.dlb parts database, on text this file builds itself."""

from __future__ import annotations

import pytest

from openparkan import descriptions
from tests.conftest import build_nres

GUN = """\
//G3:L14
//B:WPN:GUN:MK2:A3
#1L152mmC
#2Large Cannon
#6UpgradeLevel=2
#7ResearchEnergyCost=18
#8ResearchOreCost=50
#9BuildEnergyCost=18
#ABuildOreCost=50
@G@Weight       @B,weight,G,t,5,1@
@G@Rate of fire @B,Frate,G,1/s,5,@
"""

CLIP = """\
//G3:L14
//B:AMM:GUN:MK9:A3
#1152C2
#2Lrg Cannon AP clip
#4a spare line
#5Large Cannon L152mmC
#6UpgradeLevel=2
#7ResearchEnergyCost=0
#8ResearchOreCost=0
#9BuildEnergyCost=15
#ABuildOreCost=12
"""


def build_library(members) -> bytes:
    return build_nres([(descriptions.TAG, name, text.replace("\n", "\r\n").encode())
                       for name, text in members])


def test_a_description_reads_its_fields():
    d = descriptions.parse_entry(GUN, "e_gun_bc_07")
    assert d.part == "e_gun_bc_07"
    assert d.code == "L152mmC"
    assert d.name == "Large Cannon"
    assert (d.group, d.level) == (3, 14)
    assert (d.size, d.kind, d.sub, d.mark) == ("B", "WPN", "GUN", "MK2")
    assert d.size_word == "large"
    assert d.kind_word == "weapon"
    assert d.upgrade == 2


def test_the_four_costs_are_two_resources_charged_twice():
    d = descriptions.parse_entry(GUN)
    assert d.research_cost == (18.0, 50.0)
    assert d.build_cost == (18.0, 50.0)
    assert d.researched


def test_a_clip_names_its_weapon_and_is_free_to_research():
    d = descriptions.parse_entry(CLIP)
    assert d.kind == "AMM"
    assert d.kind_word == "ammunition"
    assert d.belongs_to == "Large Cannon L152mmC"
    assert d.research_cost == (0.0, 0.0)
    assert d.build_cost == (15.0, 12.0)
    assert not d.researched
    assert d.text == ("a spare line",)


def test_the_stat_rows_carry_a_field_and_a_unit():
    d = descriptions.parse_entry(GUN)
    assert [(s.label, s.field, s.unit) for s in d.stats] == [
        ("Weight", "weight", "t"),
        ("Rate of fire", "Frate", "1/s"),
    ]


def test_an_entry_with_no_stats_is_fine():
    assert descriptions.parse_entry(CLIP).stats == ()


def test_a_library_reads_in_file_order():
    data = build_library([("e_gun_bc_07", GUN), ("i_c07_b_01", CLIP)])
    lib = descriptions.parse(data)
    assert list(lib) == ["e_gun_bc_07", "i_c07_b_01"]
    assert lib["e_gun_bc_07"].name == "Large Cannon"
    assert lib["i_c07_b_01"].part == "i_c07_b_01"


def test_a_wrongly_tagged_member_is_refused():
    data = build_nres([("TEXM", "e_gun_bc_07", GUN.encode())])
    with pytest.raises(descriptions.DescriptionFormatError, match="not 'DSCR'"):
        descriptions.parse(data)


def test_an_empty_library_is_refused():
    with pytest.raises(descriptions.DescriptionFormatError, match="no DSCR"):
        descriptions.parse(build_nres([]))


def test_unknown_size_and_kind_pass_through():
    d = descriptions.parse_entry("//G1:L0\n//Z:ZZZ:ZZZ:MK1\n#2Thing\n")
    assert d.size_word == "Z"
    assert d.kind_word == "ZZZ"
