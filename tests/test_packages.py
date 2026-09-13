"""What the commander's menus offer a unit."""

from __future__ import annotations

from openparkan import objects, packages


def labels(unit_type: int, size: int) -> set[str]:
    return {p.label for p in packages.packages_for(unit_type, size)}


def test_a_small_builder_gets_the_common_packages_and_building():
    got = labels(objects.TYPE_BUILDER, 2)
    assert {"Standby", "Route", "Search and capture", "Refit", "Build", "Upgrade",
            "Search minerals"} <= got
    assert "Transport minerals" not in got


def test_a_large_warrior_cannot_capture():
    got = labels(objects.TYPE_WARRIOR, 4)
    assert "Seek and destroy" in got
    assert not got & {"Search and capture", "Capture building"}


def test_a_building_is_offered_nothing():
    assert labels(0x80000010, 3) == set()


def test_every_package_order_is_declared():
    declared = set(packages.ORDERS.values())
    assert all(p.order is None or p.order in declared for p in packages.PACKAGES)
