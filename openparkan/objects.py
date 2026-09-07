"""Object definitions: ``objects.rlb`` records and ``UNITS/**/*.dat`` assemblies.

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
"""

from __future__ import annotations

import struct
from dataclasses import dataclass
from pathlib import Path

from .nres import NResArchive

SLOT_SIZE = 64
NAME_FIELD = 32

DAT_MAGIC = 0xF0F1
DAT_HEADER = 8
DAT_COMPONENT = 112


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
    a: int
    b: int
    c: int
    d: int


@dataclass
class UnitDefinition:
    source: Path
    kind: int
    components: list[Component]

    @property
    def label(self) -> str:
        """The first component's display name, which names the whole thing."""
        return self.components[0].label if self.components else ""


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
        a, b = struct.unpack_from("<Ii", data, o + 64)
        c, d = struct.unpack_from("<Ii", data, o + 104)
        components.append(
            Component(
                ref=ResourceRef(
                    _fixed_string(data[o : o + NAME_FIELD]),
                    _fixed_string(data[o + NAME_FIELD : o + 64]),
                ),
                label=_fixed_string(data[o + 72 : o + 104]),
                a=a,
                b=b,
                c=c,
                d=d,
            )
        )
    return UnitDefinition(path, kind, components)
