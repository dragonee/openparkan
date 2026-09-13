"""An objects.rlb record: what its slots say."""

from __future__ import annotations

from openparkan import objects


def ref(member: str) -> objects.ResourceRef:
    return objects.ResourceRef("behpsp.res" if member.endswith(".var") else "bases.rlb", member)


def test_a_chassis_record_names_its_profile():
    slots = [ref(m) for m in ("r_b_04.msh", "r_b_04.wea", "r_b_04.ndp", "", "r_b_04.ctl",
                              "chas_trk.var")]
    assert objects.ObjectRecord("r_b_04", "BTLU", slots).profile == "chas_trk.var"
    assert objects.ObjectRecord("fr_l_plant", "FORT", slots).profile is None
    assert objects.ObjectRecord("r_b_04", "BTLU", slots[:5]).profile is None


def test_a_units_class_word_is_its_type():
    builder = objects.UnitDefinition(source=None, kind=objects.TYPE_BUILDER, components=[])
    mine = objects.UnitDefinition(source=None, kind=0x80000004, components=[])
    assert not builder.is_building
    assert mine.is_building


def test_a_turret_items_role_decides_a_robots_type():
    unit, turret = objects.PART_KIND_UNIT, objects.PART_SUB_TURRET
    assert objects.part_type(unit, turret, 3, 5) == objects.TYPE_HQ
    assert objects.part_type(unit, turret, 1, 3) == objects.TYPE_TRANSPORT
    assert objects.part_type(unit, turret, 2, 2) == objects.TYPE_WARRIOR
    assert objects.part_type(unit, turret, 4, 255) == objects.TYPE_WARRIOR
    assert objects.part_type(unit, 32, 3, 255) == 0          # a chassis gives none


def test_a_building_items_sub_kind_decides_its_type_and_a_bunker_its_size():
    building = objects.PART_KIND_BUILDING
    assert objects.part_type(building, 19, 3, 255) == 0x80000004      # a mine
    assert objects.part_type(building, objects.PART_SUB_BUNKER, 2, 255) == 0x80020000
    assert objects.part_type(building, 28, 3, 255) == 0                # a ruin: none
