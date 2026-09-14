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


def test_an_atmosphere_dumps_its_keyframes(tmp_path):
    import struct

    def time(hour, minute):
        return struct.pack("<8I", 0, 0, 0, hour, minute, 0, 0, 0)

    # One section of one keyframe; a keyframe's version, time and opcode come first.
    header = struct.pack("<5I", 0xFFFFFFFF, 5, 1, 1, 1) + time(23, 59) + time(0, 15)
    key = struct.pack("<I", 3) + time(12, 30) + struct.pack("<I", 0) + bytes(range(88))
    key += struct.pack("<I", 3) + b"sun" + struct.pack("<I", 0) * 5
    key += struct.pack("<4f", 2.2, 2.0, 5.0, 0.0) + struct.pack("<I", 0)
    trailer = time(1, 30) + struct.pack("<2I", 0, 0)
    path = tmp_path / "sky.ske"
    path.write_bytes(header + key + trailer)
    out = dump.atmosphere(path)
    (k,) = out["keyframes"]
    assert (out["day_seconds"], k["hour"], k["minute"], k["name"], k["opcode"]) == (900, 12, 30, "sun", 0)
    assert k["slots"][1] == [4, 5, 6, 7]
    assert out["start"][3:5] == [1, 30]


def test_an_rsli_archive_dumps_its_decrypted_directory(tmp_path):
    import struct

    from openparkan import rsli

    payload = b"xyz"
    record = bytearray(rsli.ENTRY_SIZE)
    record[:5] = b"A.BIN"
    struct.pack_into("<2h3I", record, 0x10, rsli.STORE_RAW, 0, 3,
                     rsli.HEADER_SIZE + rsli.ENTRY_SIZE, len(payload))
    header = bytearray(rsli.HEADER_SIZE)
    header[:4] = b"NL\x00\x01"
    struct.pack_into("<2h", header, 4, 1, 1)
    struct.pack_into("<2I", header, 0x10, 3, 0x5A5A)
    path = tmp_path / "t.lib"
    path.write_bytes(bytes(header) + rsli.decrypt_table(bytes(record), 0x5A5A) + payload)
    out = dump.rsli_archive(path)
    assert out["kind"] == "rsli" and out["seed"] == 0x5A5A
    (entry,) = out["entries"]
    assert entry["name"] == "A.BIN" and entry["size"] == 3
    assert entry["sha256"] == hashlib.sha256(b"xyz").hexdigest()
    json.dumps(out)


def test_a_script_its_formulas_and_the_variable_table_dump_as_stored(tmp_path):
    import struct

    node = struct.pack("<6i", 19, -1, -1, -1, 6, 3) + struct.pack("<3i", 224, 225, 226) \
        + struct.pack("<i", -1)
    scr = tmp_path / "t.scr"
    scr.write_bytes(struct.pack("<ii", 73, 1) + struct.pack("<i", 4) + b"Init" + b"\0"
                    + struct.pack("<ii", 0, 1) + node)
    fml = tmp_path / "t.fml"
    fml.write_bytes(b"//FormulaSet export file\r\n\r\nFUNCTION( , 20 + 55*fDifficulty,  )\r\n")
    var = tmp_path / "varset.var"
    var.write_bytes(b"//VAR( Type, Name, DefValue)\r\nVAR( DWORD, ERROR,\t0xffffffff);\t// all\r\n")

    (handler,) = dump.script(scr)["handlers"]
    assert handler["nodes"] == [{"head": [19, -1, -1, -1], "opcode": 6,
                                 "operands": [224, 225, 226], "trailer": -1}]
    assert dump.formula_set(fml)["formulas"] == ["20 + 55*fDifficulty"]
    assert dump.variable_table(var)["variables"] == [
        {"kind": "VAR", "type": "DWORD", "name": "ERROR", "default": "0xffffffff"}]
    assert {"scr", "fml", "varset"} <= set(dump.KINDS)


def test_a_key_binding_file_dumps_each_chord_as_written(tmp_path):
    path = tmp_path / "ui.man"
    path.write_bytes(b"CMD_CAMERA_CENTER SCAN_LSHIFT SCAN_RMOUSE\r\n\r\n"
                     b"CMD_JAMES_AIM_TARGET SCAN_NULL SCAN_RMOUSE\r\n")
    out = dump.key_bindings(path)
    assert out["kind"] == "man" and "man" in dump.KINDS
    assert out["bindings"][1] == {"command": "CMD_JAMES_AIM_TARGET", "modifier": "SCAN_NULL",
                                  "key": "SCAN_RMOUSE"}
    json.dumps(out)
