"""A unit sheet, printed: what it says and what it leaves out."""

from __future__ import annotations

from pathlib import Path

from openparkan import objects, units, weapons


def sheet(**overrides) -> units.Unit:
    rocket = weapons.Round("br_b_01", speed=80.0, range=450.0, damage=1900.0, kind=3,
                           blast=4.0, guided=False)
    gun = weapons.Gun(part="e_gun_bl_10", type_id=2, slot="i_c10_b", magazine=18,
                      capacitor=4.0, shot_energy=2.0, interval_ms=1500.0, barrels=9,
                      beams=9, salvo=False, round=rocket)
    chassis = units.Chassis(
        part="R_B_04", name="Large Track Chs", code="L-42t", size="b", locomotion="tracked",
        top_speed=90.0, acceleration=26.0, payload=80.0, slope=34.4, body_mass=20000.0,
        hit_points=4500.0, slots={"engine": "b"}, battery=(10000.0, 250.0),
        engine=(1.0, 20.0), cost=units.Cost(20, 150, 20, 120))
    fields = dict(
        source=Path("w_b_trk1.dat"), label="Large Track Chs (L-42t)",
        type=objects.TYPE_WARRIOR, role="warrior", profile="prof_war.var", size_class=4,
        chassis=chassis, turret=None,
        weapons=[units.Weapon("Base_LU_02", gun, "Large Rocket Lr", "LRL9L", "ROC",
                              weapons.Clip("i_c10_b_01", "i_c10_b", 18, 5.0, "br_b_01"),
                              "Rocket pack II")],
        packages={"hq": ["Standby", "Guard"], "wingman": ["Standby", "Attack"]})
    fields.update(overrides)
    return units.Unit(**fields)


def test_a_sheet_names_its_chassis_weapons_and_orders():
    text = "\n".join(units.render(sheet()))
    assert "Large Track Chs L-42t (R_B_04), tracked, size b" in text
    assert "Rocket pack II, 18 rounds" in text
    assert "4 m blast" in text
    assert "orders    Standby, Guard; too big to capture" in text


def test_a_small_unit_may_capture_and_a_big_one_may_not():
    assert not sheet().may_capture
    assert sheet(size_class=2).may_capture


def test_firepower_leaves_out_the_builder_beam():
    beam = weapons.Gun(part="e_gun_ms_12", type_id=30, slot="", magazine=-1, capacitor=1.0,
                       shot_energy=100.0, interval_ms=1.0, barrels=2, beams=2, salvo=True,
                       round=None)
    plain = sheet()
    both = sheet(weapons=plain.weapons + [units.Weapon("Base_LU_01", beam, "Mobile bldr",
                                                       "Bm", "BLD")])
    assert both.firepower == plain.firepower
    assert both.weapon_power == plain.weapon_power


def test_a_fitted_battery_replaces_the_chassis_slot():
    fitted = units.Part("battery", "i_pws_b_03", "Large Battery", "chassis",
                        [("holds", "31000")], 20000.0, value=31000.0, power=34.5)
    unit = sheet(parts=[fitted])
    assert unit.battery == (31000.0, 34.5)
    assert sheet().battery == (10000.0, 250.0)
    assert "runs on: battery 31,000 at 34.5/s" in "\n".join(units.render(unit))


def test_the_chassis_body_does_not_eat_into_its_payload():
    # w_b_trk1: 80 t of payload, a 20 t body, 80.6 t in all.
    load = units.Load(payload=80000.0, body=20000.0, total=80596.0)
    assert not load.over
    assert load.spare == 19404.0
    heavy = units.Load(payload=70000.0, body=17500.0, total=121015.0)
    assert heavy.over and heavy.spare == 0.0
