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

What follows the frame is **not** read.  The sections are variable-length and
nest, so their sizes are not a function of the counts: the three sections that
appear alone give strides of 160, 36 and 180, and no assignment of fixed
strides satisfies the other 520 members.  See ``docs/13-control.md``.

Everything below is re-derived by ``uv run openparkan verify``.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass

from .objects import ResourceRef, _fixed_string

#: The NRes tag every ``.ctl`` member carries.
CTL_TAG = "CTLD"

#: The frame: five counts and the 27-dword parameter block.
HEADER_SIZE = 128

#: Section 1's record, section 2's record, the fixed block the loader copies
#: after the component records, and the component type ids the factory at
#: ``0x1002d4b0`` accepts.
SECTION1_RECORD = 156
SECTION1_PER_B = 16
SECTION2_RECORD = 36
BLOCK_SIZE = 84
COMPONENT_TYPES = range(1, 31)

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
#: A field the parser treats as absent when it is -1.
COMPONENT_INDEX_AT = 0x18
#: The 64 bytes the parser copies into the object whole, which the component's
#: value getter (slot 4, ``0x10021d00``) indexes as sixteen floats by the low
#: byte of a value id -- so ``0x300``-``0x305`` are the first six.
COMPONENT_VALUES_AT = 0x2C
COMPONENT_VALUE_COUNT = 16
#: The building class whose first value is its efficiency, the ``KPD`` that
#: ``Behavior.dll`` totals at ``0x100198e0`` over a building's components of
#: this type and multiplies into research, construction and a mine's output.
EFFICIENCY_TYPE = 26
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
#:   counts[1]`` bytes, then ``counts[0] ** 2`` int32.  The engine computes the
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

    The ints are small and unresolved.  The first three are zero on 1373,
    1537 and 1474 of the 1651, the fourth is 3, 4 or 5 on 1230, and two of the
    rest count upwards across a run -- 100, 101, 102 beside 12, 13, 14 --
    which reads as an index rather than a parameter.

    On 41 records the third reads as ASCII rather than a number, so these are
    not the same nine fields on every record.  Both names resolve on all 1651;
    the ints are the part to check before relying on them.
    """

    resource: ResourceRef
    values: tuple[int, ...]
    #: Byte offset of the record within the member, for anyone extending this.
    offset: int


@dataclass(frozen=True)
class Controller:
    """The 212-byte frame at the head of a ``.ctl`` member.

    The triples are per-axis and their roles are **not** established.  What is
    established is their shape and their defaults: ``Control.dll`` drives an
    ``IControl`` of speeds, accelerations and angle limits, the two triples
    that default to a whole turn are angular, and every one of the 12744 float
    reads across the block's 24 float slots is finite.
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
    #: +104: 0, 2 or 3.  Unresolved.
    mode: int
    #: +108 and +120: -1.0 on 465 and 433, otherwise a positive bound.
    bounds: tuple[float, float]
    #: +112: ``pi/2`` on 509.
    cone: float
    #: +116: 0 on 342, then 3, 4, 16.  Unresolved.
    flags: int
    #: +124: ``FLT_MAX`` on 502.
    reach: float
    #: True when +128 to the end is the unset fill.
    bare: bool
    #: Section 4: what this controller is made of, and what each part emits.
    components: tuple[Component, ...]
    #: Section 5's groups: 1432 of the 1651 named references live here, the
    #: other 219 being components' own resources.  In the 136 members that
    #: carry no components at all, none of section 5's records is named.
    references: tuple[Reference, ...]

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
    #: The int32 at +0x18, or None where the parser's -1 means absent.
    index: int | None
    #: The 4-byte entries after the fixed part.
    entries: tuple[int, ...]
    #: The length-prefixed string at the end, empty where the length is zero.
    label: str
    #: Where the record starts, and how long it is.
    offset: int
    size: int
    #: The sixteen floats at ``COMPONENT_VALUES_AT``.
    values: tuple[float, ...] = ()

    @property
    def efficiency(self) -> float | None:
        """A building's efficiency, where this is the class that carries it.

        The engine scales the value by the component's condition and by a
        level that starts at 1, so this is what an undamaged building runs at.
        """
        if self.type_id != EFFICIENCY_TYPE or not self.values:
            return None
        return self.values[0]


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
    )


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
    for _ in range(count):
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
            out.append(Reference(ResourceRef(library, member), values, pos))
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
        reach=struct.unpack_from("<f", blob, 124)[0],
        bare=set(blob[HEADER_SIZE:]) == {UNSET},
        components=tuple(components),
        references=tuple(references),
    )
