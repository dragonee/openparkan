"""Object definitions: ``objects.rlb`` records, ``.ndp`` damage tables, ``.bas``
footprints, and ``UNITS/**/*.dat`` assemblies.

Two small fixed-layout formats that together turn a name in a mission file
into concrete resources.

``objects.rlb`` is a table of **resource reference records**.  Each record is a
run of 64-byte slots holding a ``(archive, member)`` pair, and the record's
NRes tag says what sort of thing it is::

    STAT  384 bytes, 6 slots   scenery      .msh .wea .cpt .ndp .ctl
    INTO  320 bytes, 5 slots   internal part
    EXTO  320 bytes, 5 slots   external part (turrets)
    BULL  320 bytes, 5 slots   projectile
    WPNS  320 bytes, 5 slots   weapon
    BTLU  320 / 448 bytes      creature
    FORT  128 bytes, 2 slots   fortification / building
    SUNO  128 bytes, 2 slots   sun

A ``UNITS/**/*.dat`` file is a unit or building **assembly**: a magic word, a
class word, then 112-byte components, each naming one ``objects.rlb`` record
plus the display name the game shows for it ("Large Track Chs (L-42t)",
"ARMOUR LA.Mk3 (ARM 3)").  This is the modular-robot mechanic the game is
built around, expressed directly in the data.

The component list is a **tree written depth first**: each record carries a
child count, and its children are the records that follow.  That reading
consumes all 458 shipped assemblies exactly.  A child also carries the *node
index* in its parent's mesh that it bolts onto -- always one of the parent's
geometry-less ``Base_*`` nodes, on 946 of 946 guns and 468 of 468 turrets --
so an assembled robot is built by walking the tree and composing each part's
mesh onto that node's pose.

A ``.bas`` slot is a building's **ground plan**: two closed outlines in the
model's own frame, an inner one that traces the building itself and an outer
one a clearance margin beyond it -- 1.3 to 2.4 times the area, 1.6 median.
All 30 shipped records hold exactly two, all 60 rings wind anticlockwise, and
the inner ring's XY extent is the model's own bounding box on 23 of the 30.
Placed and turned by the mission's angle, it sits on the terrain: over 167
placed buildings the ground under the outline runs a median 1.93 units from
its lowest point to its highest, and the placement height is a median 0.04
above their mean.

A ``.ndp`` slot is the model's **damage table**: one 76-byte record per mesh
node, holding that node's durability and the explosion it plays when it is
destroyed -- ``explode_tree.exp`` for a tree, ``explode_frt_b.exp`` for a big
fortification, ``selfexp_anl_01b.exp`` for an animal.  Parkan lets you shoot a
building apart piece by piece, and this is the table that says what each piece
costs and what it looks like going up.  See ``docs/07-objects.md``.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field
from pathlib import Path

from .nres import NResArchive

SLOT_SIZE = 64
NAME_FIELD = 32

#: A ``.bas`` ring: a constant 1, a point count, that many points plus a
#: repeat of the first, and -- on a ring taken off the model -- one ``int32``
#: triangle index per point followed by one ``int32`` corner.
BASE_MARKER = 1

#: A ``.ndp`` is an int32 count and then one 76-byte record per mesh node:
#: an int32 of flags, two floats, and the ``(archive, member)`` pair naming the
#: explosion that node plays when it is destroyed.  All 542 shipped members are
#: exactly ``4 + n * 76`` bytes and 541 of them have ``n`` equal to the node
#: count of the mesh they belong to.
DAMAGE_STRIDE = 76

DAT_MAGIC = 0xF0F1
DAT_HEADER = 8
DAT_COMPONENT = 112

#: A component's class, which says both what it does and where it hangs.
CLASS_CHASSIS = 0
CLASS_TURRET = 1
CLASS_ARMOUR = 2
CLASS_INTERNAL = 3
CLASS_GUN = 4
CLASS_AMMO = 5

CLASS_NAMES = {
    CLASS_CHASSIS: "chassis",
    CLASS_TURRET: "turret",
    CLASS_ARMOUR: "armour",
    CLASS_INTERNAL: "internal",
    CLASS_GUN: "gun",
    CLASS_AMMO: "ammo",
}

#: Classes whose geometry is drawn outside the hull.  Armour, internal systems
#: and ammunition are modelled but only visible from inside the machine.
EXTERNAL_CLASSES = frozenset({CLASS_CHASSIS, CLASS_TURRET, CLASS_GUN})


class ObjectFormatError(ValueError):
    pass


def _fixed_string(raw: bytes) -> str:
    return raw.split(b"\0")[0].decode("latin-1")


@dataclass(frozen=True)
class ResourceRef:
    """A ``(archive, member)`` pair naming one resource."""

    library: str
    member: str

    def __bool__(self) -> bool:
        return bool(self.library and self.member)

    @property
    def suffix(self) -> str:
        return self.member.rsplit(".", 1)[-1].lower() if "." in self.member else ""

    def __str__(self) -> str:
        return f"{self.library}/{self.member}" if self else "-"


@dataclass(frozen=True)
class NodeDamage:
    """What happens to one node of a model when it is shot to pieces."""

    #: Zero throughout, except 1 on scenery and 112 on projectiles.  Flags.
    flags: int
    #: The node's hit points.  ``Control.dll:0x1000f940`` sets a node's life to
    #: this times two object scales, and a component on the node is scaled by
    #: what is left of it.  It is also the float that grows with the node,
    #: +0.56 with its volume in log space.  1000000 where it cannot be destroyed.
    durability: float
    #: Unresolved, and not a handful of values: 104 distinct over 2334
    #: records.  The 17 whole numbers (1000 on 549, 0, 10, 1, 300, 500 ...)
    #: sit on buildings, scenery and projectiles and read as authored.  The
    #: other 87 are fractional -- 91.008, 1124.23 -- on 873 records that are
    #: all unit parts, and read as computed.  They fall as the node grows
    #: (log-log -0.46 against its bounding volume), so not a mass.
    unknown: float
    #: The explosion to play, as an ``(archive, member)`` pair like any other.
    explosion: ResourceRef

    def __bool__(self) -> bool:
        return bool(self.explosion)


def parse_damage(blob: bytes, source: str = "<ndp>") -> list[NodeDamage]:
    """Parse a ``.ndp`` damage table, one record per node of its mesh."""
    if len(blob) < 4:
        raise ObjectFormatError(f"{source}: too short to hold a record count")
    count = struct.unpack_from("<i", blob, 0)[0]
    if count < 0 or len(blob) != 4 + count * DAMAGE_STRIDE:
        raise ObjectFormatError(
            f"{source}: {len(blob)} bytes is not {count} records of {DAMAGE_STRIDE}"
        )
    out = []
    for i in range(count):
        o = 4 + i * DAMAGE_STRIDE
        flags, durability, unknown = struct.unpack_from("<iff", blob, o)
        out.append(
            NodeDamage(
                flags=flags,
                durability=durability,
                unknown=unknown,
                explosion=ResourceRef(
                    _fixed_string(blob[o + 12 : o + 44]),
                    _fixed_string(blob[o + 44 : o + DAMAGE_STRIDE]),
                ),
            )
        )
    return out


@dataclass(frozen=True)
class Footprint:
    """One ring of a ``.bas``: a closed outline in the model's own frame."""

    #: The ring's corners, without the repeated closing point.
    points: list[tuple[float, float, float]]
    #: Where each corner came from: ``(triangle, corner)`` into the building's
    #: own mesh, so the point is ``mesh.triangles[triangle][corner]``.  Only
    #: the inner ring carries it -- the outer one is a clearance the author
    #: drew rather than traced.  Empty on the outer ring.
    traced: list[tuple[int, int]] = field(default_factory=list)

    @property
    def area(self) -> float:
        """The shoelace area in XY.  Every shipped ring winds anticlockwise."""
        total = 0.0
        for i, (x1, y1, _) in enumerate(self.points):
            x2, y2, _ = self.points[(i + 1) % len(self.points)]
            total += x1 * y2 - x2 * y1
        return total / 2


def parse_base(blob: bytes, source: str = "<bas>") -> list[Footprint]:
    """Parse a ``.bas``.  Every shipped record holds exactly two rings."""
    out: list[Footprint] = []
    pos = 0
    while pos < len(blob):
        if pos + 8 > len(blob):
            raise ObjectFormatError(f"{source}: no room for a ring header at {pos}")
        marker, count = struct.unpack_from("<2i", blob, pos)
        if marker != BASE_MARKER or not 3 <= count <= 256:
            raise ObjectFormatError(
                f"{source}: ring header ({marker}, {count}) at {pos}"
            )
        end = pos + 8 + (count + 1) * 12
        if end > len(blob):
            raise ObjectFormatError(f"{source}: ring at {pos} runs past the end")
        points = [
            struct.unpack_from("<3f", blob, pos + 8 + i * 12) for i in range(count + 1)
        ]
        if points[0] != points[-1]:
            raise ObjectFormatError(f"{source}: ring at {pos} does not close")
        pos = end
        traced: list[tuple[int, int]] = []
        # A ring the author *traced* on the model carries a back-reference per
        # corner: one array of triangle indices, then one of which corner of
        # each.  The outer ring is a clearance drawn around the building
        # rather than taken off it, so it has none -- which is why this looked
        # like a header that would not divide evenly.
        if pos < len(blob):
            if pos + 8 * count > len(blob):
                raise ObjectFormatError(
                    f"{source}: no room for {count} traced corners at {pos}"
                )
            faces = struct.unpack_from(f"<{count}i", blob, pos)
            corners = struct.unpack_from(f"<{count}i", blob, pos + 4 * count)
            traced = list(zip(faces, corners, strict=True))
            pos += 8 * count
        out.append(Footprint(points[:-1], traced))
    return out


@dataclass
class ObjectRecord:
    """One record of ``objects.rlb``: a name, a tag, and its resource slots."""

    name: str
    tag: str
    slots: list[ResourceRef]

    def slot_with_suffix(self, suffix: str) -> ResourceRef | None:
        for s in self.slots:
            if s and s.suffix == suffix:
                return s
        return None

    @property
    def mesh(self) -> ResourceRef | None:
        """The ``.msh`` slot, when this record has geometry of its own."""
        return self.slot_with_suffix("msh")

    @property
    def textures(self) -> ResourceRef | None:
        """The ``.wea`` slot, a name table of texture names."""
        return self.slot_with_suffix("wea")

    @property
    def damage(self) -> ResourceRef | None:
        """The ``.ndp`` slot, a per-node damage table."""
        return self.slot_with_suffix("ndp")

    @property
    def footprint(self) -> ResourceRef | None:
        """The ``.bas`` slot, a building's ground outline."""
        return self.slot_with_suffix("bas")


class ObjectLibrary:
    """``objects.rlb``, parsed into records keyed by name."""

    def __init__(self, path: str | Path):
        self.archive = NResArchive.open(path)
        self.records: dict[str, ObjectRecord] = {}
        for entry in self.archive:
            data = self.archive.read(entry)
            slots = [
                ResourceRef(
                    _fixed_string(data[i * SLOT_SIZE : i * SLOT_SIZE + NAME_FIELD]),
                    _fixed_string(
                        data[i * SLOT_SIZE + NAME_FIELD : (i + 1) * SLOT_SIZE]
                    ),
                )
                for i in range(len(data) // SLOT_SIZE)
            ]
            self.records[entry.name.lower()] = ObjectRecord(entry.name, entry.tag, slots)

    def get(self, name: str) -> ObjectRecord | None:
        return self.records.get(name.lower())

    def by_tag(self, tag: str) -> list[ObjectRecord]:
        return [r for r in self.records.values() if r.tag == tag]

    def __len__(self) -> int:
        return len(self.records)


@dataclass
class Component:
    """One part of a unit or building assembly."""

    ref: ResourceRef
    label: str
    #: One throughout the shipped data; role unknown.
    flags: int
    #: Node index in the *parent* component's mesh that this part bolts onto.
    #: The chassis, which has no parent, carries -1.
    attach_node: int
    class_id: int
    #: How many of the following components hang off this one.
    child_count: int

    @property
    def class_name(self) -> str:
        return CLASS_NAMES.get(self.class_id, "?")

    @property
    def is_external(self) -> bool:
        """Whether this part is drawn on the outside of the machine."""
        return self.class_id in EXTERNAL_CLASSES


@dataclass
class UnitDefinition:
    source: Path
    kind: int
    components: list[Component]

    @property
    def label(self) -> str:
        """The first component's display name, which names the whole thing."""
        return self.components[0].label if self.components else ""

    def parents(self) -> list[int]:
        """Parent index of every component, walking the depth-first tree.

        The root -- the chassis -- gets -1.  Raises ObjectFormatError if the
        child counts do not account for the file exactly, which they do on all
        458 shipped assemblies.
        """
        parent = [-1] * len(self.components)
        stack: list[tuple[int, int]] = []  # (component index, children still owed)
        for i, component in enumerate(self.components):
            if stack:
                owner, owed = stack[-1]
                parent[i] = owner
                if owed == 1:
                    stack.pop()
                else:
                    stack[-1] = (owner, owed - 1)
            elif i:
                raise ObjectFormatError(
                    f"{self.source}: component {i} has no parent in the tree"
                )
            if component.child_count:
                stack.append((i, component.child_count))
        if stack:
            raise ObjectFormatError(
                f"{self.source}: {len(stack)} components still owe children at the end"
            )
        return parent


def load_unit(path: str | Path) -> UnitDefinition:
    """Parse a ``UNITS/**/*.dat`` assembly."""
    path = Path(path)
    data = path.read_bytes()
    if len(data) < DAT_HEADER:
        raise ObjectFormatError(f"{path}: too short to be a unit definition")
    magic, kind = struct.unpack_from("<II", data, 0)
    if magic != DAT_MAGIC:
        raise ObjectFormatError(f"{path}: expected magic 0x{DAT_MAGIC:x}, got 0x{magic:x}")
    body = len(data) - DAT_HEADER
    if body % DAT_COMPONENT:
        raise ObjectFormatError(
            f"{path}: {body} bytes of components is not a multiple of {DAT_COMPONENT}"
        )
    components = []
    for i in range(body // DAT_COMPONENT):
        o = DAT_HEADER + i * DAT_COMPONENT
        flags, attach = struct.unpack_from("<Ii", data, o + 64)
        class_id, children = struct.unpack_from("<Ii", data, o + 104)
        components.append(
            Component(
                ref=ResourceRef(
                    _fixed_string(data[o : o + NAME_FIELD]),
                    _fixed_string(data[o + NAME_FIELD : o + 64]),
                ),
                label=_fixed_string(data[o + 72 : o + 104]),
                flags=flags,
                attach_node=attach,
                class_id=class_id,
                child_count=children,
            )
        )
    return UnitDefinition(path, kind, components)
