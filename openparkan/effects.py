"""Readers for ``effects.rlb`` and the ``.exp`` explosion definitions.

Two small formats that together say what happens when something blows up.

An **``.exp``** (tag ``EXPL``) is what a hit does: 792 bytes, a 24-byte
header -- the hit kind, the damage, the radius, two floats of 1.0 and a
placement word -- and twelve 64-byte ``(archive, member)`` slots.  Slot 0 is
the effect to play; slots 1 to 11 are the effect for each surface struck.
``Control.dll:0x1000ebc0`` switches on the kind: 1 does nothing but the
effect, 2 is a direct hit on the node struck, 3 an area blast.  All 144
shipped explosions are 792 bytes -- 119 of them in ``weapon.rlb``, the rest
in ``animals.rlb``, ``system.rlb``, ``static.rlb`` and ``turrets.rlb`` -- and
fill 0, 1 or all 12 slots.  See ``docs/26-damage.md``.

The first word was read here once as a count of names.  It matches the filled
slots on only 26 of the 144.

An **``FXID``** (in ``effects.rlb``) is one effect: a 60-byte header, then that
many typed **emitter** blocks.  A block opens with a ``uint32`` whose low byte
is its type and whose bit 8 is a flag; the type fixes the block's length and
where inside it the ``(archive, member)`` pair sits::

    type  length  resource at   names
    ----  ------  -----------   ----------------------------------------
      1     224   --            no resource; parameters only
      2     148   84            a sound in sounds.lib
      3     200   136           a material
      4     204   136           a material
      5     112   48            a material
      6       4   --            never used by the shipped data
      7     208   144           a material
      8     248   184           a material
      9     208   136           a material
     10     208   144           a material

The lengths are the engine's own.  ``Effect.dll``'s emitter factory masks the
word to a byte, subtracts one, bounds it at 9 and jumps through a ten-entry
table; each branch allocates its class and then advances the read pointer by
exactly these strides.  That settles two things the data alone could not: type
6 exists but nothing uses it, and **bit 8 is a one-bit flag** -- the factory
computes ``(word >> 8) & 1`` and stores it on the emitter -- rather than part
of the type.

The table walks all 923 shipped effects to the byte -- 4737 emitters -- and
every one of the 3577 material references resolves through ``Material.lib``;
516 of the 517 sounds are in ``sounds.lib``.

Inside a block, ``READ_OFFSETS`` says which floats are **live**.  Each class
keeps only a pointer to its block, so the fields that matter are whatever its
own virtual methods load through that pointer, and walking the vtables of
``Effect.dll`` recovers the set: **181 of the 441 four-byte slots** across the
ten types, the rest written by the editor and never looked at.  Two things
check it -- the sound emitter's +64 and +68 were known from the data long
before the map existed and the map contains them, and not one of the 181
offsets lands inside a block's ``(archive, member)`` pair even though the map
came from the code and ``RESOURCE_AT`` from the data.  The map is a lower
bound: it counts ``fld`` loads straight off the block pointer, so a field
copied as a dword first (type 1's start colour) or read through a pointer
into the block (the exponent triples of types 3, 4, 8 and 9) is live without
being in it.

The families fall out: types 3 and 9 read the same eighteen offsets, 7 and 10
the same thirty-one, and 4 a strict subset of 3's.

Several fields are identified.  Type 2 keeps a **near and far audible
distance** at +64 and +68.  **Type 1 is a light** in its owner's light manager
(``LIGHT_*``, ``Emitter.light``).  A **type-5 bolt** runs from a start point to
where the effect is now, in segments of +36 (``Emitter.bolt_segments``); a
**type-8 stream** emits a particle every +24..+28 seconds; a particle's and a
sprite's **fade** is a (start, end, power) triple.  What the rest mean is not
established; see ``docs/11-effects.md``.

Nothing here draws: an explosion is transient and a static scene has no place
to put one.  What it gives you is the graph, from a mesh node's ``.ndp``
through an ``.exp`` to the sprites and sounds it plays.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass
from pathlib import Path

from .nres import NResArchive
from .objects import ResourceRef, _fixed_string

#: An ``.exp``'s kind, damage, radius, two floats and a placement word.
EXPLOSION_HEADER = 24

#: One ``(archive, member)`` pair of an ``.exp``, and how many there are.
EXPLOSION_STRIDE = 64
EXPLOSION_SLOTS = 12
EXPLOSION_SIZE = EXPLOSION_HEADER + EXPLOSION_SLOTS * EXPLOSION_STRIDE

#: The hit kinds ``Control.dll:0x1000ebc0`` tells apart.  Kind 4, shields
#: only, is handled and never shipped.
HIT_NONE = 1
HIT_DIRECT = 2
HIT_AREA = 3
HIT_SHIELDS = 4

#: The surface tag the effect in slots 1 to 11 carries in its name.  Slot
#: *s* + 1 plays for ground surface *s*, the class byte of the material struck
#: (``Control.dll:0x100117d0``); slot 0 when the surface is unset or its slot
#: does not load (``0x100117f7``).  A contact with no face is surface 10.
SURFACE_TAGS = ("sn", "st", "gr", "sw", "ic", "mt", "gr", "wt", "al", "an", "sh")
SURFACE_NO_FACE = 10

#: Where an explosion's effect is aimed (``Control.dll:0x100115e3``): an axis
#: of the exploding node, of the object, or the struck face's vector.
PLACE_NODE_Y = 0
PLACE_NODE_X = 1
PLACE_NODE_Z = 3
PLACE_OBJECT_X = 4
PLACE_OBJECT_Y = 5
PLACE_OBJECT_Z = 6
PLACE_CONTACT = 7

#: The header: emitter count, time mode, duration in seconds, the spread of a
#: random jitter on effect time, flags, the settings switch the effect belongs
#: to, random offset amplitudes, the point its visibility is tested at, and a
#: scale (``Effect.dll:0x10007650``, ``0x10005c60``, ``0x10008120``,
#: ``0x10007d10``).
HEADER_MODE_AT = 4
HEADER_DURATION_AT = 8
HEADER_JITTER_AT = 12
HEADER_FLAGS_AT = 16
HEADER_GATE_AT = 20
HEADER_SETTING_AT = HEADER_GATE_AT
HEADER_OFFSET_AT = 24
HEADER_POINT_AT = 36
HEADER_SCALE_AT = 48

#: Header flags.  0x10000, on one effect, is not read.
FX_JITTER = 0x1
FX_DELETE_AT_END = 0x2
#: After its first tick the instance bakes its world place into its own frame
#: and lets go of its attach point (``Effect.dll:0x10006324``): an explosion
#: stays where it went off.  409 effects, 402 of them once-through.
FX_DETACH = 0x4
FX_RANDOM_OFFSET = 0x8
FX_KEEP_WHEN_HIDDEN = 0x10
FX_PING_PONG = 0x20
FX_START_OFF = 0x40
FX_HOLD_UNLESS_PAUSED = 0x80
FX_HOLD_WHILE_PAUSED = 0x100
FX_TIMES_LINEAR = 0x200
#: Test the point at ``HEADER_POINT_AT`` for a clear line from the camera, and
#: draw nothing while it is hidden (``0x10007984``, ``0x10008016``).  An emitter
#: with ``EMITTER_FLAG`` asks for the same test.
FX_HIDE_OCCLUDED = 0x400
#: Drawn only by a draw call whose pass argument is set (``0x10007d44``,
#: ``0x10004061``); which call that is is not established.
FX_SECOND_PASS = 0x800
FX_DRAW_TEST = FX_SECOND_PASS
#: Every manager tick hands each emitter the manager's target point, which a
#: type-5 bolt takes for its start (``0x10006349``, ``0x10003070``).  All 14
#: effects with a bolt carry it.
FX_TARGET_POINT = 0x1000
#: Passed on to the renderer as effect draw flag 4 (``0x1001088c``), which
#: reaches the texture choice by distance (``Terrain.dll:0x10028417``).  One
#: effect, ``env_lightning``.
FX_SHADE_FLAG = 0x2000
FX_SKIP_SECOND_TEST = 0x8000

#: Header +0x14 is a **settings id**.  Its low byte indexes the twenty switches
#: of ``Effect.dll``'s own settings page and its high byte is the group the
#: switch is listed under; the names are the DLL's string table, loaded under
#: the same ids (``0x1000e9c0``).  An instance whose switch is off neither
#: updates nor draws (``0x1000ec40``, ``0x10007d59``, ``0x1000825e``).  Every
#: shipped effect names one of these.
EFFECT_SETTINGS: dict[int, str] = {
    0x000: "Dust", 0x001: "Smoke", 0x002: "Engine fire", 0x003: "Explode",
    0x004: "Shield", 0x005: "Lights",
    0x106: "Dust", 0x107: "Smoke", 0x108: "Explode", 0x109: "Shield",
    0x10A: "Lights", 0x10F: "Lights",
    0x20B: "Smoke", 0x20C: "Engine fire", 0x20D: "Explode", 0x20E: "Gun fire",
    0x310: "Smoke", 0x311: "Explode", 0x312: "Gun fire", 0x313: "Lights",
}

#: The three presets the page's slot 8 sets (``0x1000e7c0``), as the switches
#: they leave on.  Presets 1 and 2 turn everything on; preset 3 turns off the
#: dust and smoke of group 0, the dust, smoke and lights of group 1, the smoke
#: and engine fire of group 2 and the smoke of group 3.
EFFECT_PRESETS: dict[int, frozenset[int]] = {
    1: frozenset(EFFECT_SETTINGS),
    2: frozenset(EFFECT_SETTINGS),
    3: frozenset(EFFECT_SETTINGS) - {0x000, 0x001, 0x106, 0x107, 0x10A, 0x20B,
                                      0x20C, 0x310},
}

#: ``Iron_3D.ini``'s ``RENDER_QUALITY`` -> the preset ``iron3d.dll`` asks every
#: settings page for (``0x100616f0``); the page starts on preset 1.
RENDER_QUALITY_PRESET = {0: 3, 1: 2, 2: 1}


def setting_enabled(setting: int, preset: int = 1) -> bool:
    """Whether a settings id's switch is on under a preset."""
    return setting in EFFECT_PRESETS.get(preset, frozenset())

#: How effect time *t*, 0 to 1, is found (``Effect.dll:0x10005c60``): set from
#: outside and 0 until set; once through the duration; looping; reversed; the
#: animation value of an owner's mesh node (``TIME_POINT``: the node action 14
#: names, counted from its part's first node, read through the owner's
#: interface 0xb slot 9, ``AniMesh.dll:0x10005600`` -- the value the channel
#: animating that node sets, ``Control.dll:0x10021c97``); speed over top speed
#: (6-8 per axis); spin (10-12 per axis); one minus a point's or property's
#: value; the larger of speed and spin; a node value that only rises or only
#: falls.
TIME_MANUAL = 0
TIME_ONCE = 1
TIME_LOOP = 2
TIME_REVERSE = 3
TIME_POINT = 4
TIME_SPEED = 5
TIME_SPIN = 9
TIME_POINT_INVERSE = 13
TIME_PROPERTY_INVERSE = 14
TIME_MOTION = 15
TIME_POINT_RISING = 16
TIME_POINT_FALLING = 17
TIME_MODES = 18

#: Emitter type -> where the ``(low, high)`` span of effect time it is active
#: in sits.  Outside it the emitter does nothing.  Type 2, the sound, plays
#: once when *t* crosses its low value.
WINDOW_AT = {1: 8, 2: 8, 3: 32, 4: 32, 5: 12, 7: 20, 8: 16, 9: 32, 10: 20}

#: The 60 bytes before an effect's first emitter block.
HEADER_SIZE = 60

#: Emitter type -> the block's length in bytes, as ``Effect.dll``'s factory
#: advances its read pointer.  Type 6 never appears in the shipped data.
EMITTER_SIZE = {1: 224, 2: 148, 3: 200, 4: 204, 5: 112, 6: 4, 7: 208, 8: 248,
                9: 208, 10: 208}

#: The types the engine's jump table covers.
EMITTER_TYPES = range(1, 11)

#: A sound emitter's near and far audible distance.
SOUND_NEAR = 64
SOUND_FAR = 68

#: Emitter type -> where its ``(archive, member)`` pair starts in the block.
#: Type 1 has none.
RESOURCE_AT = {2: 84, 3: 136, 4: 136, 5: 48, 7: 144, 8: 184, 9: 136, 10: 144}

#: The one emitter type that plays a sound rather than drawing something.
EMITTER_SOUND = 2

#: Bit 8 of the type word.  Set on 1811 of the 4737 emitters, all of them
#: sprites or particles (types 3, 4, 7, 8 and 9).  It asks the instance to test
#: its point (``HEADER_POINT_AT``) for a clear line from the camera at
#: intervals (``Effect.dll:0x10007984``, ``0x10007f7f``), and while
#: the point is in view the emitter draws with the renderer's depth test off
#: (``0x10009930`` sets draw flag 1; ``Terrain.dll:0x100282c6`` clears
#: ``ZENABLE``) -- a glow seen through whatever stands in front of it.  Hidden,
#: it draws like any other sprite.
EMITTER_FLAG = 0x100
EMITTER_OVERLAY = EMITTER_FLAG

#: Type 1 is a **light** in the owner's light manager (interface 0xe, a
#: ``Terrain.dll`` ``CLightManager``; ``Effect.dll:0x1000f4b0``,
#: ``0x1000f6e0``).  Each quantity is a (start, end) pair lerped by the
#: progress through the window; the manager's record is Direct3D's
#: ``D3DLIGHT2`` layout plus three fields.
LIGHT_KIND_AT = 4
#: Where the light sits, in the effect's own frame.
LIGHT_POSITION_AT = (16, 28)
#: Which way it points.
LIGHT_DIRECTION_AT = (40, 52)
#: Its colour, RGBA -- (0.5, 0.3, 0.01) on the hero's cannon.
LIGHT_COLOUR_AT = (64, 80)
#: A random amount up to +-half of each, added to the colour every update.
LIGHT_COLOUR_JITTER_AT = 96
#: Its range -- 30 to 3 on the cannon's flash -- and a jitter on it.
LIGHT_RANGE_AT = (112, 116)
LIGHT_RANGE_JITTER_AT = 120
#: The three attenuation terms, handed to the manager as they are.
LIGHT_ATTENUATION_AT = 124

#: ``Direct3D`` light types, as the manager's record takes them.
LIGHT_POINT = 1
LIGHT_DIRECTIONAL = 3
LIGHT_PARALLEL_POINT = 4

#: Block +4 -> (light type, manager flags) (``0x1000f649``).  The manager skips
#: a light with ``0x80000000`` unless it is drawing that light's own owner
#: (``Terrain.dll:0x10047a52``) and leaves one with ``0x20000000`` out of
#: ``EmulatePointLights`` (``0x1002a200``).  Anything else is kind 1.
LIGHT_KINDS: dict[int, tuple[int, int]] = {
    1: (LIGHT_POINT, 0x80000000),
    2: (LIGHT_POINT, 0),
    3: (LIGHT_DIRECTIONAL, 0),
    4: (LIGHT_PARALLEL_POINT, 0),
    5: (LIGHT_POINT, 0),
    6: (LIGHT_POINT, 0xA0000000),
    7: (LIGHT_POINT, 0x20000000),
}

#: A type-5 bolt: at most +20 sprites (a ``uint32``), one for every +36 units
#: of its length (``Effect.dll:0x10002c53``).
BOLT_COUNT_AT = 20
BOLT_SEGMENT_AT = 36

#: A type-8 stream emits one particle every lerp(+24, +28) seconds as its
#: window runs (``Effect.dll:0x10011a6c``), into a ring of +36 (``uint32``).
#: A particle's age runs 0 to 1 over the ring, so it lives +36 intervals
#: (``0x1001209e``).
STREAM_INTERVAL_AT = (24, 28)
STREAM_CAPACITY_AT = 36

#: The value a particle (types 7 and 10, or a stream's, type 8) or a sprite
#: (3, 4 and 9) hands the renderer: ``start + (end - start) * x ** power``, x
#: the particle's age or the progress through the window (``0x100013c2``,
#: ``0x10012322``, ``0x10010881``).  Zero draws nothing
#: (``Terrain.dll:0x1002887e``).  1197 of the 1321 burst blocks and 233 of the
#: 237 streams fade from start to a smaller end.
PARTICLE_FADE_AT = (8, 12, 16)
STREAM_FADE_AT = (4, 8, 12)
SPRITE_FADE_AT = (20, 24, 28)

NAME_FIELD = 32

#: Emitter type -> the offsets its own class **loads as a float**, recovered
#: from ``Effect.dll``.  Each class keeps only a pointer to its block -- at
#: ``this+0x18`` for most of them, ``+0x1c`` for type 1 and ``+0x24`` for the
#: sound -- so the live fields are whatever its virtual methods read through
#: that pointer.  This is the set those methods reach directly or one call
#: deep.
#:
#: It is a map of what is *used*, not of what anything means, and a lower
#: bound: it counts ``fld`` loads straight off the block pointer, so a field
#: copied as a dword first (type 1's light) or read through a pointer into the
#: block (the exponent triples of types 3, 4, 8 and 9 at +64 and +124, type 8's
#: at +172) is live without being here.
#: Type 1's +120, its range jitter, was missed by the first walk and is loaded
#: in its update (``0x1000fa9a``).  See ``docs/11-effects.md``.
READ_OFFSETS: dict[int, tuple[int, ...]] = {
    1: (8, 12, 28, 32, 36, 52, 56, 60, 80, 84, 88, 92, 112, 116, 120),
    2: (8, 12, 28, 32, 36, 52, 56, 60, 64, 68, 72, 76),
    3: (8, 12, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60,
        100, 104, 108, 112, 116, 120),
    4: (8, 12, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60,
        100, 104, 108, 112, 116, 120),
    5: (4, 8, 12, 16, 24, 28, 32, 36, 40, 44),
    6: (),
    7: (12, 16, 20, 24, 28, 32, 44, 48, 52, 56, 60, 64, 68, 72, 76, 80, 84,
        88, 92, 96, 100, 104, 108, 112, 116, 120, 124, 128, 132, 136, 140),
    8: (8, 12, 16, 20, 24, 28, 32, 52, 56, 60, 88, 92, 96, 100, 104, 108,
        112, 116, 120, 136, 140, 144, 148, 152, 156, 160, 164, 168),
    9: (8, 12, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60,
        100, 104, 108, 112, 116, 120),
    10: (12, 16, 20, 24, 28, 32, 44, 48, 52, 56, 60, 64, 68, 72, 76, 80, 84,
         88, 92, 96, 100, 104, 108, 112, 116, 120, 124, 128, 132, 136, 140),
}

#: Where each class keeps the pointer to its own block.
BLOCK_FIELD = {1: 0x1C, 2: 0x24, 3: 0x18, 4: 0x18, 5: 0x18,
               6: 0x18, 7: 0x18, 8: 0x18, 9: 0x18, 10: 0x18}

#: Types 1 and 2 carry a unit vector here, ``(1, 0, 0)`` on most blocks.  On
#: the light it is the end of its direction.
DIRECTION_AT = 52
DIRECTION_TYPES = (1, 2)


class EffectFormatError(ValueError):
    pass


@dataclass(frozen=True)
class Light:
    """What a type-1 emitter asks its owner's light manager for."""

    #: ``LIGHT_POINT``, ``LIGHT_DIRECTIONAL`` or ``LIGHT_PARALLEL_POINT``.
    type: int
    #: The manager's flags word for it; see ``LIGHT_KINDS``.
    flags: int
    position: tuple[tuple[float, float, float], tuple[float, float, float]]
    direction: tuple[tuple[float, float, float], tuple[float, float, float]]
    colour: tuple[tuple[float, float, float, float], tuple[float, float, float, float]]
    colour_jitter: tuple[float, float, float, float]
    range: tuple[float, float]
    range_jitter: float
    attenuation: tuple[float, float, float]


@dataclass(frozen=True)
class Explosion:
    """An ``.exp``: what kind of hit, how hard, how wide, and what it looks like."""

    #: ``HIT_NONE``, ``HIT_DIRECT`` or ``HIT_AREA``.
    kind: int
    #: The level ratio times this, plus the life the exploding node lost, is
    #: the hit's damage (``Control.dll:0x10011794``).
    damage: float
    #: Absolute on a round; a multiple of the node's bounding radius otherwise.
    #: 2 on every ``_l`` record, 3 on every ``_m``, 4 on every ``_b``.
    radius: float
    #: The two floats after the radius, 1.0 throughout.  Not read by the hit.
    values: tuple[float, float]
    #: 7 on 63 records -- at the point of impact -- and 0 on the rest.
    placement: int
    #: The twelve name slots, blank where empty.
    slots: tuple[ResourceRef, ...]

    @property
    def effect(self) -> ResourceRef | None:
        """The effect to play, slot 0."""
        return self.slots[0] if self.slots and self.slots[0] else None

    @property
    def by_surface(self) -> tuple[ResourceRef, ...]:
        """Slots 1 to 11, one per ``SURFACE_TAGS`` entry."""
        return self.slots[1:]

    @property
    def effects(self) -> list[ResourceRef]:
        """Every effect it names, in slot order."""
        return [r for r in self.slots if r]

    def slot_for(self, surface: int | None) -> ResourceRef | None:
        """The effect it plays on ground surface ``surface``, or slot 0."""
        if surface is not None and 0 <= surface < len(SURFACE_TAGS):
            ref = self.slots[surface + 1]
            if ref:
                return ref
        return self.effect


@dataclass(frozen=True)
class Emitter:
    """One block of an effect: what it draws or plays, and how it is typed."""

    kind: int
    #: The whole type word, flag bits included.
    word: int
    resource: ResourceRef
    #: The block's bytes, so a caller can go further than this reader does.
    body: bytes

    @property
    def is_sound(self) -> bool:
        return self.kind == EMITTER_SOUND

    @property
    def flagged(self) -> bool:
        """Bit 8 of the type word: draw over the scene while the point is in view."""
        return bool(self.word & EMITTER_FLAG)

    @property
    def light(self) -> Light | None:
        """The light a type-1 emitter drives, or None for the other types."""
        if self.kind != 1 or len(self.body) < LIGHT_ATTENUATION_AT + 12:
            return None
        b = self.body
        kind = struct.unpack_from("<I", b, LIGHT_KIND_AT)[0]
        light_type, flags = LIGHT_KINDS.get(kind, LIGHT_KINDS[1])

        def pair(at: tuple[int, int], n: int):
            return tuple(struct.unpack_from(f"<{n}f", b, o) for o in at)

        return Light(
            type=light_type,
            flags=flags,
            position=pair(LIGHT_POSITION_AT, 3),
            direction=pair(LIGHT_DIRECTION_AT, 3),
            colour=pair(LIGHT_COLOUR_AT, 4),
            colour_jitter=struct.unpack_from("<4f", b, LIGHT_COLOUR_JITTER_AT),
            range=struct.unpack_from("<2f", b, LIGHT_RANGE_AT[0]),
            range_jitter=struct.unpack_from("<f", b, LIGHT_RANGE_JITTER_AT)[0],
            attenuation=struct.unpack_from("<3f", b, LIGHT_ATTENUATION_AT),
        )

    def bolt_segments(self, length: float) -> int:
        """How many sprites a type-5 bolt of ``length`` units draws; 0 otherwise.

        The length is not in the block: it is the distance from the bolt's start
        point to where the effect is now (``Effect.dll:0x10002c3f``).
        """
        if self.kind != 5 or len(self.body) < BOLT_SEGMENT_AT + 4:
            return 0
        most = struct.unpack_from("<I", self.body, BOLT_COUNT_AT)[0]
        step = struct.unpack_from("<f", self.body, BOLT_SEGMENT_AT)[0]
        n = int(length // step) if step > 0 else 0
        return max(1, min(n, most))

    @property
    def emission_interval(self) -> tuple[float, float] | None:
        """A type-8 stream's seconds between particles at its window's ends."""
        if self.kind != 8 or len(self.body) < STREAM_INTERVAL_AT[1] + 4:
            return None
        return struct.unpack_from("<2f", self.body, STREAM_INTERVAL_AT[0])

    @property
    def particle_lifetime(self) -> tuple[float, float] | None:
        """A type-8 particle's seconds of life at its window's ends."""
        interval = self.emission_interval
        if interval is None or len(self.body) < STREAM_CAPACITY_AT + 4:
            return None
        ring = struct.unpack_from("<I", self.body, STREAM_CAPACITY_AT)[0]
        return (ring * interval[0], ring * interval[1])

    @property
    def fade(self) -> tuple[float, float, float] | None:
        """``(start, end, power)`` of the value this emitter draws with."""
        at = {7: PARTICLE_FADE_AT, 10: PARTICLE_FADE_AT, 8: STREAM_FADE_AT,
              3: SPRITE_FADE_AT, 4: SPRITE_FADE_AT, 9: SPRITE_FADE_AT}.get(self.kind)
        if at is None or len(self.body) < at[2] + 4:
            return None
        return tuple(struct.unpack_from("<f", self.body, o)[0] for o in at)

    @property
    def audible_range(self) -> tuple[float, float] | None:
        """``(near, far)`` distance of a sound emitter, or None for the rest."""
        if not self.is_sound or len(self.body) < SOUND_FAR + 4:
            return None
        return struct.unpack_from("<2f", self.body, SOUND_NEAR)

    def live_floats(self) -> dict[int, float]:
        """The floats this emitter's own class actually reads, by offset.

        The rest of the block is written by the editor and never loaded.  What
        any of these mean is not established -- see ``READ_OFFSETS``.
        """
        return {
            at: struct.unpack_from("<f", self.body, at)[0]
            for at in READ_OFFSETS.get(self.kind, ())
            if at + 4 <= len(self.body)
        }

    @property
    def window(self) -> tuple[float, float] | None:
        """The ``(low, high)`` span of effect time this emitter is active in."""
        at = WINDOW_AT.get(self.kind)
        if at is None or len(self.body) < at + 8:
            return None
        return struct.unpack_from("<2f", self.body, at)

    @property
    def direction(self) -> tuple[float, float, float] | None:
        """The unit vector types 1 and 2 keep at +52, or None for the rest."""
        if self.kind not in DIRECTION_TYPES or len(self.body) < DIRECTION_AT + 12:
            return None
        return struct.unpack_from("<3f", self.body, DIRECTION_AT)


@dataclass
class Effect:
    """One ``FXID`` member: a header and its emitters."""

    name: str
    header: bytes
    emitters: list[Emitter]

    @property
    def mode(self) -> int:
        """The header's time mode, ``TIME_*``."""
        return struct.unpack_from("<I", self.header, HEADER_MODE_AT)[0]

    @property
    def duration(self) -> float:
        """Seconds from start to end."""
        return struct.unpack_from("<f", self.header, HEADER_DURATION_AT)[0]

    @property
    def flags(self) -> int:
        """``FX_*``."""
        return struct.unpack_from("<I", self.header, HEADER_FLAGS_AT)[0]

    @property
    def gate(self) -> int:
        """The settings id whose switch must be on for the effect to run."""
        return struct.unpack_from("<I", self.header, HEADER_GATE_AT)[0]

    @property
    def setting(self) -> str | None:
        """The name ``Effect.dll`` gives the effect's switch, or None."""
        return EFFECT_SETTINGS.get(self.gate)

    @property
    def test_point(self) -> tuple[float, float, float]:
        """The point, in the instance's frame, whose view from the camera is tested."""
        return struct.unpack_from("<3f", self.header, HEADER_POINT_AT)

    @property
    def scale(self) -> tuple[float, float, float]:
        """What a requested size is multiplied by."""
        return struct.unpack_from("<3f", self.header, HEADER_SCALE_AT)

    @property
    def materials(self) -> list[str]:
        """The material names this effect draws, in block order."""
        return [e.resource.member for e in self.emitters
                if e.resource and not e.is_sound]

    @property
    def sounds(self) -> list[str]:
        return [e.resource.member for e in self.emitters
                if e.resource and e.is_sound]


def _pair(blob: bytes, offset: int) -> ResourceRef:
    return ResourceRef(
        _fixed_string(blob[offset : offset + NAME_FIELD]),
        _fixed_string(blob[offset + NAME_FIELD : offset + 2 * NAME_FIELD]),
    )


def parse_explosion(blob: bytes, source: str = "<exp>") -> Explosion:
    """Parse an ``.exp``.  Raises unless it is the 792-byte record."""
    if len(blob) != EXPLOSION_SIZE:
        raise EffectFormatError(
            f"{source}: {len(blob)} bytes, not the {EXPLOSION_SIZE} of an explosion"
        )
    kind, damage, radius, one, two, placement = struct.unpack_from("<i4fi", blob, 0)
    return Explosion(
        kind=kind,
        damage=damage,
        radius=radius,
        values=(one, two),
        placement=placement,
        slots=tuple(_pair(blob, EXPLOSION_HEADER + i * EXPLOSION_STRIDE)
                    for i in range(EXPLOSION_SLOTS)),
    )


def parse_effect(blob: bytes, name: str = "<fxid>") -> Effect:
    """Parse an ``FXID``.  Raises unless the emitters fit the member exactly."""
    if len(blob) < 4:
        raise EffectFormatError(f"{name}: too short to hold an emitter count")
    count = struct.unpack_from("<i", blob, 0)[0]
    if count < 0:
        raise EffectFormatError(f"{name}: negative emitter count {count}")
    emitters = []
    pos = HEADER_SIZE
    for i in range(count):
        if pos + 4 > len(blob):
            raise EffectFormatError(f"{name}: emitter {i} starts past the end")
        word = struct.unpack_from("<I", blob, pos)[0]
        kind = word & 0xFF
        size = EMITTER_SIZE.get(kind)
        if size is None or pos + size > len(blob):
            raise EffectFormatError(
                f"{name}: emitter {i} has type {word:#x} at offset {pos}"
            )
        at = RESOURCE_AT.get(kind)
        emitters.append(
            Emitter(
                kind=kind,
                word=word,
                resource=_pair(blob, pos + at) if at else ResourceRef("", ""),
                body=blob[pos : pos + size],
            )
        )
        pos += size
    return Effect(name, blob[:HEADER_SIZE], emitters)


class EffectLibrary:
    """``effects.rlb``, parsed into effects keyed by name."""

    def __init__(self, path: str | Path):
        self.archive = NResArchive.open(path)
        self.effects: dict[str, Effect] = {}
        for entry in self.archive:
            try:
                self.effects[entry.name.lower()] = parse_effect(
                    self.archive.read(entry), entry.name
                )
            except EffectFormatError:
                continue

    def get(self, name: str) -> Effect | None:
        return self.effects.get(name.lower())

    def __len__(self) -> int:
        return len(self.effects)

    def __iter__(self):
        return iter(self.effects.values())
