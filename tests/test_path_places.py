"""A hall-way vertex's first word is a flag word that makes it a place."""

from __future__ import annotations

import struct

from openparkan import mesh


def test_a_vertex_carries_its_place_flags():
    pod = struct.pack("<3f2I", 1.0, 2.0, 3.0, mesh.PLACE_POD, 0)
    dock = struct.pack("<3f2I", 0.0, 0.0, 0.0, 0x620 | mesh.PLACE_GROUND, 0)
    link = struct.pack("<10I", 0, 1, *([0xFFFFFFFF] * 8))
    graph = mesh.parse_path_graph(pod + dock + link, 2, 1)
    first, second = graph.nodes
    assert first.flags & mesh.PLACE_POD and not first.flags & mesh.PLACE_DOCK
    assert second.flags & mesh.PLACE_DOCK and second.flags & mesh.PLACE_GROUND


def test_a_vertexs_size_gate_reads_its_own_flags():
    vertex = lambda flags: mesh.PathNode((0.0, 0.0, 0.0), flags, 0)
    # A pod carries neither flag: only a unit of size class 2 or less reaches it.
    pod = vertex(mesh.PLACE_POD)
    assert pod.fits(1, 4) and pod.fits(2, 4)
    assert not pod.fits(3, 4) and not pod.fits(4, 4)
    # A ground-level place passes anything, whatever the building is.
    assert vertex(mesh.VERTEX_ANY_SIZE).fits(4, 2)
    # The factories' seven measure the unit against the building.
    sized = vertex(mesh.VERTEX_BUILDING_SIZE)
    assert sized.fits(4, 4) and sized.fits(3, 4)
    assert not sized.fits(4, 3)


def test_a_links_gate_is_its_tails_first_and_fifth_words():
    link = lambda first, second: mesh.PathLink(
        0, 1, tuple(second if i == 5 else first if i == 1 else 0xFFFFFFFF for i in range(8))
    )
    assert link(0xFFFFFFFF, 0xFFFFFFFF).gate == 0
    assert link(1, 1).gate == 0
    assert link(0, 0xFFFFFFFF).gate == mesh.LINK_FLYER_ONLY
    assert link(0, 0).gate == mesh.LINK_SHUT
