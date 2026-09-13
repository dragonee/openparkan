"""What a gun's numbers come to: rate, energy and damage a second."""

from __future__ import annotations

import pytest

from openparkan import weapons


def gun(**overrides) -> weapons.Gun:
    shot = weapons.Round("bl_m_01", speed=10000.0, range=1000.0, damage=420.0,
                         kind=2, blast=None, guided=False)
    fields = dict(part="e_gun_mc_21", type_id=2, slot="", magazine=-1, capacitor=20.0,
                  shot_energy=18.0, interval_ms=1600.0, barrels=3, beams=2, salvo=True,
                  round=shot)
    fields.update(overrides)
    return weapons.Gun(**fields)


def test_a_salvo_gun_hits_with_every_beam():
    laser = gun()
    assert laser.shots_per_second == pytest.approx(0.625)
    assert laser.damage_per_second == pytest.approx(525.0)
    assert laser.energy_per_second == pytest.approx(11.25)
    assert not laser.uses_clips


def test_a_gun_that_fires_in_turn_hits_with_one():
    launcher = gun(salvo=False, magazine=9, slot="i_c05_l")
    assert launcher.damage_per_second == pytest.approx(420.0 * 0.625)
    assert launcher.uses_clips


def test_an_interval_under_a_millisecond_is_held_to_one():
    assert gun(interval_ms=0.0).shots_per_second == 1000.0
