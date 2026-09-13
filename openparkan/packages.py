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

from dataclasses import dataclass

from . import objects

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

#: The building types "Search and capture" looks for: all but the large tower.
CAPTURE_TYPES = 0x8017365E
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
