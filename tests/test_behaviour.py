"""The .scr behaviour scripts, on bytes this file builds itself."""

from __future__ import annotations

import struct

import pytest

from openparkan import behaviour


def build_node(head=(-1, -1, -1, -1), opcode=6, operands=(), trailer=-1) -> bytes:
    out = struct.pack("<4i", *head)
    out += struct.pack("<ii", opcode, len(operands))
    out += struct.pack(f"<{len(operands)}i", *operands) if operands else b""
    return out + struct.pack("<i", trailer)


def build_script(handlers, magic=behaviour.MAGIC, pad=behaviour.NAME_PAD) -> bytes:
    """``handlers`` is a list of ``(name, [node bytes])``."""
    out = struct.pack("<II", magic, len(handlers))
    for index, (name, nodes) in enumerate(handlers):
        raw = name.encode("latin-1")
        out += struct.pack("<I", len(raw)) + raw
        out += bytes([pad]) + struct.pack("<ii", index, len(nodes))
        out += b"".join(nodes)
    return out


def test_a_script_reads_its_handlers_and_nodes():
    data = build_script([
        ("Init", [build_node(head=(19, -1, -1, -1), operands=(224, 225, 226))]),
        ("Mission", []),
    ])
    s = behaviour.parse(data)
    assert s.magic == behaviour.MAGIC
    assert [h.name for h in s.handlers] == ["Init", "Mission"]
    assert s.nodes == 1
    node = s.handlers[0].nodes[0]
    assert node.head == (19, -1, -1, -1)
    assert node.opcode == behaviour.VARIADIC
    assert node.operands == (224, 225, 226)
    assert node.trailer == -1
    assert not node.binary


def test_a_binary_node_carries_two_operands():
    data = build_script([("Init", [build_node(opcode=1, operands=(3, 4))])])
    node = behaviour.parse(data).handlers[0].nodes[0]
    assert node.binary
    assert node.operands == (3, 4)


def test_problems_pair_and_events_do_not():
    data = build_script([
        ("Init", []),
        ("PBM_ROBOT_NEEDED_Start", []),
        ("PBM_ROBOT_NEEDED_Continue", []),
    ])
    s = behaviour.parse(data)
    assert s.problems == ("PBM_ROBOT_NEEDED",)
    assert s.events == ("Init",)
    assert s.handler("PBM_ROBOT_NEEDED_Start").phase == "Start"
    assert s.handler("PBM_ROBOT_NEEDED_Continue").phase == "Continue"
    assert s.handler("Init").problem == ""
    assert s.handler("Init").phase == ""
    assert s.handler("nothing") is None


def test_a_bad_magic_is_refused():
    data = build_script([("Init", [])], magic=74)
    with pytest.raises(behaviour.ScriptFormatError, match="magic 74"):
        behaviour.parse(data)


def test_a_bad_name_pad_is_refused():
    data = build_script([("Init", [])], pad=1)
    with pytest.raises(behaviour.ScriptFormatError, match="padded with 1"):
        behaviour.parse(data)


def test_trailing_bytes_are_refused():
    data = build_script([("Init", [])]) + b"\0\0\0\0"
    with pytest.raises(behaviour.ScriptFormatError, match="left over"):
        behaviour.parse(data)


def test_a_truncated_script_is_refused():
    data = build_script([("Init", [build_node(operands=(1, 2))])])
    with pytest.raises(behaviour.ScriptFormatError, match="ran off the end"):
        behaviour.parse(data[:-8])


def test_an_out_of_range_opcode_is_refused():
    data = build_script([("Init", [build_node(opcode=7)])])
    with pytest.raises(behaviour.ScriptFormatError, match="opcode 7"):
        behaviour.parse(data)


def test_a_misindexed_handler_is_refused():
    data = bytearray(build_script([("Init", []), ("Mission", [])]))
    # the second handler's index sits after its name and pad byte
    at = data.index(b"Mission") + len("Mission") + 1
    struct.pack_into("<i", data, at, 5)
    with pytest.raises(behaviour.ScriptFormatError, match="indexed 5, not 1"):
        behaviour.parse(bytes(data))


class TestTheEnginesOwnNames:
    """What the reader asserts about the shipped corpus, without it."""

    def test_the_event_set_is_nine_and_named(self):
        assert len(behaviour.EVENTS) == 9
        assert behaviour.EVENTS[0] == "Init"
        assert "Hero_Teleported" in behaviour.EVENTS
        assert all(not e.startswith("PBM_") for e in behaviour.EVENTS)

    def test_the_problems_are_fourteen_and_prefixed(self):
        assert len(behaviour.PROBLEMS) == 14
        assert len(set(behaviour.PROBLEMS)) == 14
        assert all(p.startswith("PBM_") for p in behaviour.PROBLEMS)
        assert list(behaviour.PROBLEMS) == sorted(behaviour.PROBLEMS)

    def test_the_arity_split_is_six_binary_and_one_variadic(self):
        assert list(behaviour.BINARY) == [0, 1, 2, 3, 4, 5]
        assert behaviour.VARIADIC == 6
        assert behaviour.VARIADIC not in behaviour.BINARY
        assert list(behaviour.OPCODES) == [*behaviour.BINARY, behaviour.VARIADIC]
