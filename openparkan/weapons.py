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

import math
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
    #: The controller's mode (+104): 3 on the four flame rounds, which fall.
    mode: int = 0
    #: The seeker's value 2, in ms: how long its gun holds a target before a
    #: shot (``Control.dll:0x100298f1``); zero on an unguided round.
    lock_ms: float = 0.0

    @property
    def lobbed(self) -> bool:
        """Whether gravity pulls it, so its gun's mount aims above the target.

        A gun keeps 1 for such a round (``Control.dll:0x100297ef``), which
        switches off the sight convergence and switches on the mount's
        ballistic elevation (``0x10028401``).
        """
        return self.mode != 0


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
            mode=parsed.mode,
            lock_ms=seeker.values[2] if seeker else 0.0,
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


#: What a gun reports when its target gate holds a shot back (``+0x11c``):
#: no target, the target out of range, the target off the barrel.
GATE_NO_TARGET = 2
GATE_OUT_OF_RANGE = 7
GATE_OFF_BARREL = 8


@dataclass(frozen=True)
class TargetGate:
    """A gun's values 8, 10 and 9, as it fills them from the round it loads.

    The file's values are zero on every gun; the gun overwrites them
    (``Control.dll:0x100297e0``-``0x10029907``): the round's range, cut to its
    seeker's reach; the cosine of the seeker's cone, else -1; the seeker's
    value 2 in seconds, else -1.
    """

    range: float
    cone_cos: float
    lock_s: float

    @property
    def needs_target(self) -> bool:
        """Whether the gun holds its fire until it has a target."""
        return self.range > 0.0 and self.lock_s > 0.0


def target_gate(shot: Round | None) -> TargetGate:
    """The gate a gun loading ``shot`` sets up."""
    if shot is None:
        return TargetGate(0.0, -1.0, -1.0)
    reach = shot.range if shot.range > 0.0 else 0.0
    if not shot.guided:
        return TargetGate(reach, -1.0, -1.0)
    return TargetGate(min(reach, shot.reach), math.cos(shot.cone), shot.lock_ms * 0.001)


def gate_state(gate: TargetGate, distance: float | None, off_barrel_cos: float) -> int | None:
    """Why the gate holds a shot back, ``GATE_*``, or None when it lets one through.

    ``distance`` is from the unit to its target, None with no target;
    ``off_barrel_cos`` the cosine between the barrel point's direction and the
    line to the target (``Control.dll:0x10029d3a``-``0x10029edd``).  A gate that
    lets a shot through may still be counting its lock down.
    """
    if gate.range <= 0.0:
        return None
    if distance is None:
        return GATE_NO_TARGET if gate.lock_s > 0.0 else None
    if distance > gate.range:
        return GATE_OUT_OF_RANGE
    if gate.cone_cos > 0.0 and off_barrel_cos <= gate.cone_cos:
        return GATE_OFF_BARREL
    return None


def mount_aim(initial: float, pitch: float, arm: float) -> tuple[float, bool]:
    """Where a gun's mount heads, and whether the gun is ready, for a round that does not fall.

    ``arm`` is the progress of the gun's arm, 1 where it has none.  While the
    arm moves, the mount blends from its channel's initial value to the
    turret's pitch target and the gun may not fire; once it is out, the mount
    follows the pitch and the gun is ready (``Control.dll:0x10028200``).  A
    lobbed round adds an elevation and may still be refused; see
    ``lobbed_elevation``.
    """
    if arm < 1.0:
        return (1.0 - arm) * initial + arm * pitch, False
    return pitch, True


def lobbed_elevation(speed: float, gravity: float,
                     to: tuple[float, float, float]) -> float | None:
    """The angle a mount raises a falling round by to reach ``to``, or None.

    The flight time t solves ``|to + g t^2 / 2 z| = speed * t``; the lower arc
    is taken when both exist, and the answer is the angle between the launch
    direction and the straight line.  None when the target is out of reach,
    which leaves the gun not ready (``Control.dll:0x10028401``).  The mount
    then rises by ``0.83 x angle / span`` of its channel, signed by the
    turret's up.
    """
    x, y, z = to
    reach = speed * speed - gravity * z
    square = x * x + y * y + z * z
    disc = reach * reach - gravity * gravity * square
    if gravity <= 0.0 or square <= 0.0 or disc <= 0.0:
        return None
    scale = 2.0 / (gravity * gravity)
    near, spread = reach * scale, math.sqrt(disc) * scale
    t2 = near + spread if near < spread else near - spread
    if t2 <= 0.0:
        return None
    t = math.sqrt(t2)
    launch = (x / t, y / t, (z + 0.5 * gravity * t2) / t)
    dot = sum(a * b for a, b in zip(launch, to, strict=True))
    norm = math.sqrt(sum(a * a for a in launch)) * math.sqrt(square)
    return math.acos(max(-1.0, min(1.0, dot / norm)))


#: The AI fires a gun one shot at a time, when a product of scores clears a
#: threshold (``Behavior.dll:0x10024f07``): 0.45, or 0.85 on a building or a
#: unit in some states.
AI_FIRE_SCORE = 0.45
AI_FIRE_SCORE_HIGH = 0.85
#: A round doing this much damage or more is held back from most targets
#: (``0x10024d30``): the winged SSMs.
AI_HEAVY_DAMAGE = 10000.0
#: Nearer than this the distance score falls toward zero (``0x1001b5e0``).
AI_TOO_CLOSE = 5.0


def ai_distance_score(distance: float, rise: float, speed: float) -> float:
    """How good ``distance`` is for a gun whose round flies at ``speed``, to the AI.

    Behavior.dll's gun record keeps three distances from the round's speed:
    none nearer than 5 m, full out to ``(speed + 1) / 2``, none past
    ``2 (speed + 1)`` (``0x1001b4b0``); the score is then scaled by
    ``1 - rise / speed`` (``0x1001b9f0``).  ``rise`` is how far the firing
    unit's origin stands above its target's (``0x10024337``): a unit firing
    down loses score.
    """
    full = (speed + 1.0) * 0.5
    none = 2.0 * (speed + 1.0)
    if distance < AI_TOO_CLOSE:
        band = distance / AI_TOO_CLOSE
    elif distance <= full:
        band = 1.0
    elif distance < none:
        band = (distance - none) / (full - none)
    else:
        band = 0.0
    return band * (1.0 - rise / speed) if speed else 0.0


def ai_fire_wait(magazine: float, difficulty: float = 1.0) -> tuple[float, float]:
    """The AI's wait between one gun's shots: (least, random extra), in seconds.

    ``30 / magazine`` each when the magazine holds more than 2, else 0.5 and
    1.5; both are divided by the difficulty profile's value
    (``Behavior.dll:0x1001b650``).  The gun's own interval still applies.
    """
    difficulty = difficulty if difficulty > 0.0 else 1.0
    if magazine > 2.0:
        each = 30.0 / magazine / difficulty
        return each, each
    return 0.5 / difficulty, 1.5 / difficulty


#: What a device of each class weighs when the AI picks the part of a target
#: to aim at (``Behavior.dll:0x10025830``, the jump table at ``0x100259c8``
#: through the byte map at ``0x100259e8``): a deflector, a turret, a gun, an
#: engine, a radar, a power store, a fight shield, a repair system.  Any other
#: class weighs ``AI_PART_OTHER``.
AI_PART_WEIGHT = {21: 30.0, 1: 20.0, 2: 15.0, 5: 14.0, 8: 14.0, 19: 10.0, 9: 7.0, 15: 3.0}
AI_PART_OTHER = 1.0
#: What a running-gear node weighs in the same pick (``0x10059978``).
AI_GEAR_WEIGHT = 7.5
#: An AI unit's line of fire starts this share of its turret's sphere's radius
#: out from the centre (``0x10059968``), is a sphere this thick past the
#: unit's own side (``0x10024954``), and passes a face flagged ``0x20`` while
#: it is shorter than ``AI_LINE_NEAR`` (``0x1005960c``).
AI_LINE_FROM = 0.7
AI_LINE_RADIUS = 0.5
AI_LINE_NEAR = 20.0


def ai_part_weight(type_id: int, life: float = 1.0) -> float:
    """What a device of class ``type_id`` weighs in the AI's pick of a part.

    ``life`` is its node's life over its maximum: the weight is the class's
    figure times ``2 - life``, and a part with no life left weighs nothing.
    """
    if life <= 0.0:
        return 0.0
    return (2.0 - life) * AI_PART_WEIGHT.get(type_id, AI_PART_OTHER)


def ai_part(classes: list[int]) -> int | None:
    """Which of a whole target's devices the AI aims at: an index into ``classes``.

    The devices are walked from the last, and only a greater weight takes the
    pick, so among equals the last listed keeps it.  None for no devices.
    """
    best: int | None = None
    for i in reversed(range(len(classes))):
        if best is None or ai_part_weight(classes[i]) > ai_part_weight(classes[best]):
            best = i
    return best
