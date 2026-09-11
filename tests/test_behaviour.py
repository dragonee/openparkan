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


VARSET = """\
//VAR( Type, Name, DefValue, Minimum, Maximum, Comment)
//STRING( Size, Name, DefValue, Comment)

// Numbers like vars
VAR( float, f0, 0)
VAR( float, f1, 1)
VAR( DWORD, d0, 0)
///////////////////////////////////////
VAR( DWORD, dCurrentProblem, 0)
VAR( DWORD, dTemp, 0)   // a scratch slot
VAR( DWORD, ClanID, 0);
STRING( 32, sName, "")
"""


def write_varset(tmp_path, text=VARSET):
    d = tmp_path / "MISSIONS" / "SCRIPTS"
    d.mkdir(parents=True)
    (d / behaviour.VARSET).write_text(text.replace("\n", "\r\n"), "latin-1")
    return tmp_path


def test_the_symbol_table_reads_in_file_order(tmp_path):
    table = behaviour.variables(write_varset(tmp_path))
    assert [v.name for v in table] == [
        "f0", "f1", "d0", "dCurrentProblem", "dTemp", "ClanID", "sName",
    ]
    assert table[0].kind == "VAR"
    assert table[0].type == "float"
    assert table[-1].kind == "STRING"
    assert table[-1].type == "32"


def test_a_trailing_semicolon_does_not_lose_a_declaration(tmp_path):
    """Dropping one would shift every index after it."""
    table = behaviour.variables(write_varset(tmp_path))
    assert behaviour.name_at(table, 5) == "ClanID"


def test_the_literal_pool_is_recognised(tmp_path):
    table = behaviour.variables(write_varset(tmp_path))
    assert [v.name for v in table if v.literal] == ["f0", "f1", "d0"]
    assert not table[3].literal


def test_name_at_is_empty_outside_the_table(tmp_path):
    table = behaviour.variables(write_varset(tmp_path))
    assert behaviour.name_at(table, behaviour.NULL) == ""
    assert behaviour.name_at(table, 999) == ""
    assert behaviour.name_at(table, 0) == "f0"


def test_an_empty_symbol_table_is_refused(tmp_path):
    with pytest.raises(behaviour.ScriptFormatError, match="no declarations"):
        behaviour.variables(write_varset(tmp_path, "// nothing here\n"))


def test_a_node_names_its_destination_and_its_sources():
    data = build_script([("Init", [build_node(head=(3, 4, -1, -1), operands=(0, 2))])])
    node = behaviour.parse(data).handlers[0].nodes[0]
    assert node.destination == 4
    assert node.operands == (0, 2)


def test_a_node_may_write_nowhere():
    node = behaviour.parse(build_script([("Init", [build_node()])])).handlers[0].nodes[0]
    assert node.destination == behaviour.NULL


def test_a_call_carries_a_function_and_nothing_else():
    data = build_script([("Init", [
        build_node(head=(19, -1, -1, -1), operands=(224, 225, 226), trailer=-1)])])
    node = behaviour.parse(data).handlers[0].nodes[0]
    assert node.calls
    assert node.function == 19
    assert not node.assigns
    assert node.source == behaviour.NULL
    assert node.immediate == behaviour.NULL


def test_an_assignment_reads_a_variable_through_the_trailer():
    data = build_script([("Init", [build_node(head=(-1, 171, -1, -1), trailer=4)])])
    node = behaviour.parse(data).handlers[0].nodes[0]
    assert not node.calls
    assert node.assigns
    assert node.destination == 171
    assert node.source == 4
    assert node.immediate == behaviour.NULL


def test_an_assignment_may_carry_an_immediate_instead():
    data = build_script([("Init", [build_node(head=(-1, 171, 6, 6), trailer=-1)])])
    node = behaviour.parse(data).handlers[0].nodes[0]
    assert node.assigns
    assert node.immediate == 6
    assert node.source == behaviour.NULL
    assert node.head[3] in behaviour.ASSIGN_TAGS


def test_a_node_that_writes_nothing_assigns_nothing():
    data = build_script([("Init", [build_node(head=(-1, -1, -1, 3))])])
    node = behaviour.parse(data).handlers[0].nodes[0]
    assert not node.calls
    assert not node.assigns
    assert node.head[3] not in behaviour.ASSIGN_TAGS


def test_the_tag_reads_head2_as_a_variable():
    data = build_script([("Init", [build_node(head=(-1, 171, 174, -1))])])
    node = behaviour.parse(data).handlers[0].nodes[0]
    assert node.tag == behaviour.REFERENCE_TAG
    assert node.reference == 174
    assert node.literal == behaviour.NULL
    assert node.assigns


def test_the_other_tag_reads_head2_as_a_number():
    data = build_script([("Init", [build_node(head=(-1, 171, 28, 6))])])
    node = behaviour.parse(data).handlers[0].nodes[0]
    assert node.tag == behaviour.LITERAL_TAG
    assert node.literal == 28
    assert node.reference == behaviour.NULL


def test_a_flagged_literal_is_returned_as_written():
    flagged = -2147483643        # 0x80000005
    data = build_script([("Init", [build_node(head=(-1, 171, flagged, 6))])])
    node = behaviour.parse(data).handlers[0].nodes[0]
    assert node.literal == flagged
    assert node.literal & behaviour.LITERAL_FLAG


def test_the_tags_that_take_an_operand():
    assert [t for t, n in behaviour.TAG_ARITY.items() if n == 1] == [3, 4]
    assert behaviour.TAG_ARITY[behaviour.REFERENCE_TAG] == 0
    assert set(behaviour.ASSIGN_TAGS) <= set(behaviour.TAG_ARITY)


def test_a_call_renders_with_its_variable_names(tmp_path):
    table = behaviour.variables(write_varset(tmp_path))
    node = behaviour.parse(build_script([("Init", [
        build_node(head=(19, -1, -1, -1), operands=(0, 2, 5))])])).handlers[0].nodes[0]
    assert behaviour.render_node(node, table) == "fn19(f0, d0, ClanID)"


def test_a_call_that_writes_renders_as_an_assignment(tmp_path):
    table = behaviour.variables(write_varset(tmp_path))
    node = behaviour.parse(build_script([("Init", [
        build_node(head=(29, 4, -1, -1), operands=(2,))])])).handlers[0].nodes[0]
    assert behaviour.render_node(node, table) == "dTemp = fn29(d0)"


def test_the_other_forms_render(tmp_path):
    table = behaviour.variables(write_varset(tmp_path))

    def one(head, opcode=6, operands=(), trailer=-1):
        data = build_script([("Init", [build_node(head, opcode, operands, trailer)])])
        return behaviour.render_node(behaviour.parse(data).handlers[0].nodes[0], table)

    assert one((-1, 4, -1, -1), trailer=0) == "dTemp = f0"
    assert one((-1, 4, 28, 6)) == "dTemp = 28"
    assert one((-1, 4, 174, -1)) == "dTemp = 174"
    assert one((-1, -1, -1, 5)) == "tag5"
    assert one((-1, -1, -1, 3), operands=(2,)) == "tag3(d0)"
    assert one((-1, -1, -1, -1), opcode=1, operands=(0, 2)) == "op1(f0, d0)"


def test_a_flagged_literal_is_shown_as_flagged(tmp_path):
    table = behaviour.variables(write_varset(tmp_path))
    data = build_script([("Init", [build_node((-1, 4, -2147483643, 6))])])
    line = behaviour.render_node(behaviour.parse(data).handlers[0].nodes[0], table)
    assert line.startswith("dTemp = 5")
    assert "flag" in line


def test_render_can_pick_one_handler(tmp_path):
    table = behaviour.variables(write_varset(tmp_path))
    script = behaviour.parse(build_script([("Init", [build_node()]), ("Mission", [])]))
    lines = behaviour.render(script, table, "Mission")
    assert lines[0].startswith("Mission:")
    assert not any("Init" in x for x in lines)
