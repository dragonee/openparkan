"""How the golden cross-check decides two dumps agree."""

from __future__ import annotations

from openparkan import golden


def test_equal_trees_have_no_differences():
    tree = {"a": [1, 2.5, "x"], "b": {"c": None}}
    assert list(golden.differences(tree, tree)) == []


def test_floats_agree_within_the_tolerance():
    assert list(golden.differences({"v": 433.0}, {"v": 433.0 * (1 + 1e-7)})) == []
    assert list(golden.differences({"v": 433.0}, {"v": 433.1})) == ["$.v: 433.0 against 433.1"]


def test_a_missing_key_a_length_and_a_string_are_differences():
    found = list(golden.differences({"a": [1, 2], "s": "x", "only": 1},
                                    {"a": [1], "s": "y"}))
    assert "$.a: 2 items against 1" in found
    assert "$.s: 'x' against 'y'" in found
    assert "$.only: only in python" in found


def test_a_boolean_is_not_a_number():
    assert list(golden.differences(True, 1)) == ["$: True against 1"]
