"""The input layer, and the value tables recovered from World3D.dll."""

from __future__ import annotations

import pytest

from openparkan import controls

TABLE = (
    "KEY   SCAN_NULL SCAN_A 1 CICLS_UNKNOWN MCMD_LEFT  1.0 0 0 0.0 0 // OBJ_MOVE_LEFT\r\n"
    "KEY   SCAN_NULL SCAN_A 0 CICLS_UNKNOWN MCMD_LEFT  0.0 0 0 0.0 0 // stop\r\n"
    "MOUSE SCAN_LSHIFT SCAN_MOUSE_X 1 CICLS_CAMERA MCMD_ANGLE_X"
    " 0.1 1 MAN_WRAP 0.05 2000 // CAMERA_RIGHT\r\n"
)


def write(tmp_path, name, text):
    path = tmp_path / name
    path.write_text(text, encoding="latin-1")
    return path


def test_a_table_row_reads_every_field(tmp_path):
    rows = controls.table(write(tmp_path, "m1.tbl", TABLE))
    assert len(rows) == 3
    down, up, mouse = rows

    assert down.device == "KEY"
    assert down.pressed is True
    assert down.key == "SCAN_A"
    assert down.command == "MCMD_LEFT"
    assert down.value == 1.0
    assert down.note == "OBJ_MOVE_LEFT"
    assert down.action == "CMD_OBJ_MOVE_LEFT"

    assert up.pressed is False
    assert up.value == 0.0          # the release cancels what the press began

    assert mouse.device == "MOUSE"
    assert mouse.modifier == "SCAN_LSHIFT"
    assert mouse.state == "MAN_WRAP"
    assert mouse.ramp == pytest.approx(0.05)
    assert mouse.ramp_time == 2000


def test_a_row_resolves_to_the_engines_numbers(tmp_path):
    down = controls.table(write(tmp_path, "m1.tbl", TABLE))[0]
    assert down.code == controls.MCMD["MCMD_LEFT"] == 9
    assert down.class_id == controls.UNKNOWN_CLASS
    assert down.dispatched is True

    mouse = controls.table(write(tmp_path, "m1.tbl", TABLE))[2]
    assert mouse.class_id == controls.CICLS["CICLS_CAMERA"]
    assert mouse.bits == controls.MAN["MAN_WRAP"]


def test_a_row_with_the_wrong_field_count_is_refused(tmp_path):
    path = write(tmp_path, "bad.tbl", "KEY SCAN_NULL SCAN_A 1\r\n")
    with pytest.raises(controls.ControlsFormatError, match="4 fields"):
        controls.table(path)


def test_bindings_read_and_refuse(tmp_path):
    path = write(tmp_path, "ui.man",
                 "CMD_OBJ_MOVE_LEFT SCAN_NULL SCAN_A\r\n"
                 "CMD_TURRET_UP SCAN_LSHIFT SCAN_MOUSE_Y\r\n")
    bound = controls.bindings(path)
    assert [b.command for b in bound] == ["CMD_OBJ_MOVE_LEFT", "CMD_TURRET_UP"]
    assert bound[0].chord == "SCAN_A"
    assert bound[1].chord == "SCAN_LSHIFT+SCAN_MOUSE_Y"
    assert bound[0].codes == (controls.SCAN["SCAN_NULL"], controls.SCAN["SCAN_A"])

    with pytest.raises(controls.ControlsFormatError, match="2 fields"):
        controls.bindings(write(tmp_path, "x.man", "CMD_A SCAN_NULL\r\n"))


def test_descriptors_pair_a_name_with_a_label(tmp_path):
    (tmp_path / "ScanCode.dsc").write_text(
        "        SCAN_NULL       \r\n        SCAN_ESC        Esc\r\n", "latin-1")
    (tmp_path / "Command.dsc").write_text(
        "        CMD_OBJ_MOVE_LEFT\tMove object left\r\n", "latin-1")
    assert controls.scancodes(tmp_path) == {"SCAN_NULL": "", "SCAN_ESC": "Esc"}
    assert controls.commands(tmp_path) == {"CMD_OBJ_MOVE_LEFT": "Move object left"}


def test_build_schemes_check_their_own_counts(tmp_path):
    (tmp_path / controls.BUILD_SCHEMES).write_text(
        "//a comment\r\n"
        "Bunker_Small 2\r\n"
        '  "UNITS\\BUILDS\\BUNKER\\sbunk01.dat"\r\n'
        '  "UNITS\\BUILDS\\BUNKER\\sbunk02.dat"\r\n'
        "Generator 1\r\n"
        '  "UNITS\\BUILDS\\GENER\\gener01.dat"\r\n', "latin-1")
    schemes = controls.build_schemes(tmp_path)
    assert [s.name for s in schemes] == ["Bunker_Small", "Generator"]
    assert len(schemes[0].members) == 2

    (tmp_path / controls.BUILD_SCHEMES).write_text(
        'Bunker_Small 3\r\n  "one.dat"\r\n', "latin-1")
    with pytest.raises(controls.ControlsFormatError, match="declared 3"):
        controls.build_schemes(tmp_path)


class TestRecoveredTables:
    """The name-to-number tables taken out of World3D.dll."""

    def test_the_scan_codes_are_the_real_ones(self):
        assert controls.SCAN["SCAN_ESC"] == 1
        assert controls.SCAN["SCAN_A"] == 30
        assert controls.SCAN["SCAN_LSHIFT"] == 42
        assert controls.SCAN["SCAN_F1"] == 59
        assert controls.SCAN["SCAN_NULL"] == 0
        assert len(controls.SCAN) == 174

    def test_the_commands_are_dense_from_zero(self):
        assert sorted(controls.MCMD.values()) == list(range(22))
        assert controls.MCMD["MCMD_STATE"] == 1
        assert controls.MCMD["MCMD_ANGLE_Z"] == 16

    def test_the_dispatched_range_is_the_controllers(self):
        inside = {n for n in controls.MCMD.values() if n in controls.DISPATCHED}
        assert inside == set(range(1, 17))
        outside = {k for k, v in controls.MCMD.items()
                   if v not in controls.DISPATCHED and v}
        assert outside == {"MCMD_MISSILE", "MCMD_FIRE_ALL",
                           "MCMD_WALK_F", "MCMD_WALK_B", "MCMD_LOCK"}

    def test_states_are_bits_and_are_shared(self):
        assert all(v == 0 or v & (v - 1) == 0 for v in controls.CIS.values())
        assert controls.CIS["CIS_CONTINUEFIGHT"] == controls.CIS["CIS_TURRETCONTROL"]

    def test_classes_are_ids_not_bits(self):
        assert controls.CICLS["CICLS_TURRET"] == 1
        assert controls.CICLS["CICLS_ENGINE"] == 5
        assert "CICLS_UNKNOWN" not in controls.CICLS

    def test_the_two_command_chains_are_disjoint_but_for_one_name(self):
        assert len(controls.CMD_OBJECT) == 43
        assert len(controls.CMD_GAME) == 31
        shared = set(controls.CMD_OBJECT) & set(controls.CMD_GAME)
        assert shared == {controls.CMD_SHARED}
        assert (controls.CMD_OBJECT[controls.CMD_SHARED]
                == controls.CMD_GAME[controls.CMD_SHARED] == 35)
        assert len(controls.CMD) == 73

    def test_the_object_commands_are_banded_by_subsystem(self):
        bands = {
            "CMD_OBJ_MOVE_LEFT": 1, "CMD_OBJ_STOP": 14,
            "CMD_TURRET_LEFT": 20, "CMD_TURRET_CENTER": 24,
            "CMD_CAMERA_LEFT": 30, "CMD_CAMERA_INFRARED": 35,
            "CMD_SELECT_ALL_WEAPON": 40, "CMD_SELECT_WEAPON_9": 49,
            "CMD_FIRE_SELECTED_CONT": 50, "CMD_FIRE_ALL": 66,
        }
        for name, value in bands.items():
            assert controls.CMD_OBJECT[name] == value
        assert max(controls.CMD_OBJECT.values()) == 66

    def test_the_game_commands_are_a_flat_run(self):
        run = sorted(v for v in controls.CMD_GAME.values() if v > 100)
        assert run[0] == 723 and run[-1] == 754
        assert set(range(723, 755)) - set(run) == {743, 745}
        assert controls.CMD_GAME["CMD_JAMES_HQ_MOVE_LEFT"] == 723
        assert controls.CMD_GAME["CMD_QUICK_LOAD"] == 754

    def test_one_command_is_resolved_but_never_named(self):
        assert controls.CMD_UNNAMED == "CMD_FIRE_SELECTED"
        assert controls.CMD[controls.CMD_UNNAMED] == 51
        assert controls.CMD["CMD_FIRE_SELECTED_CONT"] == 50


class TestBindingResolution:
    """A ``.man`` line resolves to numbers without the game being present."""

    def test_a_binding_resolves_its_command_and_chord(self):
        b = controls.Binding("CMD_FIRE_ALL", "SCAN_NULL", "SCAN_BLANK")
        assert b.code == 66
        assert b.handler == "World3D.dll"
        assert b.codes == (0, 57)
        assert b.chord == "SCAN_BLANK"

    def test_a_game_command_names_the_other_binary(self):
        b = controls.Binding("CMD_QUICK_SAVE", "SCAN_LSHIFT", "SCAN_F5")
        assert b.code == 753
        assert b.handler == "iron3d.dll"
        assert b.codes == (42, 63)
        assert b.chord == "SCAN_LSHIFT+SCAN_F5"

    def test_an_unknown_command_resolves_to_the_engines_default(self):
        b = controls.Binding("CMD_NOT_A_COMMAND", "SCAN_NULL", "SCAN_NOT_A_KEY")
        assert b.code == controls.UNRESOLVED
        assert b.handler == ""
        assert b.codes == (0, controls.UNRESOLVED)

    def test_the_shared_command_is_reported_as_an_object_command(self):
        b = controls.Binding(controls.CMD_SHARED, "SCAN_NULL", "SCAN_I")
        assert b.code == 35
        assert b.handler == "World3D.dll"


def test_every_build_scheme_is_a_building_type():
    assert len(controls.SCHEME_TYPES) == controls.BUILD_SCHEME_DECLARED + 1
    assert all(t & 0x80000000 for t in controls.SCHEME_TYPES.values())
    assert controls.BuildScheme("Mine", ()).type == 0x80000004
    assert controls.BuildScheme("Bridge", ()).type is None


class TestRamp:
    """A ramped row moves the command a step per update, growing over its time."""

    def row(self, value: float, ramp: float, ramp_time: int) -> controls.Action:
        return controls.Action("KEY", "SCAN_NULL", "SCAN_G_PLUS", True, "CICLS_UNKNOWN",
                               "MCMD_FORWARD", value, 0, "0", ramp, ramp_time, "OBJ_SPEED_MORE")

    def test_half_way_through_the_ramp_time_the_step_is_half(self):
        assert self.row(1.0, 0.05, 1000).ramped(0.0, 500) == pytest.approx(0.025)

    def test_after_the_ramp_time_the_step_is_the_whole_ramp(self):
        assert self.row(1.0, 0.05, 1000).ramped(0.5, 4000) == pytest.approx(0.55)

    def test_it_never_passes_the_value(self):
        assert self.row(1.0, 0.05, 1000).ramped(0.99, 4000) == 1.0
        assert self.row(-1.0, 0.05, 1000).ramped(-0.98, 4000) == -1.0

    def test_a_row_without_a_ramp_time_sends_its_value(self):
        assert self.row(1.0, 0.0, 0).ramped(0.3, 0) == 1.0
