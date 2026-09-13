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
