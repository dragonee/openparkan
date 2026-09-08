"""Readers for ``effects.rlb`` and the ``.exp`` explosion definitions.

Two small formats that together say what happens when something blows up.

An **``.exp``** (tag ``EXPL``) is a 24-byte header -- a count, four
``float32`` and a flags word -- followed by that many 64-byte
``(archive, member)`` pairs naming the effects to play at once.  ``24 + n *
64`` fits all 144 shipped explosions -- 119 of them in ``weapon.rlb``, the rest
spread over ``animals.rlb``, ``system.rlb``, ``static.rlb`` and
``turrets.rlb`` -- and 212 of the 213 names they carry are real ``FXID``
members.  The one that is not, ``exp_t_sn_mis``, sits beside ``exp_t_st_mis``
and ``exp_t_sw_mis`` in the same archive and reads as a typo.

The second float tracks the size in the record's own name -- 2 on every
``_l``, 3 on every ``_m``, 4 on every ``_b`` -- so it reads as a magnitude.
The file is a fixed 792-byte buffer written without being cleared, so
everything past the last pair is whatever an earlier edit left there; the
count is the only thing that says where the record ends, and ignoring it gets
you stale names that no longer resolve.

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

Inside a block, only the sound emitter is read.  Type 2 keeps a **near and far
audible distance** at +64 and +68, ordered on all 517 blocks and taking values
like (3, 40), (10, 100) and (15, 300); an explosion's is (20, 200).  What the
other types' floats mean is not established; see ``docs/11-effects.md``.

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

#: An ``.exp``'s count, four floats and a flags word.
EXPLOSION_HEADER = 24

#: One ``(archive, member)`` pair of an ``.exp``.
EXPLOSION_STRIDE = 64

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

#: Bit 8 of the type word.  Set on 1811 of the 4737 emitters; role unknown.
EMITTER_FLAG = 0x100

NAME_FIELD = 32


class EffectFormatError(ValueError):
    pass


@dataclass(frozen=True)
class Explosion:
    """An ``.exp``: some numbers and the effects it sets off."""

    #: ``values[1]`` tracks the size suffix of the record's name.  The third
    #: and fourth are 1.0 throughout; the first ranges 0 to 500 and is
    #: unresolved.
    values: tuple[float, float, float, float]
    #: 0 on 81 records and 7 on 63.  Unresolved.
    flags: int
    effects: list[ResourceRef]

    @property
    def magnitude(self) -> float:
        return self.values[1]


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
        """Bit 8 of the type word, which the engine keeps as a boolean."""
        return bool(self.word & EMITTER_FLAG)

    @property
    def audible_range(self) -> tuple[float, float] | None:
        """``(near, far)`` distance of a sound emitter, or None for the rest."""
        if not self.is_sound or len(self.body) < SOUND_FAR + 4:
            return None
        return struct.unpack_from("<2f", self.body, SOUND_NEAR)


@dataclass
class Effect:
    """One ``FXID`` member: a header and its emitters."""

    name: str
    header: bytes
    emitters: list[Emitter]

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
    """Parse an ``.exp``.  Everything past the last pair is stale."""
    if len(blob) < EXPLOSION_HEADER:
        raise EffectFormatError(f"{source}: too short to hold a header")
    count = struct.unpack_from("<i", blob, 0)[0]
    if count < 0 or EXPLOSION_HEADER + count * EXPLOSION_STRIDE > len(blob):
        raise EffectFormatError(
            f"{source}: {count} effects will not fit in {len(blob)} bytes"
        )
    return Explosion(
        values=struct.unpack_from("<4f", blob, 4),
        flags=struct.unpack_from("<i", blob, 20)[0],
        effects=[
            _pair(blob, EXPLOSION_HEADER + i * EXPLOSION_STRIDE)
            for i in range(count)
        ],
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
