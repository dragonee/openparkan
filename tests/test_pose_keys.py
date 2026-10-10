"""A pose key that is no unit quaternion (docs/10-sky.md, "The lit colour is the game's own").

The game makes a key's four ``int16`` into a matrix without normalising them
(``Ngi32.dll:0x10017e60``): ``(1 - n) I + n R`` for a squared length of *n*.  The
reader takes that matrix's turn.  The quadruples are the install's own.
"""

from __future__ import annotations

import math

from openparkan import mesh


def degrees(rotation) -> float:
    return 2 * math.degrees(math.acos(min(1.0, abs(rotation[0]))))


def game_matrix(q):
    w, x, y, z = q
    return [[1 - 2 * (y * y + z * z), 2 * (x * y + w * z), 2 * (x * z - w * y)],
            [2 * (x * y - w * z), 1 - 2 * (x * x + z * z), 2 * (y * z + w * x)],
            [2 * (x * z + w * y), 2 * (y * z - w * x), 1 - 2 * (x * x + y * y)]]


def test_a_unit_key_is_its_own_rotation_conjugated():
    r = mesh.key_rotation((23169, 23169, 0, 0))
    assert abs(degrees(r) - 90.0) < 1e-3
    assert r[1] < 0 and r[2] == r[3] == 0


def test_a_zero_key_is_the_identity():
    assert mesh.key_rotation((0, 0, 0, 0)) == (1.0, 0.0, 0.0, 0.0)


def test_a_wheels_wrap_key_is_the_identity_not_a_half_turn():
    # R_M_03 key 78: five sibling wheels carry a zero there.  Normalised it is (0, 0, 1, 0).
    assert degrees(mesh.key_rotation((0, 0, 71, 0))) < 1e-3


def test_a_key_twice_too_long_turns_past_its_unit_quaternion():
    # o_tur_ba_09 key 20: -I + 2 R(a quarter turn about z).
    r = mesh.key_rotation((-32767, 0, 0, -32767))
    assert abs(degrees(r) - (180 - math.degrees(math.atan(2.0)))) < 1e-6
    assert r[1] == r[2] == 0


def test_the_turn_is_the_matrixs_own():
    # M R^T is symmetric: what is left of the game's matrix is a stretch alone.
    for packed in ((-32767, 0, 0, -32767), (0, 0, 71, 0), (20000, 20000, 5000, -30000)):
        r = mesh.key_rotation(packed)
        m = game_matrix(tuple(v / mesh.QUATERNION_SCALE for v in packed))
        turn = game_matrix((r[0], -r[1], -r[2], -r[3]))
        left = [[sum(m[i][k] * turn[j][k] for k in range(3)) for j in range(3)] for i in range(3)]
        assert all(abs(left[i][j] - left[j][i]) < 1e-9 for i in range(3) for j in range(3))
    # ... and on the turret's key the stretch is the square root of 5 across z, 1 along it.
    r = mesh.key_rotation((-32767, 0, 0, -32767))
    m = game_matrix((-1.0, 0.0, 0.0, -1.0))
    turn = game_matrix((r[0], -r[1], -r[2], -r[3]))
    assert abs(sum(m[0][k] * turn[0][k] for k in range(3)) - math.sqrt(5.0)) < 1e-9
    assert abs(sum(m[2][k] * turn[2][k] for k in range(3)) - 1.0) < 1e-9
