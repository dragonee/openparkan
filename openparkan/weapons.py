"""Weapons: a gun, the clip it takes, and the round it fires.

Three ``objects.rlb`` records make a weapon.  The **gun** (``e_gun_*``, an
``EXTO``) has a controller in ``guns.rlb`` with one class-2 component -- or
type 30 on the builders' beams -- holding its magazine, capacitor, energy a
shot and interval, its barrels, and a **slot** label ``i_cNN_<size>`` when it
takes clips.  The **clip** (``i_cNN_*``, an ``INTO``) repeats its gun's numbers
with its own round count and mass.  The **round** is a ``BULL`` record in
``weapon.rlb``: its controller's third triple is its speed and +108 its range,
its ``.ndp`` hit points plus its explosion's damage are its damage, and a
class-17 component makes it guided.

The damage is what ``Control.dll:0x10013620`` sums for the stat panel, at a
level ratio of 1: it ignores armour and shields.  See ``docs/29-weapons.md``.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from . import control, effects, objects
from .nres import NResArchive

#: The first letter after ``e_gun_<size>``: what a gun is, by the catalogue.
GUN_LETTERS = {"c": "gun", "l": "launcher", "s": "builder", "f": "fortification"}


@dataclass(frozen=True)
class Round:
    member: str
    #: m/s, the controller's third triple.
    speed: float
    #: m, the controller's +108: how far from where it was fired a round gets.
    range: float
    #: Its nodes' hit points plus their explosions' damage.
    damage: float
    #: The first node's explosion kind, ``effects.HIT_*``, or None.
    kind: int | None
    #: The first node's explosion radius where it is a blast, else None.
    blast: float | None
    #: Whether it carries a class-17 seeker.
    guided: bool
    #: The seeker's cone half-angle in radians, and how near a target must be
    #: for it to follow, in m; zero on an unguided round.
    cone: float = 0.0
    reach: float = 0.0
    #: The most the round turns a second, in radians: its controller's turn
    #: rate, which caps the steering a seeker asks for (``Control.dll:0x1000cde5``).
    turn_rate: float = 0.0


@dataclass(frozen=True)
class Gun:
    part: str
    type_id: int
    #: The clip family it takes, ``i_c05_l``, or "" for none.
    slot: str
    magazine: int
    capacitor: float
    shot_energy: float
    interval_ms: float
    barrels: int
    #: Barrels whose section-2 record names a control point.
    beams: int
    salvo: bool
    round: Round | None
    #: Each barrel's stroke in ms, which comes before the interval.
    stroke_ms: tuple[float, ...] = ()

    @property
    def uses_clips(self) -> bool:
        return self.magazine != control.UNLIMITED

    @property
    def shots_per_second(self) -> float:
        """The stat panel's figure, ``1000 / max(1, interval)``.

        It leaves out the barrel's stroke; ``fire_rate`` counts it.
        """
        return 1000.0 / max(1.0, self.interval_ms)

    @property
    def shot_ms(self) -> float:
        """A barrel's stroke plus the interval: the time one shot takes."""
        if not self.stroke_ms:
            return self.interval_ms
        stroke = (max(self.stroke_ms) if self.salvo
                  else sum(self.stroke_ms) / len(self.stroke_ms))
        return stroke + self.interval_ms

    @property
    def fire_rate(self) -> float:
        """Shots a second as the gun takes them, stroke and interval."""
        return 1000.0 / max(1.0, self.shot_ms)

    @property
    def energy_per_second(self) -> float:
        return self.shot_energy * self.shots_per_second

    @property
    def damage_per_second(self) -> float:
        if self.round is None:
            return 0.0
        return self.round.damage * self.shots_per_second * (self.beams if self.salvo else 1)


@dataclass(frozen=True)
class Clip:
    part: str
    #: The gun slot it fills, the first seven letters of its name.
    family: str
    rounds: int
    mass: float
    round: str


class Armoury:
    """Guns, clips and rounds out of one installation, read once each."""

    def __init__(self, game: str | Path):
        self.game = Path(game)
        self.library = objects.ObjectLibrary(self.game / "objects.rlb")
        self._archives: dict[str, NResArchive] = {}
        self._rounds: dict[str, Round | None] = {}

    def read(self, ref: objects.ResourceRef) -> bytes:
        """A member of any archive, opened once."""
        key = ref.library.lower()
        if key not in self._archives:
            self._archives[key] = NResArchive.open(self.game / key)
        return self._archives[key].read_name(ref.member)

    def controller(self, part: str) -> control.Controller | None:
        """The controller an ``objects.rlb`` record names, or None."""
        record = self.library.get(part)
        ref = record.slot_with_suffix("ctl") if record else None
        return control.parse(self.read(ref)) if ref else None

    def values(self, part: str) -> tuple[float, ...]:
        """The sixteen values of a gun's or clip's firing component, or ()."""
        parsed = self.controller(part)
        firing = [p for p in parsed.components
                  if p.type_id in (control.GUN_TYPE, control.BUILDER_TYPE)] if parsed else []
        return firing[0].values if firing else ()

    def clip_values(self, part: str) -> tuple[float, float, float]:
        """A clip's capacitor, energy a shot and interval -- its gun's, if it fits."""
        values = self.values(part)
        return (values[control.GUN_CAPACITOR], values[control.GUN_SHOT_ENERGY],
                values[control.GUN_INTERVAL]) if values else (0.0, 0.0, 0.0)

    def round(self, member: str) -> Round | None:
        key = member.lower()
        if key not in self._rounds:
            self._rounds[key] = self._load_round(member)
        return self._rounds[key]

    def _load_round(self, member: str) -> Round | None:
        record = self.library.get(member)
        if record is None or record.damage is None:
            return None
        parsed = control.parse(self.read(record.slot_with_suffix("ctl")))
        rows = objects.parse_damage(self.read(record.damage), record.damage.member)
        damage = sum(row.durability for row in rows)
        first = None
        for i, row in enumerate(rows):
            if not row.explosion:
                continue
            blast = effects.parse_explosion(self.read(row.explosion), row.explosion.member)
            damage += blast.damage
            if i == 0:
                first = blast
        seeker = next((p for p in parsed.components if p.type_id == control.SEEKER_TYPE), None)
        return Round(
            member=member,
            speed=parsed.triples[control.TRIPLE_TOP_SPEED][1],
            range=parsed.bounds[0],
            damage=damage,
            kind=first.kind if first else None,
            blast=first.radius if first and first.kind == effects.HIT_AREA else None,
            guided=seeker is not None,
            cone=seeker.values[0] if seeker else 0.0,
            reach=seeker.values[1] if seeker else 0.0,
            turn_rate=max(parsed.triples[control.TRIPLE_TURN]),
        )

    def gun(self, part: str) -> Gun | None:
        """The gun an ``e_gun_*`` record is, or None where it has none."""
        parsed = self.controller(part)
        firing = [p for p in parsed.components
                  if p.type_id in (control.GUN_TYPE, control.BUILDER_TYPE)] if parsed else []
        if not firing:
            return None
        one = firing[0]
        values = one.values
        return Gun(
            part=part,
            type_id=one.type_id,
            slot=one.label.lower(),
            magazine=int(values[control.GUN_MAGAZINE]),
            capacitor=values[control.GUN_CAPACITOR],
            shot_energy=values[control.GUN_SHOT_ENERGY],
            interval_ms=values[control.GUN_INTERVAL],
            barrels=len(one.entries),
            beams=sum(1 for e in one.entries
                      if 0 <= e < len(parsed.points) and parsed.points[e] != -1),
            salvo=bool(one.flags & control.SALVO),
            round=self.round(one.resource.member) if one.resource.member else None,
            stroke_ms=tuple(parsed.channels[e].stroke_ms for e in one.entries
                            if 0 <= e < len(parsed.channels)),
        )

    def clip(self, part: str) -> Clip | None:
        """The clip an ``i_cNN_*`` record is, or None."""
        parsed = self.controller(part)
        loads = [p for p in parsed.components if p.type_id == control.GUN_TYPE] if parsed else []
        if not loads:
            return None
        one = loads[0]
        return Clip(part=part, family=part.lower()[:7],
                    rounds=int(one.values[control.GUN_MAGAZINE]),
                    mass=one.mass, round=one.resource.member)
