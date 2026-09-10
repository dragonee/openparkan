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

#: The parameter block, and the block after it that can be wholly unset.
HEADER_SIZE = 128
TRAILER_SIZE = 84
#: What a controller occupies before any section: 212 bytes on every member,
#: and the whole file on the six that carry no sections.
FRAME_SIZE = HEADER_SIZE + TRAILER_SIZE

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

#: A reference record inside a section: two 32-byte NUL-padded name fields and
#: nine int32.  They occur in runs at this stride; the runs are anchored by
#: hand because the sections around them are not parsed.
REFERENCE_STRIDE = 100
NAME_FIELD = 32
REFERENCE_INTS = 9


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
    #: The reference records found in the sections after the frame.
    references: tuple[Reference, ...]

    @property
    def sections(self) -> int:
        """How many sections the counts ask for, across all five kinds."""
        return sum(self.counts)


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


def find_references(blob: bytes, archives: frozenset[str] | None = None) -> list[Reference]:
    """Locate the 100-byte reference records in the sections after the frame.

    The sections are not parsed, so the records are found by their shape: two
    readable name fields where the first names an archive.  Passing the set of
    archive names that actually exist anchors the scan -- without it a record
    whose member field holds an uninitialised tail can be picked up four bytes
    late, splitting ``objects.rlb`` into ``cts.rlb``.
    """
    out: list[Reference] = []
    pos = HEADER_SIZE
    while pos + REFERENCE_STRIDE <= len(blob):
        library = _name(blob, pos)
        member = _name(blob, pos + NAME_FIELD)
        if library and member and (archives is None or library.lower() in archives):
            values = struct.unpack_from(f"<{REFERENCE_INTS}i", blob, pos + 2 * NAME_FIELD)
            out.append(Reference(ResourceRef(library, member), values, pos))
            pos += REFERENCE_STRIDE
        else:
            pos += 4
    return out


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
        references=tuple(find_references(blob, archives)),
    )
