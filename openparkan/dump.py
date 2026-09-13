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
import struct
from pathlib import Path

from . import landmesh, materials, mission
from . import texm as textures
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


def texm(path: Path, names: list[str] | None = None) -> dict:
    """Textures of an archive -- the ``names`` given, or all -- decoded at level 0."""
    archive = NResArchive.open(path)
    wanted = {n.upper().split(".")[0] for n in names} if names else None
    out = []
    for e in archive:
        if e.tag != "Texm" or (wanted is not None and e.name.upper().split(".")[0] not in wanted):
            continue
        blob = archive.read(e)
        tex = textures.decode(blob)
        flags14 = struct.unpack_from("<I", blob, 0x14)[0]
        out.append({
            "name": e.name,
            "width": tex.width,
            "height": tex.height,
            "format": tex.fmt,
            "mips": tex.mips,
            "flags": tex.flags,
            "flags14": flags14,
            "alpha": textures.uploads_with_alpha(tex.fmt, flags14),
            "pages": [[x, y, w, h] for x, y, w, h in tex.pages],
            "rgba_sha256": hashlib.sha256(tex.rgba).hexdigest(),
        })
    return {"kind": "texm", "textures": out}


def material_library(path: Path, names: list[str] | None = None) -> dict:
    """Every ``MAT0`` record, entries and tracks included."""
    lib = materials.MaterialLibrary(path)
    return {
        "kind": "materials",
        "materials": [
            {
                "name": m.name,
                "blend": m.blend,
                "blend_mode": m.blend_mode,
                "surface": m.surface,
                "speed_factor": number(m.speed_factor),
                "damage_rate": number(m.damage_rate),
                "frames": m.frames,
                "entries": [
                    {"texture": e.texture, "cell": e.cell, "ambient": list(e.ambient),
                     "diffuse": list(e.colour), "specular": list(e.specular),
                     "emissive": list(e.emissive),
                     "alphas": [e.ambient_alpha, e.diffuse_alpha, e.specular_alpha,
                                e.emissive_alpha],
                     "power": e.power}
                    for e in m.entries
                ],
                "tracks": [{"kind": t.kind, "param": t.param,
                            "keys": [[k.entry, k.time, k.unread] for k in t.keys]}
                           for t in m.tracks],
            }
            for m in lib.materials.values()
        ],
    }


#: How many height samples a side the land mesh dump takes, each at the centre of
#: its square so none falls on a face edge the two readers index differently.
HEIGHT_SAMPLES = 17


def land_mesh(path: Path, names: list[str] | None = None) -> dict:
    """A ``Land.msh``: every stream as parsed, and the elevation on a grid."""
    land = landmesh.load(path)
    (lo, hi) = land.bounds()
    grid = []
    for j in range(HEIGHT_SAMPLES):
        for i in range(HEIGHT_SAMPLES):
            x = lo[0] + (hi[0] - lo[0]) * (i + 0.5) / HEIGHT_SAMPLES
            y = lo[1] + (hi[1] - lo[1]) * (j + 0.5) / HEIGHT_SAMPLES
            z = land.height_at(x, y)
            grid.append(None if z is None else number(z))
    water = land.water_level()
    return {
        "kind": "landmesh",
        "bounds": [vector(lo), vector(hi)],
        "lod_split": land.lod_split,
        "water_level": None if water is None else number(water),
        "layer1": land.layer1_names,
        "layer2": land.layer2_names,
        "positions": [vector(p) for p in land.positions],
        "normals": [vector(n) for n in land.normals],
        "uv1": [vector(u) for u in land.uv1],
        "uv2": [vector(u) for u in land.uv2],
        "blend": vector(land.blend),
        "faces": [
            [list(land.faces[i]), list(land.adjacency[i]), land.face_flags[i],
             land.face_surface[i], land.face_tex1[i], land.face_tex2[i],
             vector(land.face_normal[i]), land.face_patch[i]]
            for i in range(land.face_count)
        ],
        "cells": [[c.first, c.count] for c in land.cells],
        "height_grid": grid,
    }


def _nres(path: Path, names: list[str] | None = None) -> dict:
    return nres(path)


def _mission(path: Path, names: list[str] | None = None) -> dict:
    return mission_file(path)


#: What ``openparkan dump`` and ``parkan-dump`` both accept, and the reader each runs.
KINDS = {"nres": _nres, "mission": _mission, "texm": texm, "materials": material_library,
         "landmesh": land_mesh}
