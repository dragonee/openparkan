"""A unit, described whole: what one ``UNITS/**/*.dat`` assembly is.

A robot is a chassis with a turret on it, internal parts in both, guns in the
turret's sockets and clips in the guns.  Every piece is read elsewhere --
``docs/28-chassis.md``, ``30-turrets.md``, ``29-weapons.md``, ``26-damage.md``,
``25-sensors.md``, ``31-packages.md``, ``32-builder.md`` -- and this module
puts them on one sheet: ``describe`` builds it, ``render`` prints it.

What the sheet does not do is simulate.  The figures are the parts' own, the
derived totals say so, and where the engine combines a chassis slot with a
fitted part it shows both (``docs/28-chassis.md``).
"""

from __future__ import annotations

import math
from dataclasses import dataclass, field
from pathlib import Path

from . import control, descriptions, mesh, objects, packages, profiles, weapons

#: What a unit's Type makes it.
ROLES = {
    objects.TYPE_TRANSPORT: "transport", objects.TYPE_BUILDER: "builder",
    objects.TYPE_WARRIOR: "warrior", objects.TYPE_HQ: "HQ", objects.TYPE_HERO: "hero",
    objects.TYPE_ANIMAL: "animal",
}
#: A part family by its name's first five letters, and the class it is.
PART_FAMILIES = {
    "i_eng": ("engine", control.ENGINE_TYPE),
    "i_pws": ("battery", control.POWER_STORE_TYPE),
    "i_fsh": ("shield generator", control.FIGHT_SHIELD_TYPE),
    "i_dsh": ("detection shield", control.DETECT_SHIELD_TYPE),
    "i_rps": ("repair unit", control.REPAIR_TYPE),
    "i_arm": ("armour", control.ARMOUR_TYPE),
    "i_rdr": ("radar", control.RADAR_TYPE),
    "i_def": ("deflector", control.DEFLECTOR_TYPE),
}


@dataclass
class Cost:
    research_energy: float = 0.0
    research_ore: float = 0.0
    build_energy: float = 0.0
    build_ore: float = 0.0

    @property
    def free(self) -> bool:
        return not any((self.research_energy, self.research_ore,
                        self.build_energy, self.build_ore))


@dataclass
class Chassis:
    part: str
    name: str
    code: str
    size: str
    locomotion: str
    #: km/h, m/s², t; payload None where it is the no-limit FLT_MAX.
    top_speed: float
    acceleration: float
    payload: float | None
    #: Degrees; None where the chassis ignores slope.
    slope: float | None
    #: kg: its nodes' density times their level-0 volume.
    body_mass: float
    hit_points: float
    #: The size of part each labelled slot takes, by family.
    slots: dict[str, str]
    #: What the chassis's own slot records carry -- its built-in battery
    #: (capacity, output) and engine (drive, draw).  The engine sums them with
    #: a fitted part's (docs/28-chassis.md).
    battery: tuple[float, float]
    engine: tuple[float, float]
    cost: Cost


@dataclass
class Part:
    family: str
    part: str
    name: str
    #: Where it is fitted: "chassis" or "turret".
    on: str
    #: The part's own figures, labelled, in the order they are shown.
    figures: list[tuple[str, str]]
    mass: float


@dataclass
class Weapon:
    socket: str
    gun: weapons.Gun
    name: str
    code: str
    kind: str
    clip: weapons.Clip | None = None
    clip_name: str = ""


@dataclass
class Turret:
    part: str
    name: str
    code: str
    mounting: str
    #: Gun sockets, each with the gun fitted there or None.
    sockets: dict[str, str | None]
    builtin_guns: int
    hit_points: float
    radar_takes: str
    deflector_takes: str
    hq: bool
    cost: Cost


@dataclass
class Unit:
    source: Path
    label: str
    type: int
    role: str
    profile: str | None
    size_class: int
    chassis: Chassis | None
    turret: Turret | None
    parts: list[Part] = field(default_factory=list)
    weapons: list[Weapon] = field(default_factory=list)
    #: The commander's packages, by menu: "hq" and "wingman".
    packages: dict[str, list[str]] = field(default_factory=dict)

    @property
    def may_capture(self) -> bool:
        return self.size_class <= packages.CAPTURE_SIZE

    @property
    def firepower(self) -> float:
        """Damage a second over every weapon: *derived*, before armour and shields."""
        return sum(w.gun.damage_per_second for w in self.weapons if w.gun.type_id == 2)

    @property
    def weapon_power(self) -> float:
        """Energy a second every weapon asks at full rate."""
        return sum(w.gun.energy_per_second for w in self.weapons if w.gun.type_id == 2)

    @property
    def reach(self) -> float:
        return max((w.gun.round.range for w in self.weapons if w.gun.round), default=0.0)


class Workshop:
    """Everything a sheet is read from, opened once for many units."""

    def __init__(self, game: str | Path):
        self.game = Path(game)
        self.armoury = weapons.Armoury(self.game)
        self.library = self.armoury.library
        self.catalogue = {k.lower(): v for k, v in descriptions.library(self.game).items()}
        self.profiles = profiles.load(self.game)

    def _entry(self, part: str) -> descriptions.Description | None:
        return self.catalogue.get(part.lower())

    def _cost(self, part: str) -> Cost:
        entry = self._entry(part)
        if entry is None:
            return Cost()
        return Cost(entry.research_energy, entry.research_ore,
                    entry.build_energy, entry.build_ore)

    def _mesh(self, record: objects.ObjectRecord) -> mesh.ObjectMesh | None:
        ref = record.mesh
        return mesh.parse(self.armoury.read(ref), ref.member) if ref else None

    def _damage(self, record: objects.ObjectRecord) -> list[objects.NodeDamage]:
        ref = record.damage
        return objects.parse_damage(self.armoury.read(ref), ref.member) if ref else []

    def chassis(self, part: str) -> Chassis | None:
        record = self.library.get(part.lower())
        parsed = self.armoury.controller(part.lower())
        if record is None or parsed is None:
            return None
        entry = self._entry(part)
        profile = self.profiles.get(record.profile or "", {})
        kind = profile.get("ChassisType")
        model = self._mesh(record)
        rows = self._damage(record)
        body = sum(row.unknown * model.node_volume(i)
                   for i, row in enumerate(rows[:len(model.nodes)])) if model else 0.0
        return Chassis(
            part=part,
            name=entry.name if entry else "",
            code=entry.code if entry else "",
            size=part[2].lower(),
            locomotion=profiles.CHASSIS_TYPE.get(int(kind.value), "?") if kind else "?",
            top_speed=parsed.triples[control.TRIPLE_TOP_SPEED][1] * control.KMH_PER_MS,
            acceleration=2 * parsed.triples[control.TRIPLE_ACCELERATION][1],
            payload=None if parsed.payload >= control.FLT_MAX else parsed.payload / 1000,
            slope=math.degrees(parsed.cone) if parsed.mode == control.SLOPE_MODE else None,
            body_mass=body,
            hit_points=rows[0].durability if rows else 0.0,
            slots={PART_FAMILIES[p.slot[:5]][0]: p.slot[-1] for p in parsed.components
                   if p.slot},
            battery=next(((p.values[0], p.power) for p in parsed.components
                          if p.type_id == control.POWER_STORE_TYPE), (0.0, 0.0)),
            engine=next(((p.values[0], p.power) for p in parsed.components
                         if p.type_id == control.ENGINE_TYPE), (0.0, 0.0)),
            cost=self._cost(part),
        )

    def part(self, member: str, on: str) -> Part | None:
        family = PART_FAMILIES.get(member.lower()[:5])
        parsed = self.armoury.controller(member.lower())
        if family is None or parsed is None:
            return None
        name, type_id = family
        one = next((p for p in parsed.components if p.type_id == type_id), None)
        if one is None:
            return None
        entry = self._entry(member)
        return Part(family=name, part=member, name=entry.name if entry else "", on=on,
                    figures=_figures(type_id, one), mass=one.mass)

    def turret(self, unit: objects.UnitDefinition, index: int) -> Turret | None:
        member = unit.components[index].ref.member
        record = self.library.get(member.lower())
        parsed = self.armoury.controller(member.lower())
        if record is None or parsed is None:
            return None
        model = self._mesh(record)
        rows = self._damage(record)
        names = [n.name for n in model.nodes] if model else []
        parents = unit.parents()
        mounted = {names[c.attach_node]: c.ref.member
                   for j, c in enumerate(unit.components)
                   if parents[j] == index and c.ref.member.lower().startswith("e_gun_")
                   and 0 <= c.attach_node < len(names)}
        sockets = {n: mounted.get(n) for n in names
                   if n.startswith("Base_") and n not in objects.TURRET_MOUNT_NODES}
        by_class = {p.type_id: p for p in parsed.components}
        body = by_class.get(control.TURRET_TYPE)
        entry = self._entry(member)
        return Turret(
            part=member,
            name=entry.name if entry else "",
            code=entry.code if entry else "",
            mounting="hung under a flyer" if member.lower()[7:8] == "b" else "upright",
            sockets=sockets,
            builtin_guns=sum(1 for p in parsed.components if p.type_id == control.GUN_TYPE),
            hit_points=rows[body.node].durability if body and body.node < len(rows) else 0.0,
            radar_takes=by_class[control.RADAR_TYPE].label if control.RADAR_TYPE in by_class
            else "",
            deflector_takes=by_class[control.DEFLECTOR_TYPE].label
            if control.DEFLECTOR_TYPE in by_class else "",
            hq=bool(body and body.flags & control.MOUNT_HQ),
            cost=self._cost(member),
        )

    def describe(self, dat: str | Path) -> Unit:
        """The whole sheet for one assembly."""
        unit = objects.load_unit(Path(dat))
        parents = unit.parents()
        root = unit.components[0].ref.member if unit.components else ""
        chassis = self.chassis(root) if root.lower().startswith("r_") else None
        size = profiles.CHASSIS_SIZE.get(root[2:3].lower(), 9) if chassis else 9
        sheet = Unit(
            source=Path(dat),
            label=unit.label,
            type=unit.kind,
            role=ROLES.get(unit.kind, "building" if unit.is_building else "?"),
            profile=packages.PROFILE_BY_TYPE.get(unit.kind),
            size_class=size,
            chassis=chassis,
            turret=None,
        )
        if unit.kind and unit.kind & packages.ROBOT_ANY == unit.kind:
            offered = packages.packages_for(unit.kind, size)
            sheet.packages = {menu: [p.label for p in offered if menu in p.menus]
                              for menu in (packages.HQ, packages.WINGMAN)}
        turret_names: dict[int, list[str]] = {}
        for i, c in enumerate(unit.components):
            member = c.ref.member
            low = member.lower()
            if low.startswith("e_tur_") and sheet.turret is None:
                sheet.turret = self.turret(unit, i)
                record = self.library.get(low)
                model = self._mesh(record) if record else None
                turret_names[i] = [n.name for n in model.nodes] if model else []
            elif low[:5] in PART_FAMILIES:
                owner = unit.components[parents[i]].ref.member.lower() if parents[i] >= 0 else ""
                one = self.part(member, "turret" if owner.startswith("e_tur_") else "chassis")
                if one:
                    sheet.parts.append(one)
            elif low.startswith("e_gun_"):
                gun = self.armoury.gun(low)
                if gun is None:
                    continue
                entry = self._entry(member)
                names = turret_names.get(parents[i], [])
                socket = names[c.attach_node] if 0 <= c.attach_node < len(names) else ""
                clip = next((unit.components[j].ref.member for j in range(len(parents))
                             if parents[j] == i
                             and unit.components[j].ref.member.lower().startswith("i_c")), None)
                clip_entry = self._entry(clip) if clip else None
                sheet.weapons.append(Weapon(
                    socket=socket, gun=gun,
                    name=entry.name if entry else member, code=entry.code if entry else "",
                    kind=entry.sub if entry else "",
                    clip=self.armoury.clip(clip.lower()) if clip else None,
                    clip_name=clip_entry.name if clip_entry else (clip or "")))
        return sheet


def _figures(type_id: int, part: control.Component) -> list[tuple[str, str]]:
    """The few numbers that say what a part does, by its class."""
    v = part.values
    if type_id == control.ENGINE_TYPE:
        return [("drive", f"{v[0]:g}"), ("draw", f"{part.power:g}/s at full speed")]
    if type_id == control.POWER_STORE_TYPE:
        capacity = "generator" if v[0] < 0 else f"{v[0]:g}"
        return [("holds", capacity), ("gives", f"{part.power:g}/s")]
    if type_id == control.FIGHT_SHIELD_TYPE:
        return [("per sector", f"{v[0]:g} x 6"), ("recharge", f"{v[1]:g}/s"),
                ("costs", f"{v[2]:g} a point")]
    if type_id == control.DETECT_SHIELD_TYPE:
        return [("hides mass/electronics/drive", f"{v[0]:g}/{v[1]:g}/{v[2]:g}"),
                ("camouflage", f"{v[3]:g} at {v[4]:g}/s" if v[3] else "none")]
    if type_id == control.REPAIR_TYPE:
        return [("regenerates", f"{v[0]:g} HP/s"), ("costs", f"{v[1]:g} a point")]
    if type_id == control.ARMOUR_TYPE:
        through = (1 - v[1]) / v[2] if v[2] > 0 else None
        return [("keeps", f"{v[1]:.0%} of a small hit"),
                ("stops nothing from", f"{through:,.0f}" if through else "-")]
    if type_id == control.RADAR_TYPE:
        return [("range", f"{v[control.RADAR_RANGE]:g} m"),
                ("sensitivity", f"{v[0]:g}/{v[1]:g}/{v[2]:g}")]
    if type_id == control.DEFLECTOR_TYPE:
        return [("stops", f"{v[0]:.0%} of a sector")]
    return []


def render(unit: Unit) -> list[str]:
    """The sheet as text, one line a fact."""
    out = [f"{unit.label}  ({unit.source.name})",
           f"  role      {unit.role}, Type {unit.type:#x}"
           + (f", profile {unit.profile}" if unit.profile else "")]
    c = unit.chassis
    if c:
        payload = f"{c.payload:g} t" if c.payload is not None else "no limit"
        slope = f"brakes past {c.slope:.0f} deg" if c.slope is not None else "ignores slope"
        out += [
            f"  chassis   {c.name} {c.code} ({c.part}), {c.locomotion}, size {c.size}",
            f"            {c.top_speed:g} km/h, {c.acceleration:g} m/s2, payload {payload}, "
            f"{slope}",
            f"            body {c.body_mass:,.0f} kg, {c.hit_points:,.0f} HP; build "
            f"{c.cost.build_energy:g} E / {c.cost.build_ore:g} O"
            + (", not for the player" if c.cost.free else ""),
            f"            built in: battery {c.battery[0]:,.0f} at {c.battery[1]:g}/s, "
            f"engine {c.engine[0]:g} drawing {c.engine[1]:g}/s",
            "            slots " + ", ".join(f"{k} {s}" for k, s in c.slots.items()),
        ]
    t = unit.turret
    if t:
        taken = sum(1 for g in t.sockets.values() if g)
        out += [
            f"  turret    {t.name} {t.code} ({t.part}), {t.mounting}"
            + (", HQ" if t.hq else ""),
            f"            {len(t.sockets)} sockets, {taken} filled"
            + (f", {t.builtin_guns} built-in guns" if t.builtin_guns else "")
            + f", {t.hit_points:,.0f} HP; takes {t.radar_takes or '-'}, "
            f"{t.deflector_takes or '-'}",
        ]
    for p in unit.parts:
        figures = "; ".join(f"{k} {v}" for k, v in p.figures)
        out.append(f"  {p.family:<17} {p.name or p.part} on the {p.on}: {figures}")
    for w in unit.weapons:
        g = w.gun
        if g.type_id == control.BUILDER_TYPE:
            out.append(f"  builder   {w.name} at {w.socket or '?'}: fires "
                       f"{g.round.member if g.round else '?'}")
            continue
        ammo = ("energy" if g.magazine == control.UNLIMITED and not g.slot
                else f"{w.clip_name}, {w.clip.rounds} rounds" if w.clip
                else "unlimited" if g.magazine == control.UNLIMITED else "no clip fitted")
        r = g.round
        hit = ""
        if r:
            hit = f"{r.damage:g} a round"
            hit += f", {r.blast:g} m blast" if r.blast else ""
            hit += ", guided" if r.guided else ""
            hit += f", {r.range:g} m"
        out.append(f"  weapon    {w.name} {w.code} at {w.socket or '?'}: {ammo}; "
                   f"{g.shots_per_second:.2f}/s, {hit}; {g.damage_per_second:.0f} dmg/s, "
                   f"{g.energy_per_second:.2f} E/s")
    if unit.weapons:
        out.append(f"  firepower {unit.firepower:.0f} a second (derived), weapons ask "
                   f"{unit.weapon_power:.2f} E/s, reach {unit.reach:g} m")
    if unit.role in ("builder", "transport"):
        out.append(f"  cargo     {profiles.TRANSPORT_MAX_ORE:g} ore, loaded and unloaded at "
                   f"{profiles.TRANSPORT_ORE_ON_PER_SECOND:g}/s")
    if unit.packages:
        out.append("  orders    " + ", ".join(unit.packages[packages.HQ])
                   + ("" if unit.may_capture else "; too big to capture"))
        out.append("  wingman   " + ", ".join(unit.packages[packages.WINGMAN]))
    return out
