"""The golden cross-check: the Python readers against the Rust engine's.

``openparkan golden --engine path/to/parkan-dump`` dumps every file the
engine's current milestone reads, once through ``openparkan.dump`` and once
through the engine, and compares the two trees.  Numbers agree within
``TOLERANCE``; everything else must be equal.  It needs the install, so it is
not part of ``pytest``, and nothing it produces is committed.
"""

from __future__ import annotations

import json
import subprocess
from collections.abc import Iterator
from pathlib import Path

from . import dump
from .nres import is_nres

#: How far two floats may differ, absolutely or relative to their size.
TOLERANCE = 1e-5

#: The first mission, whose map Phase One plays.
MISSION_01 = Path("MISSIONS/CAMPAIGN/CAMPAIGN.00/Mission.01")


def differences(a, b, path: str = "$") -> Iterator[str]:
    """Every place two dumps disagree, as a JSON path and the two values."""
    if isinstance(a, dict) and isinstance(b, dict):
        for key in sorted(set(a) | set(b)):
            if key not in a or key not in b:
                yield f"{path}.{key}: only in {'python' if key in a else 'engine'}"
            else:
                yield from differences(a[key], b[key], f"{path}.{key}")
    elif isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            yield f"{path}: {len(a)} items against {len(b)}"
        for i, (x, y) in enumerate(zip(a, b, strict=False)):
            yield from differences(x, y, f"{path}[{i}]")
    elif isinstance(a, bool) or isinstance(b, bool):
        if type(a) is not type(b) or a != b:
            yield f"{path}: {a!r} against {b!r}"
    elif isinstance(a, (int, float)) and isinstance(b, (int, float)):
        if abs(a - b) > TOLERANCE * max(1.0, abs(a), abs(b)):
            yield f"{path}: {a!r} against {b!r}"
    elif a != b:
        yield f"{path}: {a!r} against {b!r}"


def targets(game: Path) -> list[tuple[str, Path]]:
    """What M0 reads: every archive in the install, and Mission 01's data.tma."""
    archives = sorted(p for p in game.rglob("*") if p.is_file() and is_nres(p))
    return [("nres", p) for p in archives] + [("mission", game / MISSION_01 / "data.tma")]


def engine_dump(engine: Path, kind: str, path: Path) -> dict:
    done = subprocess.run([str(engine), kind, str(path)], capture_output=True, check=True)
    return json.loads(done.stdout)


def run(game: Path, engine: Path, limit: int = 20) -> int:
    """Compare every target; print what differs.  Returns a process exit code."""
    failed = 0
    items = targets(game)
    for kind, path in items:
        ours = json.loads(json.dumps(dump.KINDS[kind](path)))
        theirs = engine_dump(engine, kind, path)
        found = list(differences(ours, theirs))
        name = path.relative_to(game)
        if found:
            failed += 1
            print(f"DIFF  {kind} {name}: {len(found)} differences")
            for line in found[:limit]:
                print(f"      {line}")
        else:
            print(f"same  {kind} {name}")
    print(f"\n{len(items) - failed}/{len(items)} dumps agree")
    return 1 if failed else 0


def find_engine(explicit: str | None) -> Path | None:
    """The given binary, or the workspace's release or debug build."""
    if explicit:
        return Path(explicit)
    root = Path(__file__).resolve().parent.parent / "engine" / "target"
    for build in ("release", "debug"):
        candidate = root / build / "parkan-dump"
        if candidate.exists():
            return candidate
    return None
