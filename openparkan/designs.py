"""A warbot design, the way the robot constructor offers, fits, rates and writes one.

``iron3d.dll``'s constructor screen (docs/37-designer.md) lists parts by name
prefix out of the clan's research tree, fits a chassis's and a turret's slots
with their ``_df`` parts as soon as either is chosen, rates the design from the
live object it assembles, and hands an accepted design to the factory as a
``.dat``.  ``docs/38-designs.md`` reads every rule here; ``openparkan verify``
re-derives the recording's unit box from it (``check_designs``).

What this module does not model is the screen: the pages, the rows and the
buttons are docs/37's.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field
from pathlib import Path

from . import control, objects, profiles, research, units
from .nres import NResArchive

#: The mesh stream that carries one length-prefixed label per node: a socket's
#: part prefix, ``e_tur_bb`` on a chassis, ``universal_bl`` on a turret.
SOCKET_LABEL_STREAM = 10

#: What a slot's default part adds to the slot's label (``iron3d.dll:0x100520bc``).
DEFAULT_SUFFIX = "_df"
#: Slot labels the chassis fit leaves alone, and the one it files on the armour page.
BRAIN_PREFIX = "i_brn_"
ARMOUR_PREFIX = "i_arm_"
TURRET_PREFIX = "e_tur_"
GUN_PREFIX = "e_gun_"
#: The chassis page's prefixes, one more size a grade: tiny, small, medium, large
#: (``0x10048b38``).  The grade is the factory's size class.
CHASSIS_PREFIXES = ("r_t", "r_l", "r_m", "r_b")
#: A gun socket label ending in this letter takes both cannons and launchers.
EITHER_GUN = "r"
GUN_KINDS = ("c", "l")
#: A page holds at most this many rows (``0x1004878b``).
PAGE_ROWS = 64

#: ``Iron_3D.ini``'s ``[TEMP]``, as shipped: the ranges the unit box's two
#: percentages are taken over (``0x1006f6d0``).
TEMP_SECTION = "TEMP"
OFFENCE_RANGE = (0.0, 6550.0)
DEFENCE_RANGE = (0.0, 24400.0)

#: The unit box's colours: labels green, figures lavender, a full payload red.
LABEL_COLOUR = 0xFF00FF00
FIGURE_COLOUR = 0xFFB4B4FF
FULL_COLOUR = 0xFFFF0000

#: The archive every part of a written design is named in.
LIBRARY = "objects.rlb"
#: The part classes a written design's components carry.
CLASS_CHASSIS, CLASS_TURRET, CLASS_ARMOUR, CLASS_INTERNAL, CLASS_GUN, CLASS_CLIP = range(6)

#: The three letters of a unit's code (``iron3d.dll:0x10076270``).
SIZE_LETTERS = {1: "T", 2: "S", 3: "M", 4: "L"}
CHASSIS_LETTERS = {1: "F", 2: "S", 3: "W", 4: "T", 5: "A", 6: "U"}
ROLE_LETTERS = {objects.TYPE_BUILDER: "B", objects.TYPE_TRANSPORT: "T",
                objects.TYPE_WARRIOR: "W", objects.TYPE_HQ: "C", objects.TYPE_HERO: "H"}
#: The class word after the code, as ``iron3d.dll`` string ids (``0x10076490``).
CLASS_WORDS = {objects.TYPE_TRANSPORT: 6200, objects.TYPE_BUILDER: 6201,
               objects.TYPE_WARRIOR: 6202, objects.TYPE_HQ: 6203, objects.TYPE_HERO: 6204}
UNKNOWN_WORD = 6205
#: What an unbuilt design numbers itself (``0x100765e0``).
UNBUILT = "X"


def socket_labels(blob: bytes) -> list[str]:
    """Stream 10: each node's label, ``uint32`` length then that many bytes and a NUL."""
    out: list[str] = []
    at = 0
    while at + 4 <= len(blob):
        size = struct.unpack_from("<I", blob, at)[0]
        text = blob[at + 4:at + 4 + size].split(b"\0")[0]
        out.append(text.decode("latin-1"))
        at += 4 + (size + 1 if size else 0)
    if at != len(blob):
        raise ValueError(f"socket labels run {at} bytes into {len(blob)}")
    return out


def gun_prefixes(label: str) -> tuple[str, ...]:
    """The gun page a turret socket offers: ``e_gun_`` and the label's last two letters,
    both cannons and launchers when the last is ``r`` (``0x10048338``)."""
    size, kind = label[-2:-1], label[-1:]
    if kind == EITHER_GUN:
        return tuple(f"{GUN_PREFIX}{size}{k}" for k in GUN_KINDS)
    return (f"{GUN_PREFIX}{size}{kind}",)


def chassis_prefixes(grade: int) -> tuple[str, ...]:
    """The chassis page for a factory of size class ``grade``, 1 to 4."""
    return CHASSIS_PREFIXES[:max(0, min(grade, len(CHASSIS_PREFIXES)))]


def percent(value: float, span: tuple[float, float]) -> int:
    """A figure over one of ``[TEMP]``'s ranges, as the unit box prints it (``0x100700e3``)."""
    low, high = span
    if not value > low:
        return 0
    if not value < high:
        return 100
    return round(100.0 / (high - low) * (value - low))


def code(size_class: int, chassis_type: int, type_word: int) -> str:
    """The three letters: size, chassis and class, ``?`` for any it does not know."""
    return (SIZE_LETTERS.get(size_class, "?") + CHASSIS_LETTERS.get(chassis_type, "?")
            + ROLE_LETTERS.get(type_word, "?"))


def name(letters: str, number: int, word: str) -> str:
    """``LFW-X Warrior`` in the constructor, ``LFW-2 Warrior`` once built."""
    return f"{letters}-{number if number else UNBUILT} {word}"


class Catalogue:
    """What one clan's research tree lets the constructor offer (``0x1008a780``).

    A part is offered when its name starts with a page's prefix and the item that
    researches it is in the tree and researched -- unless ``FULL_RESEARCH_TREE``
    is set, when every part of the prefix is.  Rows keep the tree's ``TRFB`` order.
    """

    def __init__(self, tree: research.Tree, full: bool = False):
        self.tree = tree
        self.full = full
        self._item = {part.lower(): item for item in tree.items for part in item.parts}
        self._spelling = {part.lower(): part for part in tree.part_ids}

    def spelling(self, part: str) -> str:
        """A part id as the tree writes it, ``R_B_02`` for ``r_b_02``."""
        return self._spelling.get(part.lower(), part)

    def item(self, part: str) -> research.Item | None:
        return self._item.get(part.lower())

    def offered(self, part: str) -> bool:
        item = self.item(part)
        return item is not None and (self.full or (item.in_tree and item.researched))

    def page(self, prefixes: tuple[str, ...]) -> list[str]:
        rows = [p for p in self.tree.part_ids
                if any(p.lower().startswith(x.lower()) for x in prefixes) and self.offered(p)]
        return rows[:PAGE_ROWS]

    def default(self, label: str) -> str | None:
        """The part a slot labelled ``label`` is filled with, if it is offered."""
        part = label.lower() + DEFAULT_SUFFIX
        return part if self.offered(part) else None


@dataclass
class Part:
    """One node of a design: the part, where it attaches, what hangs under it."""

    part: str
    #: A node of the parent's mesh for a turret or gun; the parent controller's
    #: slot for an internal part or clip; -1 on the chassis.
    attach: int
    kind: int
    children: list[Part] = field(default_factory=list)

    @classmethod
    def from_unit(cls, unit: objects.UnitDefinition) -> Part:
        """A written design read back into its tree."""
        parents = unit.parents()
        nodes = [cls(c.ref.member, c.attach_node, c.class_id) for c in unit.components]
        for i, parent in enumerate(parents):
            if parent >= 0:
                nodes[parent].children.append(nodes[i])
        return nodes[0]

    def walk(self):
        yield self
        for child in self.children:
            yield from child.walk()


@dataclass(frozen=True)
class Rating:
    """The unit box's figures (``iron3d.dll:0x1006fc00``), in the game's units."""

    #: Total mass and spare payload (properties 124 and 137), kg.
    mass: float
    spare: float
    #: The live top speed (property 145), m/s.
    speed: float
    #: Defence and offence before ``[TEMP]``'s normalisation (177 and 178).
    defence: float
    offence: float
    #: The fitted radar's range (property 0x50), 0 with none.
    sensor_range: float

    @property
    def full(self) -> bool:
        """Red, and no accepting: the spare payload is not above 0."""
        return not self.spare > 0

    def lines(self, offence: tuple[float, float] = OFFENCE_RANGE,
              defence: tuple[float, float] = DEFENCE_RANGE) -> list[str]:
        return [f"{self.mass * 0.001:.0f} / {self.spare * 0.001:.0f} t",
                f"{self.speed * control.KMH_PER_MS:.0f} kph",
                f"{percent(self.defence, defence)} %",
                f"{percent(self.offence, offence)} %",
                f"{self.sensor_range:.0f} m"]


class Designer:
    """Designs out of one installation, offered from one research tree."""

    def __init__(self, shop: units.Workshop, catalogue: Catalogue):
        self.shop = shop
        self.catalogue = catalogue

    def labels(self, part: str) -> list[str]:
        """A part's socket labels, node by node."""
        record = self.shop.library.get(part)
        ref = record.mesh if record else None
        if not ref:
            return []
        inner = NResArchive(self.shop.armoury.read(ref), ref.member)
        blob = {e.type_id: inner.read(e) for e in inner}.get(SOCKET_LABEL_STREAM, b"")
        return socket_labels(blob)

    def defaults(self, part: str) -> list[Part]:
        """Every labelled slot of ``part`` but a brain's, filled with its ``_df`` part
        when that is offered (``0x10051fdb``)."""
        parsed = self.shop.armoury.controller(part)
        out = []
        for slot, component in enumerate(parsed.components if parsed else ()):
            label = component.label.lower()
            if not label or label.startswith(BRAIN_PREFIX):
                continue
            chosen = self.catalogue.default(label)
            if chosen:
                kind = CLASS_ARMOUR if label.startswith(ARMOUR_PREFIX) else (
                    CLASS_CLIP if part.lower().startswith(GUN_PREFIX) else CLASS_INTERNAL)
                out.append(Part(chosen, slot, kind))
        return out

    def chassis(self, part: str) -> Part:
        return Part(part, -1, CLASS_CHASSIS, self.defaults(part))

    def fit_turret(self, design: Part, part: str) -> Part:
        """The turret on the chassis node labelled ``e_tur_``, its slots filled."""
        node = next(i for i, s in enumerate(self.labels(design.part))
                    if s.lower().startswith(TURRET_PREFIX))
        design.children = [c for c in design.children if c.kind != CLASS_TURRET]
        turret = Part(part, node, CLASS_TURRET, self.defaults(part))
        design.children.append(turret)
        return turret

    def fit_gun(self, turret: Part, part: str, socket: int) -> Part:
        """A gun on turret node ``socket``, its clip slot filled."""
        turret.children = [c for c in turret.children
                           if not (c.kind == CLASS_GUN and c.attach == socket)]
        gun = Part(part, socket, CLASS_GUN, self.defaults(part))
        turret.children.append(gun)
        return gun

    def unit(self, design: Part, type_word: int = 0) -> objects.UnitDefinition:
        """The design as the ``.dat`` tree the constructor writes: every node's slots
        and sockets in ascending order."""
        components = []

        def put(node: Part) -> None:
            item = self.catalogue.item(node.part)
            label = f"{item.name} ({item.code})" if item else node.part
            children = sorted(node.children, key=lambda c: (c.kind in (CLASS_TURRET, CLASS_GUN),
                                                            c.attach))
            components.append(objects.Component(
                ref=objects.ResourceRef(LIBRARY, self.catalogue.spelling(node.part)),
                label=label, flags=1,
                attach_node=node.attach, class_id=node.kind, child_count=len(children)))
            for child in children:
                put(child)

        put(design)
        return objects.UnitDefinition(source=Path("<design>"), kind=type_word,
                                      components=components)

    def rate(self, design: Part) -> Rating | None:
        """The unit box's figures for ``design``, from the object it assembles."""
        assembly = self.shop.assemble(self.unit(design))
        if assembly is None:
            return None
        load = assembly.load
        engines = sum(d.values[0] for d in assembly.of_type(control.ENGINE_TYPE))
        spare = load.spare
        ratio = spare / assembly.payload if assembly.payload else 0.0
        speed = min(assembly.top_speed, assembly.top_speed * engines * (1 + ratio) / 2)
        return Rating(mass=load.total, spare=spare, speed=speed,
                      defence=defence(assembly), offence=self.offence(assembly),
                      sensor_range=next((d.values[3] for d in
                                         assembly.of_type(control.RADAR_TYPE)), 0.0))

    def offence(self, assembly: units.Assembly) -> float:
        """Every gun's round damage times its shots a second (``Control.dll:0x1002b62b``)."""
        total = 0.0
        for gun in assembly.of_type(control.GUN_TYPE):
            round_ = self.shop.armoury.round(gun.resource.member) if gun.resource.member else None
            if round_ is not None:
                total += round_.damage * 1000.0 / max(gun.values[control.GUN_INTERVAL], 1.0)
        return total

    def letters(self, design: Part, type_word: int) -> str:
        record = self.shop.library.get(design.part)
        profile = self.shop.profiles.get(record.profile or "", {}) if record else {}
        kind = profile.get("ChassisType")
        return code(profiles.CHASSIS_SIZE.get(design.part[2:3].lower(), 0),
                    int(kind.value) if kind else 0, type_word)

    def type_word(self, design: Part) -> int:
        """What the design is: its turret's role, an animal for an ``a`` chassis, else 0."""
        if design.part.lower().startswith("a"):
            return objects.TYPE_ANIMAL
        turret = next((c for c in design.children if c.kind == CLASS_TURRET), None)
        item = self.catalogue.item(turret.part) if turret else None
        if item is None or len(item.tail) < 5:
            return 0
        return objects.part_type(item.tail[1], item.tail[2], item.tail[4], item.tail[0])

    def price(self, design: Part) -> tuple[float, float, bool]:
        """Build ore and energy summed over every part, and whether all are researched
        (``Behavior.dll:0x10029810``)."""
        ore = energy = 0.0
        ok = True
        for node in design.walk():
            item = self.catalogue.item(node.part)
            if item is None or not (item.in_tree and item.researched):
                ok = False
                continue
            energy += item.build_cost[0]
            ore += item.build_cost[1]
        return ore, energy, ok


def defence(assembly: units.Assembly) -> float:
    """Property 177 (``Control.dll:0x10013940``): the life of the first node with any
    area over armour's share kept, plus the deflector's first coefficient times the
    shield generator's sector maximum when the unit has both."""
    life = next((n.life for n in assembly.nodes if n.area != 0), 0.0)
    armour = assembly.of_type(control.ARMOUR_TYPE)
    kept = armour[-1].values[1] if armour else 1.0
    value = life / kept if kept else float("inf")
    shields = assembly.of_type(control.FIGHT_SHIELD_TYPE)
    deflectors = assembly.of_type(control.DEFLECTOR_TYPE)
    if shields and deflectors:
        value += deflectors[0].values[0] * shields[0].values[0]
    return value


def dat_bytes(unit: objects.UnitDefinition) -> bytes:
    """A design as the ``.dat`` the constructor and the factory write."""
    out = bytearray(struct.pack("<II", objects.DAT_MAGIC, unit.kind))
    for c in unit.components:
        out += c.ref.library.encode("latin-1").ljust(objects.NAME_FIELD, b"\0")
        out += c.ref.member.encode("latin-1").ljust(objects.NAME_FIELD, b"\0")
        out += struct.pack("<Ii", c.flags, c.attach_node)
        out += c.label.encode("latin-1")[:objects.NAME_FIELD - 1].ljust(objects.NAME_FIELD, b"\0")
        out += struct.pack("<Ii", c.class_id, c.child_count)
    return bytes(out)
