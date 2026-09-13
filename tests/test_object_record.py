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
