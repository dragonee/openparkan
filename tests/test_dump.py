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
