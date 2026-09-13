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

from . import assembly, control, controls, effects, landmesh, materials, mission, objects, sky
from . import mesh as objmesh
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


def _pose(pose) -> list:
    return [vector(pose[0]), vector(pose[1])]


def sample_frames(frame_count: int) -> list[float]:
    """The frames a mesh dump poses every animated node at: whole, halves, past the end."""
    return [0.0, 0.5, 1.0, 2.25, frame_count * 0.5 + 0.3, frame_count - 1.0, frame_count + 1.0]


def object_mesh(path: Path, names: list[str] | None = None) -> dict:
    """One ``MESH`` member of an archive, ``names[0]``, as the reader holds it.

    Positions are the posed ones a renderer draws, and every node carries its
    world pose, so the dump checks the pose chain as well as the streams.
    """
    archive = NResArchive.open(path)
    member = (names or [""])[0]
    try:
        wear = objmesh.parse_wear(archive.read_name(member.rsplit(".", 1)[0] + ".wea"))
    except KeyError:
        wear = objmesh.Wear()
    m = objmesh.parse(archive.read_name(member), member, wear.materials)
    return {
        "kind": "mesh",
        "name": member,
        "wear": {"materials": wear.materials, "lightmaps": wear.lightmaps},
        "nodes": [
            {"name": n.name, "flags": n.flags, "parent": n.parent, "anim_start": n.anim_start,
             "fallback_key": n.fallback_key, "slot_index": list(n.slot_index),
             "world_pose": _pose(m.world_pose(i))}
            for i, n in enumerate(m.nodes)
        ],
        "slots": [[s.first_triangle, s.triangle_count, s.first_batch, s.batch_count,
                   vector(s.aabb_min), vector(s.aabb_max), vector(s.sphere),
                   number(s.area), number(s.volume)] for s in m.slots],
        "batches": [[b.material, b.flag, b.first_index, b.index_count, b.first_vertex,
                     b.vertex_count] for b in m.batches],
        "triangles": [list(t) for t in m.triangles],
        "positions": [vector(v) for v in m.posed_positions()],
        "normals": [vector(v) for v in m.normals],
        "uv": [vector(v) for v in m.uv],
        "lightmap_uv": [vector(v) for v in m.lightmap_uv],
        "keys": [[vector(k.translation), number(k.time), vector(k.rotation)] for k in m.keys],
        "face_flags": list(m.face_flags),
        "face_normals": [vector(n) for n in m.face_normal],
        "frame_map": list(m.frame_map),
        "frame_count": m.frame_count,
        "root_pose": _pose(m.root_pose()),
        "node_of_vertex": m.node_of_vertex(),
        "sphere": None if m.volume is None else [vector(m.volume.centre), number(m.volume.radius)],
        "samples": [
            {"node": i, "at": [_pose(m.pose_at(i, f)) for f in sample_frames(m.frame_count)],
             "blended": _pose(m.blended_pose(i, 0.5, m.frame_count * 0.6, 0.3))}
            for i, n in enumerate(m.nodes) if n.is_animated
        ],
    }


def mission_assembly(path: Path, names: list[str] | None = None) -> dict:
    """Every object of a mission as the parts it is drawn from, and where they sit."""
    if path.is_dir():
        path = path / "data.tma"
    m = mission.load(path)
    game = next(d for d in path.parents if (d / "objects.rlb").exists())
    built = assembly.Assembly(game)
    return {
        "kind": "assembly",
        "objects": [
            {"kind": o.kind, "path": o.path,
             "parts": [{"library": p.ref.library, "member": p.ref.member, "pose": _pose(p.pose),
                        "host": p.host, "node": p.node}
                       for p in built.parts(o.kind, o.path)]}
            for o in m.objects
        ],
    }


def _box(box) -> list:
    return [vector(box[0]), vector(box[1])]


def controllers(path: Path, names: list[str] | None = None) -> dict:
    """The ``.ctl`` members of an archive, all of them unless ``names`` picks some."""
    archive = NResArchive.open(path)
    members = names or [e.name for e in archive if e.tag == control.CTL_TAG]
    out = []
    for member in members:
        c = control.parse(archive.read_name(member))
        out.append({
            "name": member,
            "counts": list(c.counts),
            "triples": [vector(t) for t in c.triples],
            "scale": c.scale, "pair": vector(c.pair), "mode": c.mode,
            "bounds": vector(c.bounds), "cone": number(c.cone), "flags": c.flags,
            "payload": number(c.payload), "bare": c.bare,
            "states": [
                {"flags": s.flags, "mode": s.mode, "pair_a": vector(s.pair_a),
                 "pair_b": vector(s.pair_b), "blend": number(s.blend),
                 "length": number(s.length), "velocity": _box(s.velocity),
                 "spin": _box(s.spin), "engine": number(s.engine), "actions": s.actions,
                 "request": s.request,
                 "contacts": [[k.point, k.flags, k.group] for k in s.contacts]}
                for s in c.states
            ],
            "costs": vector(c.costs),
            "live_costs": [number(c.live_cost(to, frm)) for to in range(len(c.states))
                           for frm in range(len(c.states))],
            "channels": [
                [ch.node, number(ch.first), number(ch.last), number(ch.initial), ch.origin,
                 ch.point, number(ch.rate), number(ch.span), ch.flags]
                for ch in c.channels
            ],
            "components": [
                {"type_id": k.type_id, "library": k.resource.library,
                 "member": k.resource.member, "index": k.index, "entries": list(k.entries),
                 "label": k.label, "values": vector(k.values), "power": number(k.power),
                 "node": k.node, "mass": number(k.mass), "flags": k.flags, "group": k.group}
                for k in c.components
            ],
            "groups": list(c.groups),
            "references": [
                {"library": r.resource.library, "member": r.resource.member,
                 "values": list(r.values), "group": r.group}
                for r in c.references
            ],
        })
    return {"kind": "control", "controllers": out}


def control_points(path: Path, names: list[str] | None = None) -> dict:
    """The ``.cpt`` members of an archive, all of them unless ``names`` picks some."""
    archive = NResArchive.open(path)
    members = names or [e.name for e in archive if e.tag == "CTPT"]
    return {
        "kind": "cpt",
        "members": [
            {"name": member,
             "points": [{"name": p.name, "a": vector(p.a), "nodes": list(p.nodes),
                         "position": vector(p.position), "direction": vector(p.direction)}
                        for p in objmesh.parse_control_points(archive.read_name(member), member)]}
            for member in members
        ],
    }


def _ref(ref) -> list[str]:
    return [ref.library, ref.member]


def damage_tables(path: Path, names: list[str] | None = None) -> dict:
    """The ``.ndp`` members of an archive, all of them unless ``names`` picks some."""
    archive = NResArchive.open(path)
    members = names or [e.name for e in archive if e.tag == "NDPR"]
    return {
        "kind": "ndp",
        "members": [
            {"name": member,
             "nodes": [[d.flags, number(d.durability), number(d.unknown), _ref(d.explosion)]
                       for d in objects.parse_damage(archive.read_name(member), member)]}
            for member in members
        ],
    }


def explosions(path: Path, names: list[str] | None = None) -> dict:
    """The ``.exp`` members of an archive, all of them unless ``names`` picks some."""
    archive = NResArchive.open(path)
    members = names or [e.name for e in archive if e.tag == "EXPL"]
    out = []
    for member in members:
        e = effects.parse_explosion(archive.read_name(member), member)
        out.append({"name": member, "kind": e.kind, "damage": number(e.damage),
                    "radius": number(e.radius), "values": vector(e.values),
                    "placement": e.placement, "slots": [_ref(s) for s in e.slots]})
    return {"kind": "exp", "members": out}


def fx_effects(path: Path, names: list[str] | None = None) -> dict:
    """The ``FXID`` members of an archive: header fields, and each emitter's live floats."""
    import struct

    archive = NResArchive.open(path)
    members = names or [e.name for e in archive if e.tag == "FXID"]
    out = []
    for member in members:
        e = effects.parse_effect(archive.read_name(member), member)
        h = e.header
        out.append({
            "name": member,
            "header": {"count": struct.unpack_from("<i", h, 0)[0], "mode": e.mode,
                       "duration": number(e.duration),
                       "jitter": number(struct.unpack_from("<f", h, 12)[0]),
                       "flags": e.flags, "gate": e.gate,
                       "offset": vector(struct.unpack_from("<3f", h, 24)),
                       "point": vector(struct.unpack_from("<3f", h, 36)),
                       "scale": vector(e.scale)},
            "emitters": [{"kind": m.kind, "word": m.word, "resource": _ref(m.resource),
                          "window": None if m.window is None else vector(m.window),
                          "live": [[at, number(v)] for at, v in m.live_floats().items()]}
                         for m in e.emitters],
        })
    return {"kind": "fxid", "members": out}


def atmosphere(path: Path, names: list[str] | None = None) -> dict:
    """A ``sky.ske``: its sections and their headers, every keyframe as stored, and
    the time the clock starts at."""
    a = sky.load(path)
    return {
        "kind": "sky",
        "sections": a.sections,
        "day_seconds": a.day_seconds,
        "section_headers": [
            {"index": s.index, "version": s.version, "count": s.count,
             "end": list(s.end.words), "day": list(s.day.words)}
            for s in a.section_headers
        ],
        "keyframes": [
            {"version": k.version, "time": list(k.time.words), "opcode": k.opcode,
             "hour": k.hour, "minute": k.minute, "section": k.section, "name": k.name,
             "names": list(k.names), "effects": list(k.effects),
             "slots": [list(s) for s in k.slots], "intensity": vector(k.intensity)}
            for k in a.keyframes
        ],
        "start": list(a.start.words),
        "trailer_word": a.trailer_word,
        "sky_flag": a.sky_flag,
    }


def input_table(path: Path, names: list[str] | None = None) -> dict:
    """A ``.tbl``: every row, and the numbers the engine resolves its names to."""
    return {
        "kind": "controls",
        "rows": [
            {"device": a.device, "modifier": a.modifier, "key": a.key, "pressed": a.pressed,
             "target": a.target, "command": a.command, "value": number(a.value),
             "index": a.index, "state": a.state, "ramp": number(a.ramp),
             "ramp_time": a.ramp_time, "note": a.note, "code": a.code,
             "class_id": a.class_id, "bits": a.bits}
            for a in controls.table(path)
        ],
    }


def _nres(path: Path, names: list[str] | None = None) -> dict:
    return nres(path)


def _mission(path: Path, names: list[str] | None = None) -> dict:
    return mission_file(path)


#: What ``openparkan dump`` and ``parkan-dump`` both accept, and the reader each runs.
KINDS = {"nres": _nres, "mission": _mission, "texm": texm, "materials": material_library,
         "landmesh": land_mesh, "mesh": object_mesh, "assembly": mission_assembly,
         "control": controllers, "controls": input_table, "cpt": control_points,
         "ndp": damage_tables, "exp": explosions, "fxid": fx_effects,
         "sky": atmosphere}
