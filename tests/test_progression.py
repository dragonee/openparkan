"""Routes as tactical areals, and the objective list (docs/34-progression.md)."""

from __future__ import annotations

from openparkan import mission

CFG = """\
object primary_objectives
  objective1  = "1. Destroy all the targets"
  objective2  = "2. Capture the neutral warbots"
end

object bonus_objectives
  objective1  = "Keep the bridge"
end

object mission
   only_briefing = false
end
"""


def route(*xy: tuple[float, float]) -> mission.Route:
    return mission.Route(0, [(x, y, 7.0) for x, y in xy])


def test_a_square_holds_its_inside_only():
    square = route((0, 0), (4, 0), (4, 4), (0, 4))
    assert square.contains(2, 2)
    assert not square.contains(5, 2)
    assert not square.contains(2, -1)
    assert not square.contains(-0.5, 2)


def test_the_outline_closes_from_the_last_point():
    # The edge from (0, 4) back to (0, 0) is what makes the left side.
    assert route((0, 0), (4, 0), (4, 4), (0, 4)).contains(0.5, 3.5)


def test_a_notch_is_outside():
    # A U open at the top: the notch between the arms is not in it.
    u = route((0, 0), (6, 0), (6, 6), (4, 6), (4, 2), (2, 2), (2, 6), (0, 6))
    assert u.contains(1, 5) and u.contains(5, 5) and u.contains(3, 1)
    assert not u.contains(3, 4)


def test_height_is_ignored():
    r = mission.Route(0, [(0, 0, -50), (4, 0, 90), (4, 4, 0), (0, 4, 1000)])
    assert r.contains(2, 2)


def test_no_points_hold_nothing():
    assert not mission.Route(0, []).contains(0, 0)


def write(tmp_path, text):
    p = tmp_path / "mission.cfg"
    p.write_bytes(text.replace("\n", "\r\n").encode("latin-1"))
    return p


def test_an_objective_line_is_not_an_object_header(tmp_path):
    blocks = mission.load_cfg(write(tmp_path, CFG))
    assert list(blocks) == ["primary_objectives", "bonus_objectives", "mission"]
    assert list(blocks["primary_objectives"]) == ["objective1", "objective2"]


def test_objectives_are_primary_then_bonus(tmp_path):
    assert mission.objectives(write(tmp_path, CFG)) == [
        mission.Objective("1. Destroy all the targets", False),
        mission.Objective("2. Capture the neutral warbots", False),
        mission.Objective("Keep the bridge", True),
    ]


def test_a_repeated_objective_key_is_two_objectives(tmp_path):
    # Mission 04 writes objective4 twice; the game lists both lines.
    text = CFG.replace('  objective2  = "2. Capture the neutral warbots"\n',
                       '  objective2  = "2. Capture the neutral warbots"\n'
                       '  objective2  = "3. Destroy the enemy"\n')
    path = write(tmp_path, text)
    assert [o.text for o in mission.objectives(path)] == [
        "1. Destroy all the targets", "2. Capture the neutral warbots",
        "3. Destroy the enemy", "Keep the bridge"]
    assert mission.load_cfg(path)["primary_objectives"] == {
        "objective1": "1. Destroy all the targets", "objective2": "3. Destroy the enemy"}
    assert len(mission.load_cfg_lines(path)["primary_objectives"]) == 3
