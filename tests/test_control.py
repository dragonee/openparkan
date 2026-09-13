"""The .ctl controller: the frame, the sections, and what they refuse."""

from __future__ import annotations

import struct

import pytest

from openparkan import control


def test_the_smallest_controller_is_the_frame_and_the_block(ctl):
    blob = ctl()
    assert len(blob) == control.FRAME_SIZE
    c = control.parse(blob)
    assert c.counts == (0, 0, 0, 0, 0)
    assert c.components == ()
    assert c.references == ()
    assert c.bare is True


def test_the_parameter_block_reads_as_triples(ctl):
    params = [0.0] * 27
    for i in range(3):
        params[i] = 2.5                     # +20, +24, +28
        params[9 + i] = control.FULL_TURN   # +56, +60, +64
    params[23] = control.HALF_CONE          # +112
    blob = ctl(params=params)
    c = control.parse(blob)
    assert c.triples[0] == (2.5, 2.5, 2.5)
    assert c.triples[3] == pytest.approx((control.FULL_TURN,) * 3)
    assert c.cone == pytest.approx(control.HALF_CONE)
    assert c.triples[control.TRIPLE_TURN] == pytest.approx((control.FULL_TURN,) * 3)


def test_sections_one_and_two_are_stepped_over(ctl, reference):
    blob = ctl(counts=(2, 3, 4, 0, 1),
               groups=[[reference("effects.rlb", "dust_03")]])
    c = control.parse(blob)
    assert c.counts == (2, 3, 4, 0, 1)
    assert len(c.references) == 1
    assert str(c.references[0].resource) == "effects.rlb/dust_03"


def test_a_component_carries_its_resource_and_label(ctl, component):
    part = component(9, "objects.rlb", "bb_l_01", entries=(4, 5), label="i_eng_l")
    c = control.parse(ctl(counts=(0, 0, 0, 1, 0), components=[part]))
    assert len(c.components) == 1
    one = c.components[0]
    assert one.type_id == 9
    assert str(one.resource) == "objects.rlb/bb_l_01"
    assert one.entries == (4, 5)
    assert one.label == "i_eng_l"
    assert one.index is None                # the parser's -1 means absent
    assert one.size == control.COMPONENT_FIXED + 8 + 4 + len("i_eng_l") + 1


def test_a_component_carries_sixteen_values(ctl, component):
    part = component(10, values=(0.5, 0.25, 0.125) + (0.0,) * 12 + (2.0,))
    one = control.parse(ctl(counts=(0, 0, 0, 1, 0), components=[part])).components[0]
    assert len(one.values) == control.COMPONENT_VALUE_COUNT
    assert one.values[:3] == (0.5, 0.25, 0.125)
    assert one.values[-1] == 2.0


def test_only_the_efficiency_class_has_an_efficiency(ctl, component):
    parts = [component(control.EFFICIENCY_TYPE, values=(5.0,)),
             component(10, values=(5.0,))]
    got = control.parse(ctl(counts=(0, 0, 0, 2, 0), components=parts)).components
    assert got[0].efficiency == 5.0
    assert got[1].efficiency is None


def test_a_component_carries_its_power_and_channel(ctl, component):
    parts = [component(control.POWER_STORE_TYPE, power=25.0, values=(10.0,)),
             component(control.EFFICIENCY_TYPE, power=0.01, values=(1.0,))]
    store, work = control.parse(ctl(counts=(0, 0, 0, 2, 0), components=parts)).components
    assert store.power == 25.0 and store.channel == 1
    assert work.power == pytest.approx(0.01) and work.channel == 3


def test_a_component_names_its_node(ctl, component):
    part = component(5, node=17)
    one = control.parse(ctl(counts=(0, 0, 0, 1, 0), components=[part])).components[0]
    assert one.node == 17


def test_a_component_carries_its_mass(ctl, component):
    part = component(control.ENGINE_TYPE, mass=1350.0)
    one = control.parse(ctl(counts=(0, 0, 0, 1, 0), components=[part])).components[0]
    assert one.mass == 1350.0


def test_a_radar_has_a_range(ctl, component):
    parts = [component(control.RADAR_TYPE, values=(0.05, 0.7, 25.0, 400.0, 750.0)),
             component(control.ENGINE_TYPE, values=(0.0, 0.0, 0.0, 400.0))]
    radar, engine = control.parse(ctl(counts=(0, 0, 0, 2, 0), components=parts)).components
    assert radar.sensor_range == 400.0
    assert engine.sensor_range is None


def test_a_detect_shield_has_a_camouflage(ctl, component):
    part = component(control.DETECT_SHIELD_TYPE, values=(0.9, 0.45, 0.01, 0.84, 0.2))
    one = control.parse(ctl(counts=(0, 0, 0, 1, 0), components=[part])).components[0]
    assert one.camouflage == pytest.approx(0.84)
    assert one.sensor_range is None


def test_section_one_reads_as_states(ctl, state):
    walk = state(flags=0x2, velocity=((0.0, 0.6, 0.0), (0.0, 12.0, 0.0)), engine=1.5,
                 conditions=2)
    stand = state(conditions=2)
    c = control.parse(ctl(counts=(2, 2, 0, 0, 0), states=[walk, stand]))
    assert len(c.states) == 2
    assert c.states[0].flags == 0x2
    assert c.states[0].velocity == ((0.0, pytest.approx(0.6), 0.0), (0.0, 12.0, 0.0))
    assert c.states[0].engine == 1.5
    assert c.states[1].engine == 0.0


def test_every_channel_is_served_once():
    served = [c for group in control.POWER_ORDER for c in group]
    assert sorted(served) == sorted(set(control.POWER_CHANNEL))
    assert len(control.POWER_CHANNEL) == control.COMPONENT_TYPES.stop


def test_named_gathers_from_components_and_groups(ctl, component, reference):
    blob = ctl(counts=(0, 0, 0, 1, 1),
               components=[component(1, "objects.rlb", "bb_l_01")],
               groups=[[reference("effects.rlb", "step_rb"),
                        reference("", "")]])
    c = control.parse(blob)
    assert [str(r) for r in c.named] == ["objects.rlb/bb_l_01",
                                         "effects.rlb/step_rb"]


def test_a_short_member_is_refused():
    with pytest.raises(control.ControlFormatError, match="short of"):
        control.parse(bytes(control.FRAME_SIZE - 1))


def test_a_negative_count_is_refused(ctl):
    blob = bytearray(ctl())
    struct.pack_into("<i", blob, 0, -1)
    with pytest.raises(control.ControlFormatError, match="negative"):
        control.parse(bytes(blob))


def test_an_unknown_component_type_is_refused(ctl, component):
    blob = bytearray(ctl(counts=(0, 0, 0, 1, 0),
                         components=[component(9, "a.rlb", "b")]))
    struct.pack_into("<i", blob, control.HEADER_SIZE, 99)
    with pytest.raises(control.ControlFormatError, match="does not read"):
        control.parse(bytes(blob))


def test_trailing_bytes_are_refused(ctl):
    # with no sections at all the size guard catches it first
    with pytest.raises(control.ControlFormatError, match="not 212"):
        control.parse(ctl() + b"\x00\x00\x00\x00")
    # with a section, the walk runs and finds the file longer than the layout
    with pytest.raises(control.ControlFormatError, match="consumed"):
        control.parse(ctl(counts=(1, 0, 0, 0, 0)) + b"\x00\x00\x00\x00")


def test_an_archive_filter_rejects_a_name_it_does_not_know(ctl, reference):
    blob = ctl(counts=(0, 0, 0, 0, 1),
               groups=[[reference("nowhere.rlb", "x")]])
    control.parse(blob)                                   # no filter: fine
    with pytest.raises(control.ControlFormatError):
        control.parse(blob, frozenset({"effects.rlb"}))
