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


def test_a_guided_gun_takes_its_gate_from_the_round_and_its_seeker():
    missile = weapons.Round("bm_h_01", speed=70.0, range=350.0, damage=200.0, kind=3,
                            blast=10.0, guided=True, cone=0.85, reach=500.0, lock_ms=4000.0)
    gate = weapons.target_gate(missile)
    assert gate.range == 350.0
    assert gate.cone_cos == pytest.approx(math.cos(0.85))
    assert gate.lock_s == pytest.approx(4.0)
    assert gate.needs_target
    assert weapons.gate_state(gate, None, 1.0) == weapons.GATE_NO_TARGET
    assert weapons.gate_state(gate, 351.0, 1.0) == weapons.GATE_OUT_OF_RANGE
    assert weapons.gate_state(gate, 100.0, math.cos(0.9)) == weapons.GATE_OFF_BARREL
    assert weapons.gate_state(gate, 100.0, math.cos(0.5)) is None


def test_an_unguided_gun_fires_without_a_target_but_not_past_its_range():
    shell = weapons.Round("bb_h_01", speed=350.0, range=500.0, damage=100.0, kind=2,
                          blast=None, guided=False)
    gate = weapons.target_gate(shell)
    assert (gate.range, gate.cone_cos, gate.lock_s) == (500.0, -1.0, -1.0)
    assert not gate.needs_target
    assert weapons.gate_state(gate, None, 0.0) is None
    assert weapons.gate_state(gate, 499.0, -0.9) is None
    assert weapons.gate_state(gate, 501.0, 1.0) == weapons.GATE_OUT_OF_RANGE


def test_a_seeker_reaching_less_than_the_round_cuts_the_range():
    bolt = weapons.Round("bp_x", speed=150.0, range=600.0, damage=1.0, kind=2, blast=None,
                         guided=True, cone=0.5, reach=400.0, lock_ms=250.0)
    assert weapons.target_gate(bolt).range == 400.0
    assert weapons.target_gate(None) == weapons.TargetGate(0.0, -1.0, -1.0)
