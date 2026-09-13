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
