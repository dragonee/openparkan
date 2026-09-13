"""The canonical dump the golden cross-check compares."""

from __future__ import annotations

import hashlib
import json
import math

from openparkan import dump


def test_an_archive_dumps_its_directory_and_a_hash_per_member(tmp_path, nres_archive):
    path = tmp_path / "t.rlb"
    path.write_bytes(nres_archive([("Texm", "grass.0", b"abc"), ("MSH ", "tree", b"0123")]))
    out = dump.nres(path)
    assert out["kind"] == "nres"
    assert [e["name"] for e in out["entries"]] == ["grass.0", "tree"]
    assert out["entries"][0]["sha256"] == hashlib.sha256(b"abc").hexdigest()
    assert out["entries"][1]["tag"] == "MSH"
    json.dumps(out)


def test_a_float_json_cannot_hold_is_named():
    assert dump.number(math.nan) == "NaN"
    assert dump.number(-math.inf) == "-Infinity"
    assert dump.number(0.5) == 0.5
    assert dump.vector((1.0, math.inf)) == [1.0, "Infinity"]


def test_an_archive_dumps_every_controller(tmp_path, nres_archive, ctl, component, state):
    body = ctl(counts=(1, 0, 1, 1, 0), states=[state(flags=1, engine=1.0)],
               channels=[(49.0, 53.0, 0.5, 100.0, 6.28, 3)],
               components=[component(type_id=5, values=(1.0,))])
    path = tmp_path / "t.rlb"
    path.write_bytes(nres_archive([("CTLD", "r.ctl", body), ("MSH ", "r.msh", b"x")]))
    out = dump.controllers(path)
    assert [c["name"] for c in out["controllers"]] == ["r.ctl"]
    (c,) = out["controllers"]
    assert c["states"][0]["engine"] == 1.0
    assert c["channels"][0][1:3] == [49.0, 53.0]
    assert c["components"][0]["type_id"] == 5 and c["components"][0]["index"] is None
    json.dumps(out)


def test_an_input_table_dumps_its_rows_and_their_numbers(tmp_path):
    path = tmp_path / "hero.tbl"
    path.write_bytes(b"// head\r\nKEY SCAN_NULL SCAN_W 1 CICLS_UNKNOWN MCMD_WALK_F 1.0 0 0 0.0 0 "
                     b"// OBJ_MOVE_FORWARD\r\n")
    (row,) = dump.input_table(path)["rows"]
    assert (row["code"], row["class_id"], row["bits"]) == (19, 0, 0)
    assert row["note"] == "OBJ_MOVE_FORWARD"


def test_damage_tables_and_explosions_dump_every_member(tmp_path, nres_archive):
    import struct

    node = struct.pack("<iff", 0, 500.0, 1000.0) + b"weapon.rlb".ljust(32, b"\0") \
        + b"bb_h_01.exp".ljust(32, b"\0")
    blast = struct.pack("<i4fi", 3, 170.0, 7.0, 1.0, 1.0, 7) + bytes(12 * 64)
    path = tmp_path / "t.rlb"
    path.write_bytes(nres_archive([("NDPR", "t.ndp", struct.pack("<i", 1) + node),
                                   ("EXPL", "t.exp", blast)]))
    (table,) = dump.damage_tables(path)["members"]
    assert table["nodes"][0][1] == 500.0 and table["nodes"][0][3] == ["weapon.rlb", "bb_h_01.exp"]
    (e,) = dump.explosions(path)["members"]
    assert (e["kind"], e["damage"], e["radius"]) == (3, 170.0, 7.0)


def test_effects_dump_their_header_and_each_emitters_live_floats(tmp_path, nres_archive):
    import struct

    header = struct.pack("<iIfff", 1, 1, 1.5, 0.0, 0.0) + bytes(40)
    sprite = bytearray(200)
    struct.pack_into("<I", sprite, 0, 3)
    struct.pack_into("<2f", sprite, 32, 0.01, 0.5)
    path = tmp_path / "effects.rlb"
    path.write_bytes(nres_archive([("FXID", "glow", header + bytes(sprite))]))
    (e,) = dump.fx_effects(path)["members"]
    assert e["header"]["duration"] == 1.5 and e["emitters"][0]["kind"] == 3
    assert e["emitters"][0]["window"][1] == 0.5
    assert [32, e["emitters"][0]["window"][0]] in e["emitters"][0]["live"]
