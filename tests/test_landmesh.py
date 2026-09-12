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
