"""How a placed object resolves to the meshes it is drawn from."""

from __future__ import annotations

from openparkan import assembly


def slot(library: str, member: str) -> bytes:
    return library.encode().ljust(32, b"\0") + member.encode().ljust(32, b"\0")


def test_a_fort_record_finds_its_mesh_through_its_first_slot(tmp_path, nres_archive):
    records = [
        ("FORT", "fr_m_brige", slot("fortif.rlb", "fr_m_brige_geom") + slot("", "")),
        ("STAT", "fr_m_brige_geom",
         slot("fortif.rlb", "fr_m_brige.msh") + slot("fortif.rlb", "x.wea")),
        ("STAT", "s_tree_04", slot("static.rlb", "s_tree_0_04.msh")),
    ]
    (tmp_path / "objects.rlb").write_bytes(nres_archive(records))
    built = assembly.Assembly(tmp_path)
    ref = built.record_mesh(built.library.get("fr_m_brige"))
    assert (ref.library, ref.member) == ("fortif.rlb", "fr_m_brige.msh")
    [part] = built.parts(2, "s_tree_04")
    assert part.ref.member == "s_tree_0_04.msh"
    assert (part.host, part.node) == (-1, -1)


def test_a_unit_path_is_matched_without_regard_to_case(tmp_path, nres_archive):
    (tmp_path / "objects.rlb").write_bytes(nres_archive([]))
    (tmp_path / "UNITS" / "Units" / "HERO").mkdir(parents=True)
    (tmp_path / "UNITS" / "Units" / "HERO" / "tut1_p.dat").write_bytes(b"")
    built = assembly.Assembly(tmp_path)
    found = built.unit_file("UNITS\\UNITS\\HERO\\TUT1_P.dat")
    assert found is not None and found.name.lower() == "tut1_p.dat" and found.is_file()
    assert built.unit_file("UNITS\\UNITS\\HERO\\missing.dat") is None


def test_a_boxs_cylinder_lies_along_its_longest_side_and_is_half_an_end_faces_diagonal_across():
    from openparkan import mesh

    low = (-1.0, -2.0, -3.0)
    assert mesh.box_cylinder(low, (2.0, 2.0, 5.0)) == ((0.5, 0.0, 5.0), (0.5, 0.0, -3.0), 2.5)
    assert mesh.box_cylinder(low, (11.0, 2.0, 0.0)) == ((11.0, 0.0, -1.5), (-1.0, 0.0, -1.5), 2.5)
    assert mesh.box_cylinder(low, (2.0, 10.0, 1.0)) == ((0.5, 10.0, -1.0), (0.5, -2.0, -1.0), 2.5)
    # Ties: a cube stands on z, and x and y level over a shorter z lie along x.
    assert mesh.box_cylinder((0.0,) * 3, (2.0,) * 3)[0] == (1.0, 1.0, 2.0)
    assert mesh.box_cylinder((0.0,) * 3, (2.0, 2.0, 1.0))[0] == (2.0, 1.0, 0.5)
    # About a side asked for: the shortest here.
    assert mesh.box_cylinder(low, (2.0, 2.0, 5.0), axis=0) == ((2.0, 0.0, 1.0), (-1.0, 0.0, 1.0),
                                                              80 ** 0.5 / 2)


def test_a_hung_parts_box_is_turned_by_its_first_node_and_put_at_its_socket(tmp_path, nres_archive):
    import math
    from types import SimpleNamespace

    from openparkan import mesh

    (tmp_path / "objects.rlb").write_bytes(nres_archive([]))
    built = assembly.Assembly(tmp_path)

    def box(low, high):
        (x0, y0, z0), (x1, y1, z1) = low, high
        return [(x0, y0, z0), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0),
                (x0, y1, z1), (x0, y0, z1), (x1, y0, z1), (x1, y1, z1)]

    quarter = (math.cos(math.pi / 4), 0.0, 0.0, math.sin(math.pi / 4))   # 90 degrees about z
    first = ((5.0, 0.0, 0.0), quarter)
    meshes = {
        "hull": SimpleNamespace(volume=SimpleNamespace(corners=box((-1, -2, 0), (1, 2, 1))),
                                nodes=[0, 1], world_pose=lambda n: first),
        "gun": SimpleNamespace(volume=SimpleNamespace(corners=box((0, -0.5, 0), (3, 0.5, 1))),
                               nodes=[0, 1], world_pose=lambda n: first),
    }
    built.mesh = lambda ref: meshes[ref]
    hull = assembly.Part("hull", mesh.IDENTITY_POSE, -1, -1)
    # The agent's own mesh gives its corners as they are, whatever its node 1 does.
    assert built.joined_box([hull]) == ((-1, -2, 0), (1, 2, 1))
    # A hung part's are turned by its node 1 -- 3 along x becomes 3 along y -- and put down at
    # the socket, not at the node's own place 5 further on.
    gun = assembly.Part("gun", ((0.0, 0.0, 1.0), (1.0, 0.0, 0.0, 0.0)), 0, 3)
    low, high = built.joined_box([hull, gun])
    assert [round(v, 6) for v in low] == [-1, -2, 0] and [round(v, 6) for v in high] == [1, 3, 2]
    far, near, radius = built.cylinder([hull, gun])
    assert [round(v, 6) for v in far] == [0, 3, 1] and [round(v, 6) for v in near] == [0, -2, 1]
    assert abs(radius - 8 ** 0.5 / 2) < 1e-9
    assert built.cylinder([]) is None
