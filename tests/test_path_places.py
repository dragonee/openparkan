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
