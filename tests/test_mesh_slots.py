"""A variant's fifth slot is collision geometry."""

from __future__ import annotations

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
