"""Controller parameters -- the ``.ctl`` member of an object's resource record.

``Control.dll`` exports ``LoadControlSystem``, and ``AniMesh.dll`` is the only
module that imports it.  It allocates a 0x668- or 0x670-byte object with six
vtables, hands back the interface at ``+0x14``, and drives it through a
message dispatch; the ``.ctl`` member is one of six ``(archive, member)`` name
pairs it is given.

What this module reads is the **212-byte frame** every one of the 531 shipped
``.ctl`` members begins with: a 128-byte parameter block of speeds,
accelerations and angle limits, and an 84-byte block that six members leave
entirely unset.  The frame is exact -- the five files whose section counts are
all zero are 212 bytes and nothing else.

After the frame come the sections: section 1's animation states, section 2,
section 4's components, an 84-byte block, and section 5's reference groups.
All of them are read.  Section 2 is channels -- a turret's yaw and pitch, a
gun's barrels -- and the block is 21 section-5 group indices: entry 0 runs at
load and entries 10..20 run when the ground's surface id changes.
See ``docs/13-control.md`` and, for what the numbers do, ``docs/24-motion.md``.

Everything below is re-derived by ``uv run openparkan verify``.
"""

from __future__ import annotations

import heapq
import math
import struct
from dataclasses import dataclass

from .objects import ResourceRef, _fixed_string

#: The NRes tag every ``.ctl`` member carries.
CTL_TAG = "CTLD"

#: The frame: five counts and the 27-dword parameter block.
HEADER_SIZE = 128

#: A section-2 record is a **channel**: an animated value from 0 to 1 that
#: steps toward a target by at most ``rate`` a second (``0x100289f0``) and
#: plays mesh frames ``first``..``last``.  Its +20 word is a control point of
#: the same-stem ``.cpt`` -- a barrel, a turret's ``TurretDirect`` or
#: ``TargetDirect`` -- or -1 on none.
#: +0 is the mesh node the channel animates (``Control.dll:0x10008fb9``) and
#: +16 a second control point, the camera channel's ``CameraCenter``.
SECTION2_NODE_AT = 0
SECTION2_FIRST_AT = 4
SECTION2_INITIAL_AT = 12
SECTION2_ORIGIN_AT = 16
SECTION2_POINT_AT = 20
SECTION2_RATE_AT = 24
SECTION2_FLAGS_AT = 32
#: A channel's flags (``0x10009950``, ``0x10021a30``): the value wraps; the
#: node plays 1 - v; the component update does not drive it (the camera); it
#: joins the turret's list (gun mounts that follow pitch); it takes the
#: previous channel's value.
CHANNEL_WRAP = 0x1
CHANNEL_INVERT = 0x2
CHANNEL_UNDRIVEN = 0x4
CHANNEL_TURRET = 0x8
CHANNEL_FOLLOWS = 0x40

#: Section 1's record, section 2's record, the fixed block the loader copies
#: after the component records, and the component type ids the factory at
#: ``0x1002d4b0`` accepts.
SECTION1_RECORD = 156
SECTION1_PER_B = 16
SECTION2_RECORD = 36
BLOCK_SIZE = 84
COMPONENT_TYPES = range(1, 31)

#: The block is 21 int32 section-5 group indices (``Control.dll:0x100093ee``).
#: Entry 0 runs at load; entries 10..20 are one per ground surface id, run
#: when the id under the machine changes (``0x1001ab3e``).
BLOCK_ENTRIES = 21
SURFACE_GROUPS_AT = 10
SURFACES = 11
NO_GROUP = -1
#: Block entries a round runs: when its face hit, the map's edge and the end of
#: its range stop it (``0x1000d35b``, ``0x1000d36e``, ``0x1000d390``).
ENTRY_LOAD = 0
ENTRY_HIT = 2
ENTRY_EDGE = 3
ENTRY_RANGE = 4
#: Block entries the node update runs (``0x10012a40``): 6 when the machine
#: becomes critically damaged, 7 when it stops being so.  Critical is running
#: gear whose summed life fractions fall to ``n / 2 - 1`` or below for ``n``
#: gear nodes (``.ndp`` flags ``0x70``); a machine with none is critical below
#: ``CRITICAL_LIFE`` of its full hit points (``CRITICAL_LIFE_STATIC`` for an
#: agent of kind 10, ``0x1000fa85``).  While critical it loses ``BURN_RATE``
#: of the life it had when it became so, times the node update's dt
#: (``0x10012c1c``).  32 controllers set both entries, and every group
#: switches an effect on (6) and the same one off (7): smoke, a tree's flames.
ENTRY_CRITICAL = 6
ENTRY_RECOVERED = 7
CRITICAL_LIFE = 0.2
CRITICAL_LIFE_STATIC = 0.3
BURN_RATE = 0.1

#: A section-5 record's action, at int 3, and the interpreter's cases
#: (``Control.dll:0x10002800``, table ``0x10003590``).  3, 4 and 5 start an
#: effect named by the record on one control point, on three at their
#: centroid, or in the construction sphere; 8, 10, 11, 18 and 19 delete,
#: start, restart, switch on and switch off effect v4; 14 gives effect v4 its
#: time from control point v5; 15 removes the object without an explosion,
#: 17 kills it so node 0 explodes, 21 kills every unit in the construction
#: sphere and 27 explodes node v4 with the named ``.exp``.
ACTION_AT = 3
ACT_CALL = 0
ACT_EFFECT_POINT = 3
ACT_EFFECT_POINTS = 4
ACT_EFFECT_SPHERE = 5
ACT_NODE_DAMAGE = 7
ACT_EFFECT_DELETE = 8
ACT_EFFECT_START = 10
ACT_EFFECT_RESTART = 11
ACT_EFFECT_TIME_POINT = 14
ACT_REMOVE = 15
ACT_KILL = 17
ACT_EFFECT_ON = 18
ACT_EFFECT_OFF = 19
ACT_KILL_IN_SPHERE = 21
ACT_EXPLODE_NODE = 27
#: Action 0 zeroes the motion body's command, velocity, spin and step velocity
#: (``0x10014400``); action 20, on an agent of kind 3, hands the object to its
#: parent's ``ITerrain`` (interface 5) slot 3, ``CLandscape::PlaceBuilding``
#: (``Terrain.dll:0x1000df10``).
ACT_STOP = ACT_CALL
ACT_PLACE_BUILDING = 20

#: A record's int 0 and ints 1-2 gate it (``Control.dll:0x100022c0``,
#: ``0x100028b2``).  Ints 1 and 2 are a mask and an inversion over the
#: controller's sixteen **condition bytes**: condition ``i`` holds when byte
#: ``i`` is set, or when it is clear and bit ``i`` of the inversion is set.
#: ``REF_ANY`` runs the record when any masked condition holds, ``REF_ALL``
#: when every one does; a record with neither always runs.  ``REF_OPEN`` opens
#: a run and ``REF_ELSE`` closes it: the closing record also runs when no
#: record since the opening one did.  Int 8 is not read.
REF_OPEN = 0x80000000
REF_ANY = 0x40000000
REF_ALL = 0x20000000
REF_ELSE = 0x10000000
MASK_AT = 1
INVERT_AT = 2
#: The condition bytes (controller ``+0x378``).  Bytes 0-10 say which ground
#: surface id the machine stands on (``0x10002790``, set when the id
#: changes); byte 7 is then overwritten with the face's liquid-bed flag
#: (world flag ``0x400``, ``0x1001ab2d``); byte 14 is set while the machine
#: is critically damaged (``ENTRY_CRITICAL``); byte 15 is a copy of
#: ``+0x618``, which message 7 with argument 2 sets.  Bytes 11-13 are never
#: set.
CONDITIONS = 16
COND_BED = 7
COND_CRITICAL = 14
COND_MODE = 15

#: A component record's fixed part, and the fields inside it that are read.
#: Thirteen of the factory's fourteen classes share one parser at
#: ``0x10021d50``; the fourteenth (type 2, which type 30 also uses) calls that
#: one first and then does more with the object, so the record's extent is the
#: same for all of them.
COMPONENT_FIXED = 0xB0
#: The ``(archive, member)`` pair, 32 bytes each -- what the component emits.
COMPONENT_NAME_AT = 0x6C
#: How many 4-byte entries follow the fixed part.
COMPONENT_COUNT_AT = 0xAC
#: The node of the model this component sits on: the control system asks the
#: node table built from the object's ``.ndp`` for that node's remaining life
#: (``Control.dll:0x1000dc40``, id 1), which scales every value read with
#: bit ``0x100``.
COMPONENT_NODE_AT = 0x04
#: The component's initial state word, in the input layer's ``CIS_`` values;
#: -1 keeps the class's own default (``Control.dll:0x10021d86``).  Read here
#: once as an index, hence the second name.
COMPONENT_STATE_AT = 0x18
COMPONENT_INDEX_AT = COMPONENT_STATE_AT
#: A switchable component's state: ``CIS_SWITCHOFF``, ``CIS_SWITCHON``, and
#: ``CIS_SWITCH_INV``, which flips between them (``0x10022c90``).
STATE_OFF = 0x00
STATE_ON = 0x20
STATE_TOGGLE = 0x40
#: The 64 bytes the parser copies into the object whole, which the component's
#: value getter (slot 4, ``0x10021d00``) indexes as sixteen floats by the low
#: byte of a value id -- so ``0x300``-``0x305`` are the first six.
COMPONENT_VALUES_AT = 0x2C
COMPONENT_VALUE_COUNT = 16
#: The building class whose first value is its efficiency, the ``KPD`` that
#: ``Behavior.dll`` totals at ``0x100198e0`` over a building's components of
#: this type and multiplies into research, construction and a mine's output.
EFFICIENCY_TYPE = 26
#: The component's power figure, a float: what a consumer draws a second on
#: top of whatever usage it is given, and what a power store puts out a second
#: at full charge.  Read by every class's flow slot (``0x1002e540``,
#: ``0x100229a0``) and answered as property ``0x500``.
COMPONENT_POWER_AT = 0x20
#: A power store -- ``CICLS_POWERSTOR``, the ``i_pws`` batteries.  Its first
#: value is its capacity, negative on a generator, which never runs dry.
POWER_STORE_TYPE = 19
#: ``CICLS_ENGINE``.  Its first value, times its condition, is what it adds to
#: the machine's drive (``Control.dll:0x1000fca0``, property ``0xe00``).
ENGINE_TYPE = 5
#: What the part weighs, in kg, a float.  ``Control.dll:0x1000fac0`` adds it to
#: the mass of the node the part sits on.  Every internal part but armour
#: carries one, and every ammunition clip; guns and the slots a chassis
#: declares do not.
COMPONENT_MASS_AT = 0x1C
#: ``CICLS_RADAR``.  Values 0-2 are its sensitivities to a target's three
#: signatures, value 3 its range and value 4 how long a scan stays good, in
#: the control clock's milliseconds (``Control.dll:0x10024390``, ``0x10024620``).
RADAR_TYPE = 8
RADAR_RANGE = 3
RADAR_PERIOD = 4
#: ``CICLS_DETECTSHIELD``, the stealth system.  Values 0-2 cut the three
#: signatures, value 3 cuts all three again while camouflage is on, and value 4
#: is what camouflage costs a second (``0x1002bd94``, ``0x100264b0``).
DETECT_SHIELD_TYPE = 10
CAMOUFLAGE = 3
CAMOUFLAGE_POWER = 4
#: ``CICLS_TURRET`` and ``CICLS_CAMERA``.  A robot turret's controller opens
#: with a turret, a radar slot, a camera and a deflector slot.
TURRET_TYPE = 1
CAMERA_TYPE = 4
#: ``CICLS_SIMPLE``: the plain device (``Control.dll:0x10020800``) that turns a
#: machine's wheels, steers, tilts wings and spins rotors.  Flag bytes 1 and 2
#: each pick a ``SIMPLE_SOURCES`` quantity, weighted by the floats at record
#: ``+0x24`` and ``+0x28`` (``SIMPLE_GAINS_AT``); byte 0, when set, is written
#: straight into the value instead (``0x10020900``).
SIMPLE_TYPE = 3
SIMPLE_GAINS_AT = 0x24
#: A simple device's source by selector byte (``0x10020d90``): spin, negated
#: spin, the lean over triple 6 and velocity over the top speed, on x, y and
#: z; 14 is the speed over the top speed as lengths.  A byte below 2 picks
#: nothing.
SIMPLE_SOURCES = {2: ("spin", 0), 3: ("spin", 1), 4: ("spin", 2),
                  5: ("-spin", 0), 6: ("-spin", 1), 7: ("-spin", 2),
                  8: ("lean", 0), 9: ("lean", 1), 10: ("lean", 2),
                  11: ("velocity", 0), 12: ("velocity", 1), 13: ("velocity", 2),
                  14: ("speed", -1)}
#: A turret component's flags: ``MOUNT_UPRIGHT`` on every ground (``e_tur_?t``)
#: turret and clear on its twin hung under a flyer (``e_tur_?b``) -- an upright
#: turret keeps its aim triple as 1 - v (``0x100271c7``, ``0x100276ed``) and a
#: hung one negates the strafe offset (``0x100289b5``); ``MOUNT_HQ`` on the
#: HQ turrets, which the control system answers as query 13
#: (``0x1002b7bb``) and ``iron3d.dll`` reads as component value 117.
MOUNT_UPRIGHT = 0x04000000
MOUNT_HQ = 0x08000000
#: The camera's shake (``0x100234c0``), which the machine tick feeds with the
#: change in the machine's velocity over each step (``0x1000c6e7``).  A jolt
#: whose squared size reaches ``SHAKE_JOLT_SQ`` blends the offset toward half
#: of it over ``SHAKE_BLEND_S``; the next smaller one lets it ring down as
#: ``offset * cos(pi/2 * SHAKE_FREQUENCY * t) / (t + 1) ** SHAKE_DECAY``.  The
#: eye moves by the offset clamped to unit length, times ``SHAKE_SCALE``.
SHAKE_DECAY = 3.0          # camera +0xa4
SHAKE_FREQUENCY = 3.0      # camera +0xa8
SHAKE_BLEND_S = 2.5        # camera +0xac
SHAKE_JOLT_SQ = 0.3
SHAKE_SCALE = 0.02


def shake_ring(offset: tuple[float, float, float], seconds: float) -> tuple[float, float, float]:
    """The camera's shake offset ``seconds`` after it started ringing down."""
    k = math.cos(math.pi / 2 * SHAKE_FREQUENCY * seconds) / (seconds + 1.0) ** SHAKE_DECAY
    return (offset[0] * k, offset[1] * k, offset[2] * k)


#: ``CICLS_DOOR`` and ``CICLS_COMPUTER``.  ``Terrain.dll``'s building files
#: its controller's items by these (``0x100583a2``), and runs its first
#: computer as the control pod (``0x10057550``).
DOOR_TYPE = 12
COMPUTER_TYPE = 13
#: ``CICLS_MULTIGUN``, the gun, and type 30, the builder's beam, which the same
#: class builds (``Control.dll:0x100294c0``).  A gun reads four values: its
#: magazine in rounds (-1 unlimited), the capacitor it keeps charged, the
#: energy a shot takes from it, and the ms between shots.
GUN_TYPE = 2
BUILDER_TYPE = 30
GUN_MAGAZINE = 0
GUN_CAPACITOR = 1
GUN_SHOT_ENERGY = 2
GUN_INTERVAL = 3
UNLIMITED = -1
#: A projectile's seeker: value 0 its cone's half-angle in radians, value 1
#: the distance it follows a target within (``0x100247a0``, ``0x100247c0``).
SEEKER_TYPE = 17
#: The component record's flags word, read per class: on a gun ``SALVO`` fires
#: every barrel at once instead of the next in turn (``0x10029fcc``); on a
#: turret see ``MOUNT_*``.
COMPONENT_FLAGS_AT = 0x08
SALVO = 0x2000000
#: A section-5 group of the controller a gun runs when a barrel starts its
#: stroke (``Control.dll:0x1002a1df``), or -1.  34 guns set one.
COMPONENT_GROUP_AT = 0x0C
#: A component's 4-byte entries are **section-2 channel indices**, rebased by
#: the part's first channel and kept as the channels the component drives
#: (``Control.dll:0x10021de7``): a gun's barrels, a turret's yaw and pitch, a
#: wheel.  All 885 shipped entries name a channel of their controller.
#:
#: The factory's generic device (classes 3, 6, 7, 11-14, 16, 18, 20, 22-25,
#: 28, 29; ``0x10020800``) takes its rate from the machine's motion: the
#: flags word's bytes 1 and 2 select two inputs, weighted by the floats at
#: ``+0x24`` and ``+0x28`` (``0x10020985``); a non-zero byte 0 selects one
#: that sets its progress outright.  A selector ``s`` from 2 to 13 is
#: ``DEVICE_INPUT_KINDS[(s - 2) // 3]`` on axis ``(s - 2) % 3``; 14 is the
#: speed's length over the top speed's; anything else is the default, 1 for
#: the first input and 0 for the second (``0x10020d90``, ``0x10020ea0``).
COMPONENT_WEIGHTS_AT = 0x24
DEVICE_INPUT_KINDS = ("spin", "-spin", "lean", "speed")
DEVICE_INPUT_FIRST = 2
DEVICE_INPUT_LENGTH = 14
#: The hero turret's weapon arms, one per gun; built as the factory's generic
#: device (``0x1002d6ec``).  No other controller has any.
ARM_TYPE = 24
#: A barrel stroke: the channel heads for 0.5, the round leaves, the channel
#: heads for 1.0, then snaps back (``0x1002a190``) -- a whole value at the
#: channel's rate, before the gun's interval starts.
STROKE_MS = 1000.0
#: ``CICLS_FIGHTSHIELD``, the shield generator.  Values: a sector's maximum, the
#: recharge a second, the charge a point costs (``Control.dll:0x100257b0``).
FIGHT_SHIELD_TYPE = 9
SHIELD_SECTORS = 6
#: ``CICLS_REPAIRSYS``.  Values: the points it restores a second, the charge
#: a point costs (``0x10022bb0``).  It repairs only its own object, starts
#: switched off, and values 2-15 are zero on every record: it has no reach.
REPAIR_TYPE = 15
#: The deflector, ``i_def``; ``CICLS`` has no name for it.  Values 0-5: how
#: much of each shield sector stops damage (``0x1002ca30``).  The bubble is up
#: only while a fight shield and a deflector are both on and intact
#: (``0x1002c500``).
DEFLECTOR_TYPE = 21
#: Armour, ``i_arm``; ``CICLS`` has no name for it either.  Values: a rating,
#: then a linear and a square factor -- a hit of D becomes
#: ``min(D, linear x D + square x D^2)`` (``0x10010030``).  The first value is
#: also a weight per unit of area, summed over every node (``0x1000fbac``).
ARMOUR_TYPE = 27
#: A chassis's labelled slots: the label's family, and the class it is on.  A
#: part fits a slot when its ``objects.rlb`` name starts with the label, and
#: the label's last letter is the size of part it takes.
SLOT_FAMILIES = {"i_eng": ENGINE_TYPE, "i_pws": POWER_STORE_TYPE,
                 "i_fsh": FIGHT_SHIELD_TYPE, "i_dsh": DETECT_SHIELD_TYPE,
                 "i_rps": REPAIR_TYPE, "i_arm": ARMOUR_TYPE}
#: The classes a control system keeps one pointer to, overwritten by every
#: record it builds (``Control.dll:0x1002d53c``..``0x1002d604``), and armour,
#: whose numbers every class-27 record refreshes (``0x1002d7b6``): the last in
#: load order wins.  Class 17 has no ``CICLS_`` name.
SINGLE_POINTER_CLASSES = (TURRET_TYPE, RADAR_TYPE, FIGHT_SHIELD_TYPE, 17,
                          DEFLECTOR_TYPE, ARMOUR_TYPE)

#: A section-1 state: ``SECTION1_RECORD`` bytes, then ``counts[1]`` 16-byte
#: conditions.  Bits 0-2 of the flags switch on the velocity box per axis and
#: bits 4-6 the spin box; a state applies while the machine's velocity and spin
#: lie inside the boxes it switches on (``Control.dll:0x10001000``).
STATE_FLAGS_AT = 0x00
STATE_VELOCITY_AT = 0x24     # min xyz, then max xyz at +0x30
STATE_SPIN_AT = 0x3C         # min xyz, then max xyz at +0x48
#: The state's engine factor.  The control system copies the current state to
#: ``+0x100`` (``0x1000c36f``), so this is the ``+0x154`` the engine draw
#: multiplies by (``0x100266e1``).
STATE_ENGINE_AT = 0x54
#: +0x04 the state's mode bits; +0x0c and +0x14 its two frame pairs, A and B;
#: +0x1c the blend base toward B; +0x20 a fixed step length in ms, or 0.
STATE_MODE_AT = 0x04
STATE_PAIR_A_AT = 0x0C
STATE_PAIR_B_AT = 0x14
STATE_BLEND_AT = 0x1C
STATE_LENGTH_AT = 0x20
#: An anchor the planner chooses among (``0x100051c0``); a state that moves
#: the body by velocity x step (``0x10015920``); a fixed step that integrates
#: nothing (``0x100053ab``); a step jittered by up to 12.5% (``0x100057d6``).
STATE_ANCHOR = 0x1
STATE_BY_VELOCITY = 0x10000
STATE_FIXED = 0x100000
STATE_JITTER = 0x1000000
#: The body follows the ground on its contact points: it falls under the
#: world's gravity until the nearest flagged contact reaches its ground point
#: (``0x1001b3bd``, ``0x10015d60``).  Without it the sphere is only ever lifted
#: out of the ground.  Set on every state of the controllers that declare
#: contact points, and on no other.
STATE_GROUND_CONTACTS = 0x4
#: Unbounded motion: no clamp to the live top speed (``0x1001461e``,
#: ``0x100158c4``), no slope brake (``0x10015681``) and no turn-rate clamp
#: (``0x10014c02``).  Rounds, trees and stones carry it; no chassis does.
STATE_UNBOUNDED = 0x40000
#: How the hull rights itself (``Control.dll:0x1000c3a2``): toward the world's
#: up, toward the vector at control ``+0x348``, or not at all.  Pitch is taken
#: when ``0x10`` or ``0x40`` is set, roll when ``0x20`` or ``0x80`` is.  Triple 5
#: (``TRIPLE_SETTLE``) is the share of the tilt taken back each step.
STATE_RIGHT_UP = 0xC0
STATE_RIGHT_TO_VECTOR = 0x30
STATE_RIGHT_PITCH = 0x50
STATE_RIGHT_ROLL = 0xA0
STATE_NO_RIGHTING = 0x400000
#: A step is held to this many seconds, and a fixed-length velocity state is
#: cut so that speed x step stays within ``FIXED_STEP_REACH`` (``0x1000550e``).
STEP_MIN = 0.01
STEP_MAX = 5.0
FIXED_STEP_REACH = 5.0
#: A transition cost at or above this is no edge.
NO_EDGE = 1_000_000.0
#: The section-5 group entering the state runs (``0x1000c37c``), and the request
#: code the state waits for, -1 for any (``0x10001140``).  A building's states
#: answer the construction sphere's codes (docs/32-builder.md).
STATE_ACTIONS_AT = 0x90
STATE_REQUEST_AT = 0x98
#: Bytes +0x08..+0x0a pick what leans the body about x, y and z
#: (``0x10014e8d``): 1-3 the turn about x, y or z this step over dt x the
#: authored turn rate, 4-6 the velocity along x, y or z over the authored top
#: speed, 8-10 the last step's change of velocity (``0x10015906``) over the
#: authored acceleration; ``LEAN_NEGATE`` flips it.  The lean heads for that fraction of triple 6 at
#: the live turn rate, is held within triple 6, and turns the body's tilt
#: (``0x100151ae``, ``0x10015251``).
STATE_LEAN_AT = 0x08
LEAN_NEGATE = 0x80
LEAN_TURN = (1, 2, 3)
LEAN_VELOCITY = (4, 5, 6)
LEAN_ACCELERATION = (8, 9, 10)
#: The same selectors as (quantity, axis), one byte per axis -- pitch, roll,
#: yaw (``Control.dll:0x10014e46``): the step's turn over the authored turn
#: rate x dt, velocity over the authored top speed, the step's change of
#: velocity over the authored acceleration.  0 and 7 lean nothing, and the
#: lean on that axis is the source's fraction times triple 6 (``TRIPLE_LEAN``).
LEAN_SOURCES = {1: ("turn", 0), 2: ("turn", 1), 3: ("turn", 2),
                4: ("velocity", 0), 5: ("velocity", 1), 6: ("velocity", 2),
                8: ("acceleration", 0), 9: ("acceleration", 1), 10: ("acceleration", 2)}
#: A state's 16-byte conditions are its **contacts**, one per foot, wheel or
#: leg, the same points in every state of a controller:
#:
#: * +0 a control point of the object's ``.cpt``, placed on the node its third
#:   slot names (``0x1001ac0d``);
#: * +4 flags.  ``NEEDS_INTACT`` and ``NEEDS_DESTROYED`` make the state apply
#:   only while that point's node is intact, or destroyed (``0x10001107``,
#:   ``0x1001ac39``) -- a walker's limping states.  ``CONTACT_FALLBACK`` moves a
#:   destroyed point on to the next control point whose node is intact.
#:   ``CONTACT_SUPPORT`` counts it in the body's ground height, and
#:   ``CONTACT_PLACE`` puts the node on the ground (``0x1001b00a``,
#:   ``0x1001afcb``).  ``CONTACT_PLANTED`` is set at load on the states whose
#:   last pose holds the point within ``PLANTED_WITHIN`` of its rest height
#:   (``0x1001a328``); ``0x10`` and ``0x20`` ask the load to set
#:   ``CONTACT_SUPPORT`` and ``CONTACT_PLACE`` from the pose as well.
#: * +8 the section-5 group run when the point lands: when a planted state
#:   follows one where it was not (``0x1001b0a5``) -- footsteps.
#: * +12 zero in the file; the load writes the planted gap there.
CONTACT_SUPPORT = 0x1
CONTACT_PLACE = 0x2
CONTACT_FALLBACK = 0x4
NEEDS_INTACT = 0x100
NEEDS_DESTROYED = 0x200
CONTACT_PLANTED = 0x1000
PLANTED_WITHIN = 0.1

#: Each class's power channel, by type id -- ``Control.dll:0x1003ccc8``.
POWER_CHANNEL = (0, 4, 4, 0, 2, 3, 0, 0, 2, 5, 5, 0, 0, 0, 1, 0,
                 3, 2, 0, 1, 3, 5, 4, 0, 4, 0, 3, 5, 0, 0, 4)
#: The order a controller's tick (``0x1002d340``) serves the channels in.
#: Each group gets ``min(1, what is left / what it wants)``; the stores, on
#: channel 1, come last and drain by the share that was used.
POWER_ORDER = ((3,), (0,), (2, 5), (4,), (1,))
#: What the control system answers through ``IDeviceManager`` (interface
#: 0x204, the sub-object at +0xc) slot 4, by id: a 16-way table
#: (``Control.dll:0x1002b410``, table ``0x1002b9e8``).  This is the table once
#: taken for a dispatch of ``MCMD_`` 1-16.  ``Behavior.dll`` asks for 1, 2, 5, 6
#: and 7, ``ArealMap.dll`` for 6, ``iron3d.dll`` for 2, and a fired round's
#: gun for 10-12.  "Guns" figures are ``+0x174`` (the round's property 0x35)
#: times 1000 over the gun's interval, at least 1 ms.
DEVICE_QUERIES = {
    1: "battery fill",
    2: "battery capacity",
    3: "channel 0/2/5 demand a second",
    4: "channel 0/3/4 demand a second",
    5: "guns not in state 1, a second",
    6: "all guns, a second",
    7: "fight shield: mean sector fill",
    8: "radar value 3, range",
    9: "radar value 4, period",
    10: "seeker value 1",
    11: "seeker value 0",
    12: "seeker value 2",
    13: "turret is an HQ turret",
    14: "shield fill x deflector values 0-5 x shield value 0",
    15: "deflector values 0-5 x shield value 0",
    16: "deflector value 0 x shield value 0",
}
#: The smallest a controller can be.  The 128-byte frame and the 84-byte block
#: are **not** adjacent in general -- sections 1, 2 and 4 lie between them --
#: but a member with none of those is exactly the two, which is why 212 is the
#: floor and why the block looked like a trailer before the layout was read.
FRAME_SIZE = HEADER_SIZE + BLOCK_SIZE

#: The engine's "not set" fill, the same byte the ``MAT0`` loader treats as
#: absent.  The six section-less members are 0xFF from +128 to the end.
UNSET = 0xFF

#: Where the five section counts sit.  Their strides are not fixed; only the
#: fact that all five zero means a 212-byte file is established.
COUNT_AT = (0, 4, 8, 12, 16)

#: The six float triples, in order.  Each is a per-axis ``(x, y, z)``: the
#: three components are equal on 440, 521, 438, 484, 504 and 502 of the 531.
TRIPLE_AT = (20, 32, 44, 56, 68, 80)
#: The triples ``Control.dll:0x1000fca0`` turns into live limits, by index.
#: Acceleration is doubled live; top speed is m/s with y forward, capped at the
#: authored value; the turn rate scales with the engines and the load.
TRIPLE_ACCELERATION = 0
TRIPLE_TOP_SPEED = 2
TRIPLE_TURN = 3
#: +32: nothing in ``Control.dll`` reaches +32..+40 in either copy of the
#: block, and 518 of 531 leave it zero; its forward component is the AI
#: walker's speed floor (``Behavior.dll:0x1003bed0``).
TRIPLE_IDLE = 1
TRIPLE_UNREAD = TRIPLE_IDLE
#: +68: the share of the hull's tilt righted each step (``0x10014b0b``),
#: multiplied into the spin integrator (``0x10014b15``).
TRIPLE_SETTLE = 4
#: +80, triple 6: the most the hull leans, in radians, per axis
#: (``STATE_LEAN_AT``, ``0x10014f80``).
TRIPLE_LEAN = 5
#: The ``mode`` that brakes on a slope steeper than ``cone``
#: (``Control.dll:0x100157ac``).
SLOPE_MODE = 2
#: The stat panel shows top speed times this as km/h (``iron3d.dll:0x1006f3c7``).
KMH_PER_MS = 3.6

#: The file's own two-pi, on all three components of the triple at +56 on 364
#: members and of the triple at +80 on 422.  A limit of a whole turn is the
#: same as no limit, which is why it reads as a default.  It is **6.28**, a
#: decimal somebody typed rather than the real constant, and this is that
#: decimal as a ``float32`` -- the exact bits the constructor writes.
FULL_TURN = 6.28000020980835

#: The engine's own half-turn-of-a-cone at +112, on 509 of the 531.  Note it is
#: **1.57079**, not ``pi/2``: like ``FULL_TURN`` it is a decimal somebody typed,
#: and the constructor writes exactly these bits.
HALF_CONE = 1.5707900524139404

#: ``FLT_MAX`` at +124 on 502, and -1.0 at +108 on 465 and +120 on 433.  Both
#: read as "unbounded"; which one a field uses follows its sign convention.
FLT_MAX = 3.4028234663852886e38
NO_LIMIT = -1.0

#: Where the frame's float block lands in the live controller.  ``Control.dll``
#: builds a 0x668- or 0x670-byte object whose initialiser at ``0x10006689``
#: writes a default into every one of these slots, and **the value it writes is
#: the commonest value in the file** for all 27 of them.  So the frame from +20
#: on is that object's parameter block: object offset = file offset + 0x45c.
#: The five counts below +20 are not part of it -- the object keeps pointers
#: there.
FIELD_BASE = 0x45C

#: File offset -> the default the constructor writes.  A shipped controller
#: that leaves a slot alone is carrying the engine's own compiled-in value.
DEFAULTS: dict[int, float | int] = {
    20: 2.5, 24: 2.5, 28: 2.5,
    32: 0.0, 36: 0.0, 40: 0.0,
    44: 0.0, 48: 0.0, 52: 0.0,
    56: FULL_TURN, 60: FULL_TURN, 64: FULL_TURN,
    68: 1.0, 72: 1.0, 76: 1.0,
    80: FULL_TURN, 84: FULL_TURN, 88: FULL_TURN,
    92: 0, 96: 0.0, 100: 0.0, 104: 0,
    108: NO_LIMIT, 112: HALF_CONE, 116: 0,
    120: NO_LIMIT, 124: FLT_MAX,
}

#: The slots ``DEFAULTS`` holds as integers rather than floats.
DEFAULT_INTS = (92, 104, 116)

#: The sections after the frame, recovered from the loader at ``0x10008b10``.
#: It reads the five counts one at a time, copies the parameter block, and then
#: walks the body in this order.
#:
#: * **section 1** -- ``counts[0]`` records of ``SECTION1_RECORD + 16 *
#:   counts[1]`` bytes, then ``counts[0] ** 2`` floats, a transition table.  The engine computes the
#:   whole span as ``A * (4 * A + 16 * B + 156)`` when it skips the section,
#:   which is the same arithmetic.
#: * **section 2** -- ``counts[2]`` records of 36 bytes.
#: * **section 4** -- ``counts[3]`` records whose first int32 is a **type id
#:   from 1 to 30**.  Each is parsed by the class the factory at ``0x1002d4b0``
#:   builds for that id, through its own vtable slot, so the sizes live in 30
#:   different classes and are **not** read here.
#: * a fixed **84-byte block**, copied into the object.
#: * **section 5** -- ``counts[4]`` groups, each an int32 ``n`` followed by
#:   ``n`` reference records.
#: A section-5 record: **nine int32, then** two 32-byte NUL-padded name fields.
#: The order was the other way round in this reader until the sections were
#: walked properly -- anchoring on the names put the ints where the names are.
REFERENCE_STRIDE = 100
NAME_FIELD = 32
REFERENCE_INTS = 9
#: Where the ``(archive, member)`` pair starts inside the record.
REFERENCE_NAME_AT = 36


class ControlFormatError(ValueError):
    pass


@dataclass(frozen=True)
class Reference:
    """One ``(archive, member)`` pair inside a controller, and its nine ints.

    A record is one **action** in a group the controller runs in order
    (``Control.dll:0x10002800``): int 0 carries flags, ints 1 and 2 its
    condition (``REF_*``), int 3 the action (``ACT_*``) and ints 4-7 its
    arguments.  Only the effect actions 3, 4 and 5 and the explosion action 27
    name anything.  Int 8 is not read.
    """

    resource: ResourceRef
    values: tuple[int, ...]
    #: Byte offset of the record within the member, for anyone extending this.
    offset: int
    #: The section-5 group the record sits in, which the block's entries and a
    #: state's action group (+0x90) index.  -1 when not known.
    group: int = -1

    @property
    def action(self) -> int:
        return self.values[ACTION_AT]

    @property
    def args(self) -> tuple[int, ...]:
        """v4 to v7."""
        return tuple(self.values[ACTION_AT + 1:ACTION_AT + 5])

    @property
    def flags(self) -> int:
        return self.values[0] & 0xFFFFFFFF

    @property
    def mask(self) -> int:
        """The condition bytes the record looks at, one bit each."""
        return self.values[MASK_AT] & 0xFFFF

    @property
    def inverted(self) -> int:
        """The masked conditions that hold on a clear byte rather than a set one."""
        return self.values[INVERT_AT] & 0xFFFF

    def holds(self, conditions) -> bool:
        """Whether the record's own condition lets it run.

        ``conditions`` is the controller's sixteen condition bytes, as truth
        values.  A record flagged neither ``REF_ANY`` nor ``REF_ALL`` always
        runs; ``REF_ANY`` wins over ``REF_ALL`` when both are set.
        """
        if not self.flags & (REF_ANY | REF_ALL):
            return True
        met = [bool(conditions[i]) != bool(self.inverted >> i & 1)
               for i in range(CONDITIONS) if self.mask >> i & 1]
        return any(met) if self.flags & REF_ANY else all(met)


def run_group(records, conditions) -> list[Reference]:
    """The records of one group that run, in order, given the condition bytes.

    A ``REF_OPEN`` record opens a run unless one is open; the next ``REF_ELSE``
    record closes it, and runs on its own condition or when no record since
    the run opened has run.
    """
    out: list[Reference] = []
    inside = ran = False
    for record in records:
        forced = False
        if record.flags & REF_OPEN and not inside:
            inside, ran = True, False
        if record.flags & REF_ELSE:
            forced = inside and not ran
            inside = ran = False
        if forced or record.holds(conditions):
            out.append(record)
            ran = True
    return out


@dataclass(frozen=True)
class Channel:
    """One section-2 record: an animated, rate-limited value from 0 to 1.

    ``Control.dll``'s turret steps the value toward its target by at most
    ``rate`` a second (``0x100289f0``), the short way round on a wrapping
    channel; ``span`` is the radians 0..1 covers.  On every turret the yaw
    channel spans 2 pi over four frames and names ``TurretDirect``, and the
    pitch channel names ``TargetDirect``.
    """

    #: +0: the mesh node the channel animates, on a segment of its own.
    node: int
    #: +4 and +8: the mesh frames the value plays from 0 to 1.
    first: float
    last: float
    #: +12: the value it starts at.  0.5 looks ahead on a yaw channel.
    initial: float
    #: +16: a second control point or -1 -- the camera channel's
    #: ``CameraCenter``.
    origin: int
    #: +20: a control point of the same-stem ``.cpt``, or -1.
    point: int
    #: +24: value per second.
    rate: float
    #: +28: radians from value 0 to 1.
    span: float
    #: +32: ``CHANNEL_*``; 3, wrapping and inverted, on every turret yaw.
    flags: int

    @property
    def stroke_ms(self) -> float:
        """How long a barrel on this channel takes to fire, 0 to 0.5 to 1."""
        return STROKE_MS / self.rate if self.rate > 0 else 0.0

    def frame(self, value: float) -> float:
        """The frame the node plays at ``value``.

        ``0x10009950`` hands the node the pair and its value, and
        ``AniMesh.dll:0x10008b30`` lerps across it.  Whether the inversion
        applies before or after a wrap is not established; here it follows.
        """
        v = value % 1.0 if self.flags & CHANNEL_WRAP else min(1.0, max(0.0, value))
        if self.flags & CHANNEL_INVERT:
            v = 1.0 - v
        return self.first + v * (self.last - self.first)


@dataclass(frozen=True)
class Contact:
    """One of a state's 16-byte conditions: a foot, wheel or leg (``CONTACT_*``)."""

    #: +0: a control point of the object's ``.cpt``.
    point: int
    #: +4: ``CONTACT_*``, ``NEEDS_INTACT``, ``NEEDS_DESTROYED``.
    flags: int
    #: +8: the section-5 group run when the point lands, or -1.
    group: int = -1

    def allows(self, intact: bool) -> bool:
        """Whether this condition lets its state apply, given the point's node."""
        if self.flags & NEEDS_INTACT and not intact:
            return False
        return not (self.flags & NEEDS_DESTROYED and intact)


@dataclass(frozen=True)
class State:
    """One section-1 record: an animation state the controller moves between."""

    #: The flags at +0: which axes of the two boxes are switched on.
    flags: int
    #: The velocity box, ``(min xyz, max xyz)``, in m/s with y forward.
    velocity: tuple[tuple[float, float, float], tuple[float, float, float]]
    #: The spin box, ``(min xyz, max xyz)``, in rad/s with z yaw.
    spin: tuple[tuple[float, float, float], tuple[float, float, float]]
    #: What the engine draw is multiplied by while this state is current.
    engine: float
    #: The section-5 group entering it runs, and the request code it waits for.
    actions: int = -1
    request: int = -1
    #: +0x04: ``STATE_ANCHOR``, ``STATE_BY_VELOCITY``, ``STATE_FIXED``,
    #: ``STATE_JITTER``.
    mode: int = 0
    #: The two frame pairs a step plays, first and last, and the blend base
    #: toward B.
    pair_a: tuple[float, float] = (0.0, 0.0)
    pair_b: tuple[float, float] = (0.0, 0.0)
    blend: float = 1.0
    #: A fixed step length in ms; 0 lets speed and stride set it.
    length: float = 0.0
    #: The contacts, one per ``counts[1]``, the same points in every state.
    contacts: tuple[Contact, ...] = ()
    #: +0x08..+0x0a: what leans the body about x, y and z (``STATE_LEAN_AT``).
    lean: tuple[int, int, int] = (0, 0, 0)
    #: +0x08..+0x0b as one little-endian dword: the three lean selectors, pitch
    #: in the low byte, and a fourth byte that is 0 throughout.
    lean_word: int = 0

    @property
    def anchor(self) -> bool:
        return bool(self.mode & STATE_ANCHOR)

    def lean_source(self, axis: int) -> tuple[str, int, bool] | None:
        """What leans the hull on ``axis`` (0 pitch, 1 roll, 2 yaw) in this state.

        ``(quantity, source axis, negated)`` from ``LEAN_SOURCES``, or None where
        the selector leans nothing.
        """
        byte = self.lean[axis]
        source = LEAN_SOURCES.get(byte & ~LEAN_NEGATE & 0xFF)
        return (*source, bool(byte & LEAN_NEGATE)) if source else None

    @property
    def by_velocity(self) -> bool:
        return bool(self.mode & STATE_BY_VELOCITY)

    def allows(self, intact) -> bool:
        """Whether the contacts let this state apply.

        ``intact`` says, per contact, whether its point's node is intact; the
        velocity and spin boxes and the request code are tested besides
        (``Control.dll:0x10001000``).
        """
        return all(c.allows(ok) for c, ok in zip(self.contacts, intact, strict=True))

    @property
    def blends(self) -> bool:
        """Whether a step of this state weighs its two pairs by speed.

        Only a state that is neither velocity-driven, fixed, nor of a set
        length computes a weight (``Control.dll:0x1000555b``); every other
        state plays with a weight of 1.
        """
        return not self.mode & (STATE_BY_VELOCITY | STATE_FIXED) and self.length == 0

    @property
    def blend_floor(self) -> float:
        """lo: the smaller of the velocity box's largest ``|min|`` and ``|max|``.

        Taken over all three axes, switched on or not; an axis left off holds
        ``FLT_MAX`` and would make lo infinite.
        """
        lo, hi = self.velocity
        return min(max(abs(v) for v in lo), max(abs(v) for v in hi))

    @property
    def blend_divisor(self) -> float:
        """D: the largest ``| |max| - |min| |`` over the velocity box's axes.

        ``0x100055d4``: the absolute bounds, their difference and its absolute
        value, then the largest component.  An axis left off gives 0; a box
        that straddles zero gives less than its span.
        """
        lo, hi = self.velocity
        return max(abs(abs(h) - abs(m)) for m, h in zip(lo, hi, strict=True))

    def blend_weight(self, speed: float) -> float:
        """q, the weight toward pair B at a speed (the largest local component).

        1 unless the state ``blends``, the machine moves (speed above 1e-9,
        ``0x1000553e``) and D is positive; otherwise ``p + (1 - p) * blend``
        with ``p = (speed - lo) / D`` held to 0..1 (``0x100057a3``).
        """
        divisor = self.blend_divisor
        if not self.blends or speed <= 1e-9 or divisor <= 0:
            return 1.0
        p = min(1.0, max(0.0, (speed - self.blend_floor) / divisor))
        return p + (1.0 - p) * self.blend


@dataclass(frozen=True)
class Controller:
    """The 212-byte frame at the head of a ``.ctl`` member, and its sections.

    The triples are per-axis.  +20 is the acceleration, +44 the top speed,
    +56 the turn rate, +68 how fast the hull rights itself and +80 the most it
    leans (``TRIPLE_*``); +32 is never read by ``Control.dll``, and its forward
    component is the AI's speed floor.
    Every one of the 12744 float reads across the block's 24 float slots is
    finite.
    """

    #: The five section counts at +0..+16.  All five are zero on five members,
    #: and those members are exactly 212 bytes.
    counts: tuple[int, int, int, int, int]
    #: The six ``(x, y, z)`` triples at +20, +32, +44, +56, +68 and +80.
    triples: tuple[tuple[float, float, float], ...]
    #: +92: 0 on 324, then 5000, 1000, 2000 -- a round count, not a float.
    scale: int
    #: +96 and +100: zero on 512.
    pair: tuple[float, float]
    #: +104: 0, 2 or 3.  2 brakes on slopes steeper than ``cone``; 3 takes a
    #: branch of its own that is not read.
    mode: int
    #: +108 and +120: -1.0 on 465 and 433, otherwise a positive bound.
    bounds: tuple[float, float]
    #: +112: the steepest slope a mode-2 machine climbs freely, in radians.
    #: The default 1.57079 on 509; 0.6 on every mode-2 controller.
    cone: float
    #: +116: 0 on 342, then 3, 4, 16.  Unresolved.
    flags: int
    #: +124: the most the machine can carry, in kg; the stat panel shows it
    #: times 0.001 as tonnes.  ``FLT_MAX`` on 502 that carry nothing.
    payload: float
    #: True when +128 to the end is the unset fill.
    bare: bool
    #: Section 4: what this controller is made of, and what each part emits.
    components: tuple[Component, ...]
    #: Section 5's groups: 1432 of the 1651 named references live here, the
    #: other 219 being components' own resources.  In the 136 members that
    #: carry no components at all, none of section 5's records is named.
    references: tuple[Reference, ...]
    #: Section 1: the animation states.
    states: tuple[State, ...] = ()
    #: Section 2's control-point words, one a record: an index into the
    #: same-stem ``.cpt``, or -1.  A gun's entries index these records: its
    #: barrels.
    points: tuple[int, ...] = ()
    #: Section 2 whole: the channels a component's entries name -- a gun's
    #: barrels, a turret's yaw and pitch, a camera's eye.
    channels: tuple[Channel, ...] = ()
    #: Section 1's transition table, ``A x A`` floats, the row the destination.
    costs: tuple[float, ...] = ()
    #: The 84-byte block: 21 section-5 group indices, ``NO_GROUP`` for none.
    groups: tuple[int, ...] = ()

    def cost(self, to: int, frm: int) -> float:
        """What moving from state ``frm`` to state ``to`` costs.

        The row is the destination: on every zero-cost edge the source's pair
        B ends on the frame the destination's starts.
        """
        return self.costs[to * len(self.states) + frm]

    def transition_factor(self, to: int, frm: int) -> float:
        """What the loader multiplies the file's cost from ``frm`` to ``to`` by.

        ``Control.dll:0x10001790``: one, plus the largest ``|centre - min|``
        over the velocity axes ``frm`` switches on -- the centre of ``to``'s
        box, the minimum of ``frm``'s -- plus the same over the spin axes.  It
        is not the gap between the two boxes: a state's cost to itself grows
        by half its own span.
        """
        dest, src = self.states[to], self.states[frm]

        def largest(dest_box, src_box, shift):
            out = 0.0
            for axis in range(3):
                if src.flags >> (shift + axis) & 1:
                    centre = (dest_box[0][axis] + dest_box[1][axis]) / 2
                    out = max(out, abs(centre - src_box[0][axis]))
            return out

        return 1.0 + largest(dest.velocity, src.velocity, 0) + largest(dest.spin, src.spin, 4)

    def live_cost(self, to: int, frm: int) -> float:
        """The cost the planner uses: the file's, scaled at load (``0x10001790``).

        It is the file's cost times one plus two gaps.  The velocity gap is the
        largest distance, over the axes the source state switches on (+0x00 bits
        0-2), from the centre of the destination's velocity box to the minimum of
        the source's; the spin gap is the same over bits 4-6
        (``0x10001af0`` takes the larger of three).  See ``transition_factor``.
        """
        return self.cost(to, frm) * self.transition_factor(to, frm)

    def path(self, frm: int, to: int, live: bool = True) -> list[int] | None:
        """The cheapest run of states from ``frm`` to ``to``, without ``frm``.

        Dijkstra as the planner runs it (``0x100019d0``), over the costs as the
        loader leaves them (``live_cost``); ``live=False`` walks the file's
        own numbers instead.  A cost at or above ``NO_EDGE`` is no edge either
        way.
        """
        weigh = self.live_cost if live else self.cost
        best = {frm: 0.0}
        back: dict[int, int] = {}
        queue = [(0.0, frm)]
        while queue:
            cost, state = heapq.heappop(queue)
            if state == to and state != frm:
                out = []
                while state != frm:
                    out.append(state)
                    state = back[state]
                return out[::-1]
            if cost > best.get(state, float("inf")):
                continue
            for nxt in range(len(self.states)):
                if nxt == state or self.cost(nxt, state) >= NO_EDGE:
                    continue
                step = weigh(nxt, state)
                if step >= NO_EDGE:
                    continue
                if cost + step < best.get(nxt, float("inf")):
                    best[nxt] = cost + step
                    back[nxt] = state
                    heapq.heappush(queue, (cost + step, nxt))
        return None

    def group(self, entry: int) -> list[Reference]:
        """The records of the group block entry ``entry`` names, in order."""
        index = self.groups[entry] if self.groups else NO_GROUP
        return [] if index == NO_GROUP else [r for r in self.references if r.group == index]

    @property
    def load_group(self) -> int:
        """The group the loader runs once (``Control.dll:0x10009408``)."""
        return self.groups[0] if self.groups else NO_GROUP

    @property
    def surface_groups(self) -> tuple[int, ...]:
        """One group per ground surface id 0..10, run when the id changes."""
        return self.groups[SURFACE_GROUPS_AT:] or (NO_GROUP,) * SURFACES

    @property
    def named(self) -> list[ResourceRef]:
        """Everything this controller names, from its parts and its groups."""
        out = [c.resource for c in self.components if c.resource]
        out += [r.resource for r in self.references if r.resource]
        return out

    @property
    def sections(self) -> int:
        """How many sections the counts ask for, across all five kinds."""
        return sum(self.counts)


@dataclass(frozen=True)
class Component:
    """One section-4 record: a class id, what it emits, and its tail.

    The type id picks one of 30 classes at the factory in ``Control.dll``.
    Thirteen sizes of object come out of it, but every class parses its record
    with the same code, so the record is one shape:

    ``COMPONENT_FIXED`` bytes, then ``len(entries)`` int32, then a length and
    that many bytes of text where the length is not zero.
    """

    type_id: int
    resource: ResourceRef
    #: The int32 at +0x18, the initial state word, or None where -1 keeps the
    #: class's default.  See ``state``.
    index: int | None
    #: The 4-byte entries after the fixed part: the section-2 channels the
    #: component drives.
    entries: tuple[int, ...]
    #: The length-prefixed string at the end, empty where the length is zero.
    label: str
    #: Where the record starts, and how long it is.
    offset: int
    size: int
    #: The sixteen floats at ``COMPONENT_VALUES_AT``.
    values: tuple[float, ...] = ()
    #: The float at ``COMPONENT_POWER_AT``.
    power: float = 0.0
    #: The int at ``COMPONENT_NODE_AT``: an index into the object's ``.ndp``.
    node: int = 0
    #: The float at ``COMPONENT_MASS_AT``: what the part weighs, in kg.
    mass: float = 0.0
    #: The word at ``COMPONENT_FLAGS_AT``.
    flags: int = 0
    #: The group at ``COMPONENT_GROUP_AT``, ``NO_GROUP`` for none.
    group: int = -1
    #: The two floats at ``COMPONENT_WEIGHTS_AT``: a generic device's input
    #: weights.
    weights: tuple[float, float] = (0.0, 0.0)

    @property
    def inputs(self) -> tuple[int, int, int]:
        """The flags word's three low bytes: a generic device's input selectors."""
        return (self.flags & 0xFF, self.flags >> 8 & 0xFF, self.flags >> 16 & 0xFF)

    @property
    def slot(self) -> str | None:
        """The part family and size this record is a slot for, ``i_eng_b``, or None."""
        return self.label.lower() if self.label[:5].lower() in SLOT_FAMILIES else None

    @property
    def state(self) -> int | None:
        """The state the component starts in, or None for its class's default."""
        return self.index

    @property
    def channel(self) -> int:
        """The power channel this class draws from, or supplies on."""
        return POWER_CHANNEL[self.type_id]

    @property
    def efficiency(self) -> float | None:
        """A building's efficiency, where this is the class that carries it.

        The engine scales the value by the component's condition and by a
        level that starts at 1, so this is what an undamaged building runs at.
        """
        if self.type_id != EFFICIENCY_TYPE or not self.values:
            return None
        return self.values[0]

    @property
    def sensor_range(self) -> float | None:
        """A radar's range, or None for any other class."""
        if self.type_id != RADAR_TYPE or not self.values:
            return None
        return self.values[RADAR_RANGE]

    @property
    def camouflage(self) -> float | None:
        """How much a detect shield's camouflage cuts, or None for any other class."""
        if self.type_id != DETECT_SHIELD_TYPE or not self.values:
            return None
        return self.values[CAMOUFLAGE]


def read_component(blob: bytes, pos: int) -> Component | None:
    """Read one component record.  None if it does not read as one."""
    if pos + COMPONENT_FIXED > len(blob):
        return None
    type_id = struct.unpack_from("<i", blob, pos)[0]
    count = struct.unpack_from("<i", blob, pos + COMPONENT_COUNT_AT)[0]
    if type_id not in COMPONENT_TYPES or not 0 <= count <= 4096:
        return None
    end = pos + COMPONENT_FIXED
    entries = struct.unpack_from(f"<{count}i", blob, end) if count else ()
    end += 4 * count
    if end + 4 > len(blob):
        return None
    length = struct.unpack_from("<i", blob, end)[0]
    end += 4
    label = ""
    if length:
        if not 0 < length < 4096 or end + length + 1 > len(blob):
            return None
        label = _fixed_string(blob[end : end + length + 1])
        end += length + 1
    index = struct.unpack_from("<i", blob, pos + COMPONENT_INDEX_AT)[0]
    return Component(
        type_id=type_id,
        resource=ResourceRef(
            _name(blob, pos + COMPONENT_NAME_AT) or "",
            _name(blob, pos + COMPONENT_NAME_AT + NAME_FIELD) or "",
        ),
        index=None if index == -1 else index,
        entries=tuple(entries),
        label=label,
        offset=pos,
        size=end - pos,
        values=struct.unpack_from(f"<{COMPONENT_VALUE_COUNT}f", blob,
                                  pos + COMPONENT_VALUES_AT),
        power=struct.unpack_from("<f", blob, pos + COMPONENT_POWER_AT)[0],
        node=struct.unpack_from("<i", blob, pos + COMPONENT_NODE_AT)[0],
        mass=struct.unpack_from("<f", blob, pos + COMPONENT_MASS_AT)[0],
        flags=struct.unpack_from("<I", blob, pos + COMPONENT_FLAGS_AT)[0],
        group=struct.unpack_from("<i", blob, pos + COMPONENT_GROUP_AT)[0],
        weights=struct.unpack_from("<2f", blob, pos + COMPONENT_WEIGHTS_AT),
    )


def device_input(selector: int) -> tuple[str, int] | None:
    """What a generic device's input selector reads: a kind and an axis.

    The axis is 0-2, or -1 for a length; None is the default input.
    """
    if selector == DEVICE_INPUT_LENGTH:
        return DEVICE_INPUT_KINDS[3], -1
    if DEVICE_INPUT_FIRST <= selector < DEVICE_INPUT_LENGTH:
        kind, axis = divmod(selector - DEVICE_INPUT_FIRST, 3)
        return DEVICE_INPUT_KINDS[kind], axis
    return None


def read_states(blob: bytes, counts: tuple[int, ...]) -> tuple[State, ...]:
    """Section 1's states, which start right after the frame."""
    stride = SECTION1_RECORD + SECTION1_PER_B * counts[1]
    out = []
    for i in range(counts[0]):
        at = HEADER_SIZE + i * stride
        velocity = struct.unpack_from("<6f", blob, at + STATE_VELOCITY_AT)
        spin = struct.unpack_from("<6f", blob, at + STATE_SPIN_AT)
        out.append(State(
            flags=struct.unpack_from("<I", blob, at + STATE_FLAGS_AT)[0],
            velocity=(velocity[:3], velocity[3:]),
            spin=(spin[:3], spin[3:]),
            engine=struct.unpack_from("<f", blob, at + STATE_ENGINE_AT)[0],
            actions=struct.unpack_from("<i", blob, at + STATE_ACTIONS_AT)[0],
            request=struct.unpack_from("<i", blob, at + STATE_REQUEST_AT)[0],
            mode=struct.unpack_from("<I", blob, at + STATE_MODE_AT)[0],
            pair_a=struct.unpack_from("<2f", blob, at + STATE_PAIR_A_AT),
            pair_b=struct.unpack_from("<2f", blob, at + STATE_PAIR_B_AT),
            blend=struct.unpack_from("<f", blob, at + STATE_BLEND_AT)[0],
            length=struct.unpack_from("<f", blob, at + STATE_LENGTH_AT)[0],
            contacts=tuple(
                Contact(*struct.unpack_from("<iIi", blob,
                                            at + SECTION1_RECORD + SECTION1_PER_B * j))
                for j in range(counts[1])),
            lean=tuple(blob[at + STATE_LEAN_AT:at + STATE_LEAN_AT + 3]),
            lean_word=struct.unpack_from("<I", blob, at + STATE_LEAN_AT)[0],
        ))
    return tuple(out)


def read_costs(blob: bytes, counts: tuple[int, ...]) -> tuple[float, ...]:
    """Section 1's A x A transition table, which follows the states."""
    a = counts[0]
    at = HEADER_SIZE + a * (SECTION1_RECORD + SECTION1_PER_B * counts[1])
    return struct.unpack_from(f"<{a * a}f", blob, at)


def _points(blob: bytes, counts: tuple[int, ...]) -> tuple[int, ...]:
    at = section4_start(counts) - counts[2] * SECTION2_RECORD
    return tuple(struct.unpack_from("<i", blob, at + i * SECTION2_RECORD + SECTION2_POINT_AT)[0]
                 for i in range(counts[2]))


def read_channels(blob: bytes, counts: tuple[int, ...]) -> tuple[Channel, ...]:
    """Section 2 as channels."""
    at = section4_start(counts) - counts[2] * SECTION2_RECORD
    out = []
    for i in range(counts[2]):
        base = at + i * SECTION2_RECORD
        first, last, initial = struct.unpack_from("<3f", blob, base + SECTION2_FIRST_AT)
        rate, span = struct.unpack_from("<2f", blob, base + SECTION2_RATE_AT)
        out.append(Channel(
            node=struct.unpack_from("<i", blob, base + SECTION2_NODE_AT)[0],
            origin=struct.unpack_from("<i", blob, base + SECTION2_ORIGIN_AT)[0],
            first=first, last=last, initial=initial,
            point=struct.unpack_from("<i", blob, base + SECTION2_POINT_AT)[0],
            rate=rate, span=span,
            flags=struct.unpack_from("<i", blob, base + SECTION2_FLAGS_AT)[0],
        ))
    return tuple(out)


def section4_start(counts: tuple[int, ...]) -> int:
    """Where the component records begin -- everything before them is fixed."""
    a, b, c = counts[0], counts[1], counts[2]
    return (HEADER_SIZE
            + a * (SECTION1_RECORD + SECTION1_PER_B * b) + 4 * a * a
            + c * SECTION2_RECORD)


def reference_groups(blob: bytes, pos: int, count: int,
                     archives: frozenset[str] | None = None
                     ) -> tuple[list[Reference], int] | None:
    """Read ``count`` reference groups at ``pos``.  None if they do not fit."""
    out: list[Reference] = []
    for group in range(count):
        if pos + 4 > len(blob):
            return None
        n = struct.unpack_from("<i", blob, pos)[0]
        pos += 4
        if n < 0 or pos + n * REFERENCE_STRIDE > len(blob):
            return None
        for _i in range(n):
            library = _name(blob, pos + REFERENCE_NAME_AT) or ""
            member = _name(blob, pos + REFERENCE_NAME_AT + NAME_FIELD) or ""
            if archives is not None and library and library.lower() not in archives:
                return None
            values = struct.unpack_from(f"<{REFERENCE_INTS}i", blob, pos)
            out.append(Reference(ResourceRef(library, member), values, pos, group))
            pos += REFERENCE_STRIDE
    return out, pos


def _triple(blob: bytes, offset: int) -> tuple[float, float, float]:
    return struct.unpack_from("<3f", blob, offset)


def _name(blob: bytes, offset: int) -> str | None:
    """A 32-byte NUL-padded ASCII field, or None if the bytes are not one."""
    if offset + NAME_FIELD > len(blob):
        return None
    field = blob[offset : offset + NAME_FIELD]
    end = field.find(b"\0")
    if end <= 0:
        return None
    text = field[:end]
    if not all(32 <= b < 127 for b in text):
        return None
    return _fixed_string(field)


def parse(blob: bytes, archives: frozenset[str] | None = None) -> Controller:
    """Read a ``.ctl`` member.  Raises unless the 212-byte frame is present."""
    if len(blob) < FRAME_SIZE:
        raise ControlFormatError(
            f"controller is {len(blob)} bytes, short of the {FRAME_SIZE}-byte frame"
        )
    counts = struct.unpack_from("<5i", blob, 0)
    if any(n < 0 for n in counts):
        raise ControlFormatError(f"negative section count in {counts}")
    if not any(counts) and len(blob) != FRAME_SIZE:
        raise ControlFormatError(f"no sections but {len(blob)} bytes, not {FRAME_SIZE}")
    pos = section4_start(counts)
    components: list[Component] = []
    for _ in range(counts[3]):
        part = read_component(blob, pos)
        if part is None:
            raise ControlFormatError(
                f"component {len(components)} of {counts[3]} does not read at {pos}"
            )
        components.append(part)
        pos += part.size
    groups = reference_groups(blob, pos + BLOCK_SIZE, counts[4], archives)
    if groups is None:
        raise ControlFormatError(f"reference groups do not read at {pos + BLOCK_SIZE}")
    references, end = groups
    if end != len(blob):
        raise ControlFormatError(f"consumed {end} of {len(blob)} bytes")
    return Controller(
        counts=counts,
        triples=tuple(_triple(blob, at) for at in TRIPLE_AT),
        scale=struct.unpack_from("<i", blob, 92)[0],
        pair=struct.unpack_from("<2f", blob, 96),
        mode=struct.unpack_from("<i", blob, 104)[0],
        bounds=(
            struct.unpack_from("<f", blob, 108)[0],
            struct.unpack_from("<f", blob, 120)[0],
        ),
        cone=struct.unpack_from("<f", blob, 112)[0],
        flags=struct.unpack_from("<i", blob, 116)[0],
        payload=struct.unpack_from("<f", blob, 124)[0],
        bare=set(blob[HEADER_SIZE:]) == {UNSET},
        components=tuple(components),
        references=tuple(references),
        states=read_states(blob, counts),
        points=_points(blob, counts),
        channels=read_channels(blob, counts),
        costs=read_costs(blob, counts),
        groups=struct.unpack_from(f"<{BLOCK_ENTRIES}i", blob, pos),
    )
