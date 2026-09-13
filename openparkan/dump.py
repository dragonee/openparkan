"""Canonical JSON of what the readers make of a file, for the golden cross-check.

The Rust engine's ``parkan-dump`` writes the same shape from its own readers,
and ``openparkan golden`` compares the two.  So this module's output is a
contract rather than a convenience: every key here has a twin in
``engine/crates/parkan-world/src/dump.rs``, and a field the two do not agree
on is a bug in one of the ports.

Raw payloads are not written out.  A member appears as its directory fields
and a SHA-256 of its bytes, which is enough to show both readers cut the
archive in the same places.  Floats that are not finite become the strings
``"NaN"``, ``"Infinity"`` and ``"-Infinity"``, which JSON has no number for.
"""

from __future__ import annotations

import hashlib
import math
from pathlib import Path

from . import mission
from .nres import NResArchive


def number(value: float) -> float | str:
    """A float JSON can hold, or the name of one it cannot."""
    if math.isnan(value):
        return "NaN"
    if math.isinf(value):
        return "Infinity" if value > 0 else "-Infinity"
    return value


def vector(values) -> list[float | str]:
    return [number(v) for v in values]


def nres(path: Path) -> dict:
    """An archive's version and directory, one SHA-256 per member."""
    archive = NResArchive.open(path)
    return {
        "kind": "nres",
        "version": archive.version,
        "entries": [
            {
                "name": e.name,
                "tag": e.tag,
                "type_id": e.type_id,
                "offset": e.offset,
                "size": e.size,
                "index": e.index,
                "element_count": e.element_count,
                "link_count": e.link_count,
                "sha256": hashlib.sha256(archive.read(e)).hexdigest(),
            }
            for e in archive
        ],
    }


def _property(p: mission.Property) -> dict:
    convert = number if p.is_float else int
    return {"name": p.name, "kind": p.type, "value": convert(p.value),
            "minimum": convert(p.b), "maximum": convert(p.c)}


def mission_file(path: Path) -> dict:
    """A ``data.tma`` as the reader holds it, unknown fields included."""
    m = mission.load(path)
    return {
        "kind": "mission",
        "version": m.version,
        "routes": [{"id": r.id, "points": [vector(p) for p in r.points]} for r in m.routes],
        "clans": [
            {
                "name": c.name,
                "parent": c.unknown[0],
                "base": vector(c.base),
                "kind": c.type,
                "ai_script": c.ai_script,
                "zones": [{"kind": z.kind, "position": vector(z.position),
                           "inner": number(z.inner), "outer": number(z.outer)}
                          for z in c.zones],
                "behaviour": c.behaviour,
                "minds": c.minds,
                "relations": [[name, word] for name, word in c.relations.items()],
            }
            for c in m.clans
        ],
        "unknown_pre_objects": m.unknown_pre_objects,
        "objects": [
            {
                "kind": o.kind,
                "path": o.path,
                "unknown_q": o.unknown[0],
                "logical_id": o.logical_id,
                "position": vector(o.position),
                "pad": list(o.unknown[1]),
                "rotation": number(o.rotation),
                "scale": vector(o.scale),
                "name": o.name,
                "tail": list(o.unknown[2]),
                "properties": [_property(p) for p in o.properties.values()],
            }
            for o in m.objects
        ],
        "map_path": m.map_path,
        "description": m.description,
        "viewpoints": [{"position": vector(v.position), "unknown": list(v.unknown)}
                       for v in m.viewpoints],
    }


#: What ``openparkan dump`` and ``parkan-dump`` both accept, and the reader each runs.
KINDS = {"nres": nres, "mission": mission_file}
