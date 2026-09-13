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


def test_sections_one_and_two_do_not_hide_section_five(ctl, reference):
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


def test_a_gun_names_its_barrels_and_whether_they_fire_together(ctl, component):
    gun = component(control.GUN_TYPE, entries=(0, 1, 2), flags=control.SALVO,
                    values=(-1.0, 20.0, 18.0, 1600.0))
    c = control.parse(ctl(counts=(0, 0, 3, 1, 0), components=[gun], points=(4, 5, -1)))
    one = c.components[0]
    assert one.flags & control.SALVO
    assert c.points == (4, 5, -1)
    assert [c.points[e] for e in one.entries if c.points[e] != -1] == [4, 5]


def test_a_turret_says_how_it_is_mounted(ctl, component):
    ground = component(control.TURRET_TYPE, flags=control.MOUNT_UPRIGHT)
    hung = component(control.TURRET_TYPE, flags=control.MOUNT_HQ)
    a, b = control.parse(ctl(counts=(0, 0, 0, 2, 0), components=[ground, hung])).components
    assert a.flags & control.MOUNT_UPRIGHT and not a.flags & control.MOUNT_HQ
    assert b.flags & control.MOUNT_HQ and not b.flags & control.MOUNT_UPRIGHT


def test_a_labelled_record_is_a_slot(ctl, component):
    parts = [component(control.ENGINE_TYPE, label="i_eng_b"),
             component(control.GUN_TYPE, label="i_c05_b")]
    engine, gun = control.parse(ctl(counts=(0, 0, 0, 2, 0), components=parts)).components
    assert engine.slot == "i_eng_b"
    assert gun.slot is None


def test_a_component_carries_its_initial_state(ctl, component):
    parts = [component(24, index=33), component(control.REPAIR_TYPE)]
    turret, repair = control.parse(ctl(counts=(0, 0, 0, 2, 0), components=parts)).components
    assert turret.state == 33
    assert repair.state is None


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


def test_the_block_is_twenty_one_group_indices(ctl, reference):
    groups = [-1] * control.BLOCK_ENTRIES
    groups[0] = 1
    groups[control.SURFACE_GROUPS_AT:] = [0, 1, 0] + [1] * 8
    blob = ctl(counts=(0, 0, 0, 0, 2),
               block=struct.pack(f"<{control.BLOCK_ENTRIES}i", *groups),
               groups=[[reference("", "")], [reference("", ""), reference("", "")]])
    c = control.parse(blob)
    assert c.load_group == 1
    assert c.surface_groups == (0, 1, 0) + (1,) * 8
    assert [r.group for r in c.references] == [0, 1, 1]


def test_an_unset_block_names_no_group(ctl):
    c = control.parse(ctl())
    assert c.load_group == control.NO_GROUP
    assert c.surface_groups == (control.NO_GROUP,) * control.SURFACES


def test_section_two_reads_as_channels(ctl):
    blob = ctl(counts=(0, 0, 2, 0, 0), points=(1, -1),
               channels=[(49.0, 53.0, 0.5, 100.0, 6.28, 3),
                         (55.0, 57.0, 0.25, 0.75, 1.92, 0)])
    yaw, pitch = control.parse(blob).channels
    assert (yaw.first, yaw.last, yaw.initial, yaw.point, yaw.rate, yaw.flags) == (
        49.0, 53.0, 0.5, 1, 100.0, 3)
    assert pitch.point == -1
    assert abs(pitch.span - 1.92) < 1e-6


def test_a_state_carries_its_frame_pairs_blend_and_length(ctl, state):
    blob = bytearray(ctl(counts=(1, 0, 0, 0, 0), states=[state()]))
    at = control.HEADER_SIZE
    struct.pack_into("<I", blob, at + control.STATE_MODE_AT,
                     control.STATE_ANCHOR | control.STATE_BY_VELOCITY)
    struct.pack_into("<5f", blob, at + control.STATE_PAIR_A_AT, 5.0, 6.0, 9.0, 13.0, 0.6)
    struct.pack_into("<f", blob, at + control.STATE_LENGTH_AT, 125.0)
    (s,) = control.parse(bytes(blob)).states
    assert (s.pair_a, s.pair_b, s.length) == ((5.0, 6.0), (9.0, 13.0), 125.0)
    assert abs(s.blend - 0.6) < 1e-6
    assert s.anchor and s.by_velocity


def test_the_cheapest_path_reads_the_table_row_as_the_destination(ctl, state):
    blob = bytearray(ctl(counts=(3, 0, 0, 0, 0), states=[state()] * 3))
    at = control.HEADER_SIZE + 3 * control.SECTION1_RECORD
    no = control.NO_EDGE
    # row = to, column = from: 0 -> 1 costs 1, 1 -> 2 costs 1, 0 -> 2 costs 5
    table = [no, no, no,
             1.0, no, no,
             5.0, 1.0, no]
    struct.pack_into("<9f", blob, at, *table)
    c = control.parse(bytes(blob))
    assert c.cost(1, 0) == 1.0
    assert c.path(0, 2) == [1, 2]
    assert c.path(2, 0) is None


def test_the_loader_scales_a_cost_by_the_gap_from_the_destinations_centre(ctl, state):
    forward = state(flags=0b10, velocity=((0.0, 6.0, 0.0), (0.0, 14.0, 0.0)))
    backward = state(flags=0b10, velocity=((0.0, -14.0, 0.0), (0.0, -6.0, 0.0)))
    blob = bytearray(ctl(counts=(3, 0, 0, 0, 0), states=[forward, backward, forward]))
    at = control.HEADER_SIZE + 3 * control.SECTION1_RECORD
    no = control.NO_EDGE
    struct.pack_into("<9f", blob, at, no, 1.0, 1.0, 1.0, no, 1.0, 1.0, 1.0, no)
    c = control.parse(bytes(blob))
    # Forward to backward: |-10 - 6| = 16; forward to forward: |10 - 6| = 4.
    assert c.live_cost(1, 0) == 17.0
    assert c.live_cost(2, 0) == 5.0
    assert c.path(0, 1, live=True) == [1]


def test_a_channel_names_the_node_it_plays(ctl):
    blob = bytearray(ctl(counts=(0, 0, 1, 0, 0), points=(3,),
                         channels=[(49.0, 53.0, 0.5, 100.0, 6.28,
                                    control.CHANNEL_WRAP | control.CHANNEL_INVERT)]))
    at = control.section4_start((0, 0, 1, 0, 0)) - control.SECTION2_RECORD
    struct.pack_into("<i", blob, at + control.SECTION2_NODE_AT, 7)
    struct.pack_into("<i", blob, at + control.SECTION2_ORIGIN_AT, 2)
    (ch,) = control.parse(bytes(blob)).channels
    assert (ch.node, ch.origin, ch.point) == (7, 2, 3)
    assert ch.frame(0.25) == 52.0
    assert ch.frame(1.25) == 52.0


def test_a_record_is_an_action_with_four_arguments(ctl, reference):
    groups = [-1] * control.BLOCK_ENTRIES
    groups[control.ENTRY_HIT] = 1
    kill = (0, 0, 0, control.ACT_KILL, 0, 0, 0, 0, 0)
    start = (0, 0, 0, control.ACT_EFFECT_START, 5, 0, 0, 0, 0)
    blob = ctl(counts=(0, 0, 0, 0, 2),
               block=struct.pack(f"<{control.BLOCK_ENTRIES}i", *groups),
               groups=[[reference("", "", start)], [reference("", "", start),
                                                    reference("", "", kill)]])
    c = control.parse(blob)
    assert [r.action for r in c.group(control.ENTRY_HIT)] == [
        control.ACT_EFFECT_START, control.ACT_KILL]
    assert c.group(control.ENTRY_HIT)[0].args == (5, 0, 0, 0)
    assert c.group(control.ENTRY_EDGE) == []


def _surface(i: int) -> list[bool]:
    return [n == i for n in range(control.CONDITIONS)]


def _step(effect: int, flags: int, mask: int, invert: int = 0) -> tuple[int, ...]:
    signed = flags - (1 << 32) if flags >= 1 << 31 else flags
    return (signed, mask, invert, control.ACT_EFFECT_START, effect, 1, 0, 0, 0)


def test_a_record_runs_on_its_masked_conditions(ctl, reference):
    any_ = control.REF_ANY
    blob = ctl(counts=(0, 0, 0, 0, 1),
               groups=[[reference("", "", _step(1, any_, 1 << 5)),
                        reference("", "", _step(2, any_, 1 << 7, 1 << 7)),
                        reference("", "", _step(3, control.REF_ALL, (1 << 1) | (1 << 2))),
                        reference("", "", _step(4, 0, 1 << 3))]])
    c = control.parse(blob)
    metal, stone = _surface(5), _surface(1)
    assert [r.args[0] for r in control.run_group(c.references, metal)] == [1, 2, 4]
    bed = list(stone)
    bed[control.COND_BED] = True
    assert [r.args[0] for r in control.run_group(c.references, bed)] == [4]
    assert c.references[1].mask == 1 << 7 and c.references[1].inverted == 1 << 7


def test_a_run_closes_with_an_else(ctl, reference):
    opening = control.REF_OPEN | control.REF_ANY
    closing = opening | control.REF_ELSE
    group = [reference("", "", _step(101, opening, 1 << 5)),
             reference("", "", _step(201, opening, 1 << 1)),
             reference("", "", _step(301, closing, 1 << 9))]
    c = control.parse(ctl(counts=(0, 0, 0, 0, 1), groups=[group]))
    ran = lambda s: [r.args[0] for r in control.run_group(c.references, _surface(s))]  # noqa: E731
    assert ran(5) == [101]
    assert ran(1) == [201]
    assert ran(9) == [301]
    assert ran(0) == [301]          # nothing matched, so the else runs


def test_a_state_carries_its_contacts_and_lean(ctl, state):
    stride = control.SECTION1_RECORD + 2 * control.SECTION1_PER_B
    rec = bytearray(state(conditions=2))
    rec[control.STATE_LEAN_AT:control.STATE_LEAN_AT + 3] = bytes(
        (control.LEAN_NEGATE | control.LEAN_ACCELERATION[1], control.LEAN_TURN[2], 0))
    struct.pack_into("<iIi", rec, control.SECTION1_RECORD, 4,
                     control.NEEDS_INTACT | control.CONTACT_SUPPORT, 3)
    struct.pack_into("<iIi", rec, control.SECTION1_RECORD + 16, 6,
                     control.NEEDS_DESTROYED | control.CONTACT_FALLBACK, -1)
    assert len(rec) == stride
    (s,) = control.parse(ctl(counts=(1, 2, 0, 0, 0), states=[bytes(rec)])).states
    assert [(k.point, k.group) for k in s.contacts] == [(4, 3), (6, -1)]
    assert s.lean == (0x89, 3, 0)
    assert s.allows([True, False])
    assert not s.allows([True, True])
    assert not s.allows([False, False])


def test_a_generic_device_names_its_inputs(ctl, component):
    wheel = bytearray(component(3, entries=(0,), flags=0x01070C00))
    struct.pack_into("<2f", wheel, control.COMPONENT_WEIGHTS_AT, 1.0, 0.5)
    (part,) = control.parse(ctl(counts=(0, 0, 1, 1, 0), components=[bytes(wheel)])).components
    assert part.inputs == (0, 12, 7)
    assert part.weights == (1.0, 0.5)
    assert control.device_input(12) == ("speed", 1)
    assert control.device_input(7) == ("-spin", 2)
    assert control.device_input(14) == ("speed", -1)
    assert control.device_input(1) is None
def test_the_transition_factor_reaches_from_source_minimum_to_destination_centre(ctl, state):
    walk = state(flags=0x2, velocity=((-0.6, 6.0, -0.6), (0.6, 14.0, 0.6)))
    near = state(flags=0x2, velocity=((-0.6, 2.0, -0.6), (0.6, 10.0, 0.6)))
    far = state(flags=0x2, velocity=((-0.6, 10.0, -0.6), (0.6, 20.0, 0.6)))
    c = control.parse(ctl(counts=(3, 0, 0, 0, 0), states=[walk, near, far]))
    # to near (centre y 6) from walk (min y 6): no reach
    assert c.transition_factor(1, 0) == 1.0
    # to far (centre y 15) from walk: 1 + 9, though the boxes overlap
    assert abs(c.transition_factor(2, 0) - 10.0) < 1e-6
    # only the source's switched-on axes count: near switches on y too
    assert abs(c.transition_factor(0, 1) - 9.0) < 1e-6


def test_the_blend_weight_divides_by_the_largest_absolute_difference(ctl, state):
    run = state(velocity=((-0.6, 6.0, -0.6), (0.6, 14.0, 0.6)))
    stand = state(velocity=((-2.0, -2.0, -2.0), (2.0, 2.0, 2.0)))
    s, still = control.parse(ctl(counts=(2, 0, 0, 0, 0), states=[run, stand])).states
    assert s.blends and s.blend == 0.0
    assert (s.blend_floor, s.blend_divisor) == (6.0, 8.0)
    assert abs(s.blend_weight(10.0) - 0.5) < 1e-6
    assert s.blend_weight(20.0) == 1.0 and s.blend_weight(0.0) == 1.0
    # a box symmetric about zero has D = 0: the weight stays 1
    assert still.blend_divisor == 0.0 and still.blend_weight(1.0) == 1.0
