"""A variant's fifth slot, and the level a round is tested against."""

from __future__ import annotations

import struct

from openparkan import mesh

N = mesh.NO_SLOT


def node(slots, flags=0, name="o01_0"):
    return mesh.Subobject(name=name, flags=flags, parent=-1,
                          anim_start=mesh.NO_ANIMATION, fallback_key=0,
                          slot_index=list(slots) + [N] * (15 - len(slots)))


def test_the_fifth_slot_is_the_collision_slot():
    n = node([3, 4, 5, N, 9])
    assert n.collision_slot() == 9
    # and it is not a level
    assert n.slots_for_lod(0) == [3]


def test_a_hull_has_only_the_collision_slot():
    """Exactly the nodes flagged as hulls carry nothing else; level 0 falls
    back to it, which is how a hull's geometry is found -- never drawn."""
    hull = node([N, N, N, N, 7], flags=mesh.SUBOBJECT_COLLISION, name="CP_01")
    assert hull.is_collision
    assert hull.collision_slot() == 7
    assert hull.slots_for_lod(0) == [7]


def test_a_node_without_one_has_no_collision_slot():
    assert node([1, 2]).collision_slot() is None


def test_a_later_variant_has_its_own_fifth_slot():
    n = node([1, N, N, N, 2, 5, N, N, N, 6])
    assert n.collision_slot(variant=1) == 6


def test_a_node_weighs_by_its_level_zero_slot_only():
    from types import SimpleNamespace
    box = mesh.Slot(0, 1, 0, 1, (0.0, 0.0, 0.0), (2.0, 2.0, 2.0), volume=8.0)
    hull = mesh.Slot(0, 1, 0, 1, (0.0, 0.0, 0.0), (1.0, 1.0, 1.0), volume=1.0)
    model = SimpleNamespace(nodes=[node([0]), node([N, N, N, N, 1])], slots=[box, hull])
    assert mesh.ObjectMesh.node_volume(model, 0) == 8.0
    # a hull's level 0 falls back to its collision slot for drawing; not for weight
    assert mesh.ObjectMesh.node_volume(model, 1) == 0.0


def test_a_round_is_tested_against_level_zero_never_the_fifth_slot():
    n = node([7, 8, N, N, 9])
    assert n.hit_slot() == 7
    assert n.collision_slot() == 9


def test_a_hull_has_no_slot_to_hit():
    n = node([N, N, N, N, 3], flags=mesh.SUBOBJECT_COLLISION, name="CP_m1o1")
    assert n.hit_slot() is None
    assert n.collision_slot() == 3


def test_a_control_point_names_its_node_in_its_first_triple():
    as_float = struct.unpack("<f", struct.pack("<i", 35))[0]
    point = mesh.ControlPoint("CameraCenter", (0.0, as_float, as_float),
                              (0.0, 0.0, 0.0), (0.0, 1.0, 0.0))
    assert point.nodes == (35, 35)


def animated(keys):
    m = mesh.ObjectMesh.__new__(mesh.ObjectMesh)
    object.__setattr__(m, "nodes", [mesh.Subobject(
        name="Turn", flags=0, parent=mesh.NO_PARENT, anim_start=0,
        fallback_key=len(keys) - 1, slot_index=[N] * 15)])
    object.__setattr__(m, "keys", keys)
    object.__setattr__(m, "frame_map", list(range(len(keys))))
    object.__setattr__(m, "frame_count", len(keys))
    return m


def test_a_pose_between_two_keys_is_blended_by_time():
    q = (1.0, 0.0, 0.0, 0.0)
    m = animated([mesh.PoseKey((0.0, 0.0, 0.0), 0.0, q), mesh.PoseKey((2.0, 0.0, 0.0), 1.0, q),
                  mesh.PoseKey((9.0, 0.0, 0.0), 2.0, q)])
    assert m.pose_at(0, 0.0)[0] == (0.0, 0.0, 0.0)
    assert m.pose_at(0, 0.25)[0] == (0.5, 0.0, 0.0)
    assert m.pose_at(0, 1.0)[0] == (2.0, 0.0, 0.0)
    assert m.pose_at(0, 7.0)[0] == (9.0, 0.0, 0.0)


def test_two_frames_and_a_weight():
    q = (1.0, 0.0, 0.0, 0.0)
    m = animated([mesh.PoseKey((0.0, 0.0, 0.0), 0.0, q), mesh.PoseKey((4.0, 0.0, 0.0), 1.0, q),
                  mesh.PoseKey((8.0, 0.0, 0.0), 2.0, q)])
    assert m.blended_pose(0, 0.0, 2.0, 0.0)[0] == (0.0, 0.0, 0.0)
    assert m.blended_pose(0, 0.0, 2.0, 1.0)[0] == (8.0, 0.0, 0.0)
    assert m.blended_pose(0, 0.0, 2.0, 0.5)[0] == (4.0, 0.0, 0.0)
    assert m.blended_pose(0, -1.0, 1.0, 0.5)[0] == (4.0, 0.0, 0.0)


def test_the_cockpit_flag_is_the_bit_once_read_as_a_hull():
    n = node([N, N, N, N, 4], flags=mesh.SUBOBJECT_COCKPIT, name="CP_m1o1")
    assert mesh.SUBOBJECT_COLLISION == mesh.SUBOBJECT_COCKPIT
    assert n.is_cockpit and n.is_collision
    assert n.cockpit_slot() == n.collision_slot() == 4
    assert mesh.COCKPIT_LOD == mesh.COLLISION_SLOT == 4


def test_a_node_without_a_fifth_slot_draws_nothing_to_its_own_view():
    """The draw asks for slot variant*5 + 4 for a registered view, with no
    fallback, so an ordinary node without one is simply not drawn there."""
    assert node([1, 2, 3]).cockpit_slot() is None
    assert node([1, 2, 3, N, 8]).cockpit_slot() == 8


def test_an_object_face_record_ends_in_the_winged_edge_link():
    m = mesh.ObjectMesh.__new__(mesh.ObjectMesh)
    # edge 0 meets the neighbour's edge 2, edge 1 its edge 0, edge 2 is open;
    # 0x1C0 above the six bits is exporter leftover and must not matter
    object.__setattr__(m, "face_class", [0x1C0 | 2 | (0 << 2) | (3 << 4)])
    assert m.edge_twin(0, 0) == 2
    assert m.edge_twin(0, 1) == 0
    assert m.edge_twin(0, 2) is None


def test_a_control_point_is_placed_on_one_node_and_carried_by_another():
    def as_float(i):
        return struct.unpack("<f", struct.pack("<i", i))[0]
    wheel = mesh.ControlPoint("weel_fl", (0.0, as_float(0), as_float(5)),
                              (0.0, 0.0, 0.0), (0.0, 0.0, 1.0))
    assert wheel.placed_on == 0
    assert wheel.carrier == 5
