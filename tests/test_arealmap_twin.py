"""An areal edge's second field is the twin edge."""

from __future__ import annotations

from openparkan import arealmap


def areal(edges):
    return arealmap.Areal(centre=(0.0, 0.0), area=1.0,
                          vertices=[(0.0, 0.0, 0.0)] * len(edges),
                          edges=edges, flags=(1, 0, 1, 0))


def test_a_shared_edge_names_its_twin():
    a = areal([(1, 2), (arealmap.NO_NEIGHBOUR, arealmap.NO_NEIGHBOUR), (3, 0)])
    assert a.twin(0) == (1, 2)
    assert a.twin(2) == (3, 0)


def test_a_boundary_edge_has_no_twin():
    a = areal([(arealmap.NO_NEIGHBOUR, arealmap.NO_NEIGHBOUR)])
    assert a.twin(0) is None


def test_a_lake_sets_all_four_high_bits_of_the_third_word():
    edges = [(arealmap.NO_NEIGHBOUR, arealmap.NO_NEIGHBOUR)]
    lake = arealmap.Areal((0.0, 0.0), 1.0, [(0.0, 0.0, 0.0)], edges, (0, 0, 242, 0))
    shore = arealmap.Areal((0.0, 0.0), 1.0, [(0.0, 0.0, 0.0)], edges, (0, 0, 0x70, 0))
    assert lake.lake
    assert not shore.lake
    assert not areal(edges).lake


def test_a_sub_block_adds_three_pairs_and_a_point_list():
    import struct
    header = struct.pack("<2f2ff3f4I2I", 1.0, 2.0, 0.0, 0.0, 3.0, 0.0, 0.0, 1.0,
                         1, 0, 1, 0, 3, 1)
    vertices = struct.pack("<9f", 0, 0, 0, 1, 0, 0, 0, 1, 0)
    polygon = struct.pack("<6i", 4, 1, -1, -1, 7, 0)
    extra = struct.pack("<6i", 10, 11, 12, 13, 14, 15)
    block = struct.pack("<I6f", 2, 5, 5, 0, 6, 6, 0)
    blob = header + vertices + polygon + extra + block
    a, end = arealmap._read_areal(blob, 0)
    assert end == len(blob)
    assert a.edges == [(4, 1), (-1, -1), (7, 0)]
    assert a.extra_edges == [(10, 11), (12, 13), (14, 15)]
    assert a.sub_blocks == [[(5.0, 5.0, 0.0), (6.0, 6.0, 0.0)]]
