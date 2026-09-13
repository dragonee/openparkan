"""What a gun's numbers come to: rate, energy and damage a second."""

from __future__ import annotations

import math

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


def test_a_gun_is_not_ready_until_its_arm_is_out():
    aim, ready = weapons.mount_aim(initial=0.25, pitch=0.75, arm=0.5)
    assert aim == pytest.approx(0.5) and not ready
    assert weapons.mount_aim(initial=0.25, pitch=0.75, arm=1.0) == (0.75, True)


def test_a_lobbed_round_aims_above_a_level_target_and_refuses_one_out_of_reach():
    angle = weapons.lobbed_elevation(50.0, 10.0, (0.0, 100.0, 0.0))
    # the lower arc to a level target 100 m out at 50 m/s: sin 2a = g d / v^2
    assert angle == pytest.approx(0.5 * math.asin(10.0 * 100.0 / 2500.0), rel=1e-6)
    assert weapons.lobbed_elevation(50.0, 10.0, (0.0, 300.0, 0.0)) is None
    assert weapons.lobbed_elevation(50.0, 0.0, (0.0, 100.0, 0.0)) is None


def test_the_ai_scores_a_distance_by_its_rounds_speed():
    assert weapons.ai_distance_score(2.5, 0.0, 70.0) == pytest.approx(0.5)
    assert weapons.ai_distance_score(35.0, 0.0, 70.0) == 1.0
    assert weapons.ai_distance_score(142.0, 0.0, 70.0) == 0.0
    assert weapons.ai_distance_score(88.75, 0.0, 70.0) == pytest.approx(0.5)
    assert weapons.ai_distance_score(20.0, 35.0, 70.0) == pytest.approx(0.5)
    assert weapons.ai_fire_wait(300) == (0.1, 0.1)
    assert weapons.ai_fire_wait(-1) == (0.5, 1.5)


def test_a_shot_is_the_barrel_stroke_and_then_the_interval():
    cannon = gun(interval_ms=0.0, stroke_ms=(250.0,))
    assert cannon.shot_ms == 250.0
    assert cannon.fire_rate == pytest.approx(4.0)
    assert cannon.shots_per_second == 1000.0
    laser = gun(interval_ms=200.0, stroke_ms=(250.0,))
    assert laser.fire_rate == pytest.approx(1000.0 / 450.0)
