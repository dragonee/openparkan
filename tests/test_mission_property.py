"""A mission property's two further words are its bounds."""

from __future__ import annotations

from openparkan import mission


def prop(value, b, c, type_=mission.TYPE_FLOAT, name="X"):
    return mission.Property(name, type_, value, b, c)


def test_the_words_are_minimum_and_maximum():
    p = prop(0.5, 0.0, 1.0)
    assert p.minimum == 0.0 and p.maximum == 1.0
    assert not p.locked


def test_minus_one_on_an_int_is_no_maximum():
    p = prop(3, 0, mission.NO_MAXIMUM, type_=1, name="ClanID")
    assert p.maximum is None


def test_minus_one_on_a_float_is_a_real_bound():
    assert prop(-2.0, -5.0, -1.0).maximum == -1.0


def test_equal_bounds_on_the_value_lock_it():
    assert prop(7, 7, 7, type_=1, name="LogicalID").locked
