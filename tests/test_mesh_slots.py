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
