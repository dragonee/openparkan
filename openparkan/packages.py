"""Orders, the commander's packages, and the profiles that run them.

What a unit does is a stack of tasks in ``Behavior.dll``, and a task comes from
an **order**: ``varset.var`` declares their numbers and
``MTaskStack::CreateTaskFromOrder`` (``Behavior.dll:0x10033a80``) switches on
them.  The player gives orders through the commander's menus in
``iron3d.dll``, whose entries -- the manual's *packages* -- each offer one order
to the robot types in their mask.  A unit's Type (its ``.dat`` class word, see
``objects.TYPE_*``) picks both its menu entries and its behaviour profile.

Everything here is a number the game ships; see ``docs/31-packages.md``.
"""

from __future__ import annotations

import struct
from collections.abc import Sequence
from dataclasses import dataclass

from . import mission, objects

#: ``varset.var``'s ``ORDER_*`` numbers.
ORDERS = {
    "ORDER_ROBOT_STOP": 1, "ORDER_ROBOT_GO": 2, "ORDER_ROBOT_ATTACK": 3,
    "ORDER_ROBOT_PATROL": 4, "ORDER_ROBOT_SEARCH": 5, "ORDER_ROBOT_TRANSPORT": 6,
    "ORDER_ROBOT_BUILD": 7, "ORDER_ROBOT_RELOAD": 8, "ORDER_ROBOT_REPARE": 9,
    "ORDER_BUILDING_MINE": 10, "ORDER_BUILDING_CHARGE": 11,
    "ORDER_BUILDING_CONSTRUCT": 12, "ORDER_ROBOT_RANDOMGO": 13,
    "ORDER_ROBOT_CAPTURE": 17, "ORDER_ROBOT_SHUTDOWN": 19, "ORDER_ROBOT_LEAVE": 20,
    "ORDER_ROBOT_STAYGROUND": 21, "ORDER_ROBOT_FOLLOW": 22,
    "ORDER_ROBOT_GETONBOARD": 23, "ORDER_ROBOT_UPGRADE": 24,
}
STOP, GO, ATTACK, PATROL, SEARCH, TRANSPORT, BUILD, RELOAD = 1, 2, 3, 4, 5, 6, 7, 8
CAPTURE, STAYGROUND, FOLLOW, UPGRADE = 17, 21, 22, 24
RANDOMGO, SHUTDOWN, LEAVE = 13, 19, 20
#: The orders ``varset.var`` leaves undeclared, and no binary names.  14 is the
#: player's research of one technology by id (``iron3d.dll:0x100880b0``), 16
#: the AI's research of a design by name (``ai.dll:0x10011100``), 18 the
#: construction sphere a building runs, 15 an animal's migration.
RESEARCH_BY_ID, MIGRATE, RESEARCH_BY_NAME, SPHERE = 14, 15, 16, 18

#: An order's target kind.
TARGET_BY_LOGIC_ID = 0x201
TARGET_BY_PLACE = 0x202
TARGET_BY_TYPE = 0x203
TARGET_NOT_DEFINED = 0x204
TARGET_BY_NAME = 0x205

#: Every robot Type, as the menus' mask for "every robot".
ROBOT_ANY = 0x0103E000
#: The behaviour profile each Type loads (``Behavior.dll:0x10008a80``).
PROFILE_BY_TYPE = {
    objects.TYPE_TRANSPORT: "prof_trn.var", objects.TYPE_BUILDER: "prof_bld.var",
    objects.TYPE_WARRIOR: "prof_war.var", objects.TYPE_HQ: "prof_hq.var",
    objects.TYPE_HERO: "prof_hero.var", objects.TYPE_ANIMAL: "prof_animal.var",
}
#: A profile's fourteen task flags, in binding order.  Nothing was found that
#: reads them.
TASK_FLAGS = ("Task_Stop", "Task_Go", "Task_Attack", "Task_Search", "Task_Patrol",
              "Task_Reload", "Task_Repare", "Task_Transport", "Task_Build",
              "Task_RandomGo", "Task_Charge", "Task_Mine", "Task_Construct",
              "Task_Research")

#: The building types "Search and capture" looks for: all but the power mast,
#: the little teleport and the heavy tower -- and the plan skips main teleports
#: and bridges as well (``Behavior.dll:0x10030859``).
CAPTURE_TYPES = 0x8017365E
SEARCH_SKIPS = (0x80000200, 0x80001000)
#: The robot Types "Seek and destroy" hunts: transports, builders, warriors --
#: never an HQ or a hero (``0x10030d7e``).
HUNTED = 0x0100E000
#: How far a search looks for enemies, and how near they must be before a
#: capturer with nothing to take moves away from them.
HUNT_RANGE = 3000.0
FLEE_RANGE = 300.0
#: A generator counts at this fraction of its distance when a capturer picks a
#: building (``0x100308ac``).
GENERATOR_WEIGHT = 0.5
#: The search task's replan timers, ``(every, plus up to)`` seconds: capture
#: mode, and every other mode.
CAPTURE_TIMER = (3.0, 3.0)
SEARCH_TIMER = (15.0, 15.0)
#: Random roaming keeps this far inside the map (``0x10030f29``).
ROAM_MARGIN = 100.0
#: The type "Search minerals" looks for.
MINERALS = 0x10001000
#: The largest size class that may capture (``Behavior.dll:0x100301a9``).
CAPTURE_SIZE = 2

#: The status line's strings in ``iron3d.dll``, from this id on.
STATUS_FIRST = 6180
STATUS = ("no order", "stopping", "shutting down", "standing", "moving", "escaping",
          "following", "getting onboard", "attacking", "patrolling", "searching",
          "transporting", "building", "refitting", "repairing", "capturing", "upgrading",
          "unknown")
#: Which of ``STATUS`` an order shows (``iron3d.dll:0x10076f90``, a switch over
#: 0..24).  Every order not here -- 10 to 16, 18, above 24 -- shows "unknown".
STATUS_BY_ORDER = {
    STOP: 1, SHUTDOWN: 2, STAYGROUND: 3, GO: 4, LEAVE: 5, FOLLOW: 6, 23: 7, ATTACK: 8,
    PATROL: 9, SEARCH: 10, TRANSPORT: 11, BUILD: 12, RELOAD: 13, 9: 14, CAPTURE: 15,
    UPGRADE: 16,
}


def status(orders: Sequence[int]) -> str:
    """The status line for a unit whose order list is ``orders``, head first.

    The line is picked by the head order alone: a unit on the menu's Search and
    capture (``SEARCH``) reads "searching", and only ``CAPTURE`` "capturing".
    """
    if not orders:
        return STATUS[0]
    return STATUS[STATUS_BY_ORDER.get(orders[0], len(STATUS) - 1)]


# --------------------------------------------------------------------------
# What runs beside the task: the fire control, interrupts, the walker's speed
# --------------------------------------------------------------------------

#: The fire control's modes (``MBehaviour+0x35c``, ``Behavior.dll:0x100240a6``):
#: no target, the given target, the nearest hostile contact within
#: ``FIRE_RANGE``, and two weighted pickers.
FIRE_NONE, FIRE_GIVEN, FIRE_NEAREST, FIRE_WEIGHTED, FIRE_WEIGHTED_AREAL = 0, 1, 2, 3, 4
FIRE_RANGE = 500.0
#: The mode a task asks the fire control for when it starts.  A task not listed
#: asks for nothing and leaves the mode as it was; a new unit starts in
#: ``FIRE_NEAREST`` (``0x10023e80``).
FIRE_MODE_BY_ORDER = {
    ATTACK: FIRE_GIVEN,
    STOP: FIRE_NEAREST, GO: FIRE_NEAREST, PATROL: FIRE_NEAREST, SEARCH: FIRE_NEAREST,
    CAPTURE: FIRE_NEAREST, TRANSPORT: FIRE_NEAREST, RANDOMGO: FIRE_NEAREST,
    STAYGROUND: FIRE_NEAREST, FOLLOW: FIRE_NEAREST, LEAVE: FIRE_NEAREST,
    SHUTDOWN: FIRE_NONE, MIGRATE: FIRE_NONE,
}

#: Why a unit gives itself a task (``Behavior.dll:0x100179c0``, built by
#: ``0x10034510``): its radar's best contact, the object whose explosion hit it,
#: an attack by logic id nothing asks for, a refit, nothing (no task), and a
#: clan-mate's call for help, which attacks a place.
REASON_ENGAGE, REASON_RETALIATE, REASON_ATTACK, REASON_REFIT, REASON_NONE, REASON_HELP = range(6)
#: A unit hit by an explosion calls every warrior of its clan within this for
#: help (``0x1000c260``; a constant of its own, not ``Patrol_Attack_Range``).
HELP_RANGE = 400.0

#: The behaviour constants a walk's speed is taken from and held by
#: (``Behavior.dll:0x10016250``; no file names them).
GO_SPEED_PERCENT = 1.0
BUILD_SPEED_PERCENT = 1.0
TRANSPORT_SPEED_PERCENT = 1.0
MOVEMENT_SPEED_PERCENT = 1.0
MOVEMENT_MIN_SPEED_PERCENT = 1.0
MOVEMENT_MAX_SPEED = 600.0
#: The slowest walk the walker allows (``Behavior.dll:0x10059148``).
MIN_WALK_SPEED = 2.0
#: Bound and never read: the patrol's attack range and the builder's distance.
PATROL_ATTACK_RANGE = 400.0
BUILD_BUILD_DISTANCE = 150.0
#: A guard's speed figure by what it guards.  The building's 80 is held to the
#: unit's full speed like any figure above 1.
PATROL_SPEED_PERCENT = {"unit": 1.0, "building": 80.0, "place": 0.8}


def walk_speed(requested: float, top: float, low: float = 0.0, *,
               maximum_factor: float = 1.0) -> float:
    """The speed ``MWalker::SetTarget`` walks at (``Behavior.dll:0x1003be4f``).

    ``requested`` is the unit's speed times the task's figure; ``top`` and
    ``low`` are the two speeds of the unit's control record, and
    ``maximum_factor`` is the difficulty profile's ``Speed_MaximumFactor``.
    Since the task's speed and ``top`` are the same number, a figure above 1
    walks exactly as 1 does.
    """
    speed = min(requested, top * MOVEMENT_SPEED_PERCENT * maximum_factor)
    speed = min(speed, MOVEMENT_MAX_SPEED)
    speed = max(speed, low * MOVEMENT_MIN_SPEED_PERCENT)
    return max(speed, MIN_WALK_SPEED)


#: A clan's areal map refreshes every areal on a timer of this many ms, plus up
#: to the second figure (``ArealMap.dll:0x10001010``, ``0x1002d570``), and on
#: its first tick.
CLAN_MAP_REFRESH_MS = (46 * 64, 46 * 64 * 255 / 256)

#: The auto-driver levels ``CMD_JAMES_AUTO_DRIVER`` steps through
#: (``iron3d.dll:0x10075fc0``).
AUTO_DRIVER_LEVELS = 3


def unit_takt_runs(driven: bool, auto_driver: int = 0, hero: bool = False) -> bool:
    """Whether a player's unit has its behaviour flag ``0x10``, which the unit's
    own takt -- self-refit, the repair decision, the escape from a building --
    needs.  Read off the wizard (``Wizard.dll:0x10003890``) as
    ``iron3d.dll:0x10074ff0`` sets it: a bot let go runs it, a driven bot only
    from auto-driver level 1, and the player's hero never.
    """
    if hero:
        return False
    return not driven or auto_driver % AUTO_DRIVER_LEVELS >= 1


# --------------------------------------------------------------------------
# Mineral lodes: the mission trailer's records
# --------------------------------------------------------------------------

#: How near a unit searching for minerals must come to a lode to find it, and
#: how far from a mine the lodes it draws on may lie (``Behavior.dll:0x1002cd10``).
LODE_REACH = 10.0
MINE_LODE_RADIUS = 250.0


@dataclass(frozen=True)
class MineralLode:
    """One of a mission's mineral lodes, as the game keeps it.

    The records are the trailer's four-word entries ``mission.Viewpoint`` carries
    (``MisLoad.dll:0x10001b10``, handed on by ``iron3d.dll:0x10081880``): a
    position, a found flag, a type word, an amount and a last float.  The game
    keeps x, y, the flag and the amount; z and the last two are dropped, and the
    type is set to ``MINERALS`` whatever the file says.
    """

    x: float
    y: float
    z: float
    found: bool
    type_word: int
    amount: float
    last: float


def mineral_lodes(m: mission.Mission) -> list[MineralLode]:
    """A mission's mineral lodes, from its trailer."""
    out = []
    for record in m.viewpoints:
        found, type_word, amount, last = record.unknown
        out.append(MineralLode(
            *record.position, found=bool(found), type_word=type_word,
            amount=struct.unpack("<f", struct.pack("<I", amount & 0xFFFFFFFF))[0],
            last=struct.unpack("<f", struct.pack("<I", last & 0xFFFFFFFF))[0]))
    return out


@dataclass(frozen=True)
class Package:
    #: The menu command id.
    command: int
    #: Its name's string id in ``iron3d.dll``.
    string: int
    label: str
    #: The robot Types it is offered to: a Type ``t`` gets it when ``mask & t == t``.
    mask: int
    #: The order it gives, or None for a building placement.
    order: int | None
    target: int
    #: The menus it is on: the HQ menu, the second (wingman) menu, or both.
    menus: tuple[str, ...]


HQ, WINGMAN = "hq", "wingman"

#: The commander's packages (``iron3d.dll:0x10104f98``, ``0x10105150``), with
#: the order each gives (``0x10079230``).  The seven Build entries (commands
#: 10-16) place a building and the seven Upgrade entries (17-23) give
#: ``UPGRADE``; they are builders' only.
PACKAGES = (
    Package(0, 5000, "Standby", ROBOT_ANY, STAYGROUND, TARGET_NOT_DEFINED, (HQ, WINGMAN)),
    Package(1, 5001, "Route", ROBOT_ANY, GO, TARGET_BY_PLACE, (HQ,)),
    Package(2, 5002, "Search and capture", ROBOT_ANY, SEARCH, TARGET_BY_TYPE, (HQ, WINGMAN)),
    Package(3, 5003, "Seek and destroy", ROBOT_ANY, SEARCH, TARGET_NOT_DEFINED, (HQ, WINGMAN)),
    Package(4, 5004, "Attack", ROBOT_ANY, ATTACK, TARGET_BY_LOGIC_ID, (WINGMAN,)),
    Package(5, 5005, "Capture building", ROBOT_ANY, SEARCH, TARGET_BY_LOGIC_ID, (WINGMAN,)),
    Package(6, 5006, "Guard", ROBOT_ANY, PATROL, TARGET_BY_LOGIC_ID, (HQ,)),
    Package(7, 5007, "Refit", ROBOT_ANY, RELOAD, TARGET_NOT_DEFINED, (HQ, WINGMAN)),
    Package(8, 5020, "Transport minerals", objects.TYPE_TRANSPORT, TRANSPORT,
            TARGET_NOT_DEFINED, (HQ,)),
    Package(9, 5030, "Search minerals", objects.TYPE_BUILDER, SEARCH, TARGET_BY_TYPE, (HQ,)),
    Package(10, 5031, "Build", objects.TYPE_BUILDER, None, TARGET_BY_PLACE, (HQ,)),
    Package(17, 1008, "Upgrade", objects.TYPE_BUILDER, UPGRADE, TARGET_BY_LOGIC_ID, (HQ,)),
    Package(24, 5008, "Follow me", ROBOT_ANY, FOLLOW, TARGET_BY_LOGIC_ID, (WINGMAN,)),
)
#: The packages that capture, which a unit bigger than ``CAPTURE_SIZE`` fails.
CAPTURING = (2, 5)


def packages_for(unit_type: int, size_class: int) -> list[Package]:
    """What the commander's menus offer a unit of this Type and size class."""
    out = [p for p in PACKAGES if unit_type and p.mask & unit_type == unit_type]
    if size_class > CAPTURE_SIZE:
        out = [p for p in out if p.command not in CAPTURING]
    return out
