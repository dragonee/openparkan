"""The land mesh's surface word, on meshes this file builds itself."""

from __future__ import annotations

from openparkan import landmesh


def build(surface: list[int], names=("GROUND", "ENV_LAVA", "ENV_LAVA_BOT")):
    """A mesh with one face per surface word, cycling the layer-1 materials."""
    n = len(surface)
    return landmesh.LandMesh(
        positions=[(0.0, 0.0, 0.0)] * 3,
        normals=[], uv1=[], uv2=[], blend=[],
        faces=[(0, 1, 2)] * n,
        adjacency=[(0xFFFF,) * 3] * n,
        face_flags=[0] * n,
        face_surface=list(surface),
        face_tex1=[i % len(names) for i in range(n)],
        face_tex2=[0xFF] * n,
        face_patch=[0] * n,
        layer1_names=list(names),
    )


def test_the_lava_bit_is_clear_on_lava():
    # ground, lava, lava bed
    mesh = build([0x10, 0x02, 0x00])
    assert mesh.marks_lava
    assert not mesh.is_lava(0)
    assert mesh.is_lava(1)
    assert mesh.is_lava(2)


def test_water_keeps_the_bit_set():
    """Water is 0x12: liquid, and not lava."""
    mesh = build([0x10, 0x12])
    assert mesh.is_water(1)
    assert not mesh.is_lava(1)


def test_a_map_that_never_sets_the_bit_has_no_lava():
    """Four of the 33 files leave it clear throughout; a clear bit says
    nothing there, and calling the whole map lava would be wrong."""
    mesh = build([0x00, 0x02, 0x00])
    assert not mesh.marks_lava
    assert not any(mesh.is_lava(i) for i in range(3))


def test_field_13_unpacks_as_three_edge_codes():
    """Low to high: edge 0 in bits 0-1, edge 1 in 2-3, edge 2 in 4-5."""
    mesh = build([0x10])
    # edge 0 -> 1, edge 1 -> 2, edge 2 -> 0
    mesh.face_patch[0] = 1 | (2 << 2) | (0 << 4)
    assert [mesh.edge_twin(0, e) for e in range(3)] == [1, 2, 0]


def test_a_free_edge_has_no_twin():
    mesh = build([0x10])
    mesh.face_patch[0] = landmesh.EDGE_NONE | (1 << 2) | (landmesh.EDGE_NONE << 4)
    assert [mesh.edge_twin(0, e) for e in range(3)] == [None, 1, None]


def strip():
    """Two unit squares side by side, two counter-clockwise faces each.

    Face 1 is the upper-left triangle, face 2 the lower-right; a walk between
    them crosses the left diagonal, the shared side and the right diagonal.
    """
    none = landmesh.NO_NEIGHBOUR
    return landmesh.LandMesh(
        positions=[(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (2.0, 0.0, 0.0),
                   (0.0, 1.0, 0.0), (1.0, 1.0, 0.0), (2.0, 1.0, 0.0)],
        normals=[], uv1=[], uv2=[], blend=[],
        faces=[(0, 1, 4), (0, 4, 3), (1, 2, 5), (1, 5, 4)],
        adjacency=[(none, 3, 1), (0, none, none), (none, none, 3), (2, none, 0)],
        face_flags=[0] * 4, face_surface=[0] * 4, face_tex1=[0] * 4,
        face_tex2=[0xFF] * 4, face_patch=[0] * 4,
        face_normal=[(0.0, 0.0, 1.0)] * 4,
    )


def test_the_walk_crosses_the_edge_the_line_leaves_by():
    mesh = strip()
    assert mesh.contains_xy(1, 0.2, 0.7) and not mesh.contains_xy(0, 0.2, 0.7)
    assert mesh.walk(1, (0.2, 0.7), (1.8, 0.3)) == 2
    assert mesh.walk(2, (1.8, 0.3), (0.2, 0.7)) == 1


def test_the_walk_gives_up_off_its_start_face_or_on_a_steep_face():
    mesh = strip()
    assert mesh.walk(0, (0.2, 0.7), (1.8, 0.3)) is None
    mesh.face_normal[3] = (0.0, 0.99, 0.1)
    assert mesh.walk(1, (0.2, 0.7), (1.8, 0.3)) is None
    # leaving the mesh has no neighbour to cross to
    assert mesh.walk(1, (0.2, 0.7), (0.2, 1.5)) is None
