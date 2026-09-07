"""Builds a self-contained HTML terrain viewer from parsed map data.

The output embeds geometry and textures directly, so the file works from a
local filesystem, a static host, or a Claude Artifact with no server and no
asset directory beside it.
"""

from __future__ import annotations

import base64
import json
import struct
import zlib
from importlib import resources
from pathlib import Path

from . import landmesh, mission, objects, texm
from . import mesh as objmesh
from .nres import NResArchive
from .png import _chunk

_SPECIAL_MATERIALS = {
    # Not present in Textures.lib; the engine treats these as animated or
    # procedural surfaces.  Approximated here with flat colours.
    "WATER": (0x2E, 0x6B, 0x8F, 0.72),
    "WATER_M": (0x2E, 0x6B, 0x8F, 0.72),
    "WATER_BOT": (0x4A, 0x54, 0x40, 1.0),
    "B_S0": (0x6B, 0x6B, 0x63, 1.0),
    "B_MTP_01": (0x6B, 0x6B, 0x63, 1.0),
    "ENV_NLAVA": (0xC4, 0x4A, 0x1E, 1.0),
    "ENV_NLAVA_M": (0xC4, 0x4A, 0x1E, 1.0),
    "ENV_LAVA_BOT": (0x5A, 0x2A, 0x18, 1.0),
}


def _png_data_uri(width: int, height: int, rgb: bytes) -> str:
    raw = bytearray()
    row = width * 3
    for y in range(height):
        raw.append(0)
        raw += rgb[y * row : (y + 1) * row]
    blob = (
        b"\x89PNG\r\n\x1a\n"
        + _chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
        + _chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + _chunk(b"IEND", b"")
    )
    return "data:image/png;base64," + base64.b64encode(blob).decode("ascii")


def _b64(arr: bytes) -> str:
    return base64.b64encode(arr).decode("ascii")


class TextureResolver:
    """Resolves terrain texture names into a pool of materials shared by every map.

    Maps reuse the same handful of ground textures, so the pool is what keeps a
    33-map viewer from embedding the same PNG thirty-three times.
    """

    def __init__(self, game: Path, max_size: int = 128):
        self.archive = NResArchive.open(game / "Textures.lib")
        self.max_size = max_size
        self.index = {}
        for e in self.archive:
            # entries are named like "L04.0" -- strip the trailing sub-index
            self.index.setdefault(e.name.split(".")[0].upper(), e)
        self.pool: list[dict] = []
        self._by_name: dict[str, int] = {}

    def _downsample(self, tex, rgb: bytes) -> tuple[int, int, bytes]:
        """Halve until within ``max_size``; terrain tiles do not need mip 0."""
        w, h = tex.width, tex.height
        while max(w, h) > self.max_size and w % 2 == 0 and h % 2 == 0:
            nw, nh = w // 2, h // 2
            out = bytearray(nw * nh * 3)
            for y in range(nh):
                for x in range(nw):
                    o = (y * nw + x) * 3
                    for c in range(3):
                        a = rgb[((2 * y) * w + 2 * x) * 3 + c]
                        b = rgb[((2 * y) * w + 2 * x + 1) * 3 + c]
                        d = rgb[((2 * y + 1) * w + 2 * x) * 3 + c]
                        e = rgb[((2 * y + 1) * w + 2 * x + 1) * 3 + c]
                        out[o + c] = (a + b + d + e) // 4
            rgb, w, h = bytes(out), nw, nh
        return w, h, rgb

    def resolve(self, name: str) -> int:
        """Return the index in ``self.pool`` for this terrain texture name."""
        key = name.upper()
        if key in self._by_name:
            return self._by_name[key]
        entry = self.index.get(key)
        if entry is not None:
            tex = texm.decode(self.archive.read(entry))
            w, h, rgb = self._downsample(tex, texm.to_rgb(tex, (90, 90, 90)))
            mat = {"kind": "texture", "name": name, "url": _png_data_uri(w, h, rgb), "opacity": 1.0}
        else:
            r, g, b, a = _SPECIAL_MATERIALS.get(key, (0x80, 0x80, 0x80, 1.0))
            mat = {"kind": "colour", "name": name, "colour": (r << 16) | (g << 8) | b, "opacity": a}
        self._by_name[key] = len(self.pool)
        self.pool.append(mat)
        return self._by_name[key]


def build_map_payload(mesh: landmesh.LandMesh, resolver: TextureResolver, name: str) -> dict:
    """Pack one map into the JSON blob the viewer's JavaScript consumes."""
    nv = mesh.vertex_count
    (minx, miny, minz), (maxx, maxy, maxz) = mesh.bounds()
    cx, cy = (minx + maxx) / 2, (miny + maxy) / 2

    # Game space is Z-up with +Y north; WebGL wants Y-up.  Recentre on the way
    # so the camera maths is about a map sitting at the origin.
    # Attributes keep their source precision rather than being widened to
    # float32: normals are int8 in the file and UVs are 8.8 fixed point, so
    # storing them narrow is both smaller and closer to what the game shipped.
    # The viewer restores the UV scale with texture.repeat = 1/256.
    pos = bytearray()
    nrm = bytearray()
    uv = bytearray()
    for i in range(nv):
        x, y, z = mesh.positions[i]
        pos += struct.pack("<3f", x - cx, z, -(y - cy))
        a, b, c = mesh.normals[i]
        nrm += struct.pack(
            "<3h",
            max(-32767, min(32767, round(a * 32767))),
            max(-32767, min(32767, round(c * 32767))),
            max(-32767, min(32767, round(-b * 32767))),
        )
        u, v = mesh.uv1[i]
        uv += struct.pack(
            "<2H",
            min(0xFFFF, round(u * landmesh.UV_FIXED_POINT_SCALE)),
            min(0xFFFF, round(v * landmesh.UV_FIXED_POINT_SCALE)),
        )

    # Sort faces by material so each run becomes one draw group.
    buckets: dict[tuple[int, int], list[int]] = {}
    for fi, _tri in enumerate(mesh.faces):
        key = (mesh.face_tex1[fi], mesh.is_water(fi))
        buckets.setdefault(key, []).append(fi)

    wide = nv > 0xFFFF
    idx = bytearray()
    groups = []
    materials = []
    for (tex_index, is_water), face_ids in sorted(buckets.items()):
        start = len(idx) // (4 if wide else 2)
        for fi in face_ids:
            a, b, c = mesh.faces[fi]
            idx += struct.pack("<3I" if wide else "<3H", a, b, c)
        tex_name = mesh.texture_name(1, tex_index) or "?"
        groups.append({"start": start, "count": len(face_ids) * 3, "material": len(materials)})
        materials.append({"pool": resolver.resolve(tex_name), "water": is_water})

    wet = mesh.water_faces()
    level = mesh.water_level()
    wet_layers = sorted({mesh.face_tex1[fi] for fi in wet})

    return {
        "name": name,
        "water": {"z": round(level, 2), "faces": len(wet)} if level is not None else None,
        "wetLayers": wet_layers,
        "vertexCount": nv,
        "faceCount": mesh.face_count,
        "extent": [round(maxx - minx, 1), round(maxy - miny, 1)],
        # The recentring applied to the geometry, so mission markers given in
        # raw game coordinates can be placed into the same frame.
        "centre": [round(cx, 4), round(cy, 4)],
        "height": [round(minz, 1), round(maxz, 1)],
        "wideIndex": wide,
        "position": _b64(bytes(pos)),
        "normal": _b64(bytes(nrm)),
        "uv": _b64(bytes(uv)),
        "index": _b64(bytes(idx)),
        "groups": groups,
        "materials": materials,
        "layer1": mesh.layer1_names,
        "layer2": mesh.layer2_names,
    }


#: Clan colours, chosen to stay distinguishable on both the light and dark ground.
CLAN_COLOURS = [0x3E7CB1, 0xC1453C, 0x4E9A51, 0xB07A2A, 0x7B5EA7, 0x2E9A96]
NEUTRAL_COLOUR = 0x8A8A85


class SceneryLibrary:
    """Resolves scenery names to geometry, once each, for the viewer.

    Missions place scenery by name into ``objects.rlb``, whose STAT record
    points at a ``.msh`` in another archive.  Meshes are shared across
    missions, so they are packed once and referenced by index.
    """

    def __init__(self, game: Path):
        self.game = game
        self.library = objects.ObjectLibrary(game / "objects.rlb")
        self._archives: dict[str, object] = {}
        self.models: list[dict] = []
        self._index: dict[str, int | None] = {}

    def _archive(self, name: str):
        if name not in self._archives:
            self._archives[name] = NResArchive.open(self.game / name)
        return self._archives[name]

    def resolve(self, name: str) -> int | None:
        """Index into ``self.models``, or None when the name has no geometry."""
        key = name.lower()
        if key in self._index:
            return self._index[key]
        record = self.library.get(name)
        ref = record.mesh if record else None
        slot = None
        if ref:
            m = objmesh.parse(self._archive(ref.library).read_name(ref.member), ref.member)
            pos = bytearray()
            nrm = bytearray()
            for (x, y, z), (a, b, c) in zip(m.positions, m.normals, strict=True):
                # Game space is Z-up; the viewer is Y-up, matching the terrain.
                pos += struct.pack("<3f", x, z, -y)
                nrm += struct.pack(
                    "<3h",
                    max(-32767, min(32767, round(a * 32767))),
                    max(-32767, min(32767, round(c * 32767))),
                    max(-32767, min(32767, round(-b * 32767))),
                )
            wide = m.vertex_count > 0xFFFF
            idx = bytearray()
            for tri in m.triangles:
                idx += struct.pack("<3I" if wide else "<3H", *tri)
            slot = len(self.models)
            self.models.append({
                "name": name,
                "wide": wide,
                "position": _b64(bytes(pos)),
                "normal": _b64(bytes(nrm)),
                "index": _b64(bytes(idx)),
                "tris": m.triangle_count,
            })
        self._index[key] = slot
        return slot


def build_mission_payload(
    m: mission.Mission, map_index: int, scenery: SceneryLibrary | None = None
) -> dict:
    """Pack a mission into markers the viewer can drop onto its map.

    Positions are converted into the same recentred, Y-up frame as the terrain,
    which is the point of the exercise: if the two disagree, the markers float
    or sink and you can see it immediately.
    """
    clans = [
        {"name": c.name, "index": c.index, "base": [c.base[0], c.base[1]],
         "colour": CLAN_COLOURS[i % len(CLAN_COLOURS)]}
        for i, c in enumerate(m.clans)
    ]
    placed = []
    for o in m.objects:
        x, y, z = o.position
        # ClanID is a 0-based index into the clan list, not the clan's own
        # `index` field -- see docs/04-missions.md.
        slot = o.clan_id if o.clan_id is not None and 0 <= o.clan_id < len(clans) else None
        entry = {
            "p": [round(x, 2), round(z, 2), round(y, 2)],
            "k": o.kind,
            "c": -1 if slot is None else slot,
            "n": o.name or o.path.replace("\\", "/").rsplit("/", 1)[-1],
            "r": round(o.rotation, 4),
        }
        if scenery is not None and o.is_static:
            model = scenery.resolve(o.path)
            if model is not None:
                entry["m"] = model
        placed.append(entry)
    # Campaign missions are all called Mission.0N, so qualify them with the
    # campaign directory to keep the selector unambiguous.
    folder = m.source.parent
    label = folder.name
    if folder.parent.name.upper().startswith("CAMPAIGN."):
        label = f"{folder.parent.name} / {folder.name}"

    return {
        "name": folder.name,
        "label": label,
        "title": m.title,
        "map": map_index,
        "clans": clans,
        "objects": placed,
        "routes": [[[round(v, 2) for v in pt] for pt in r.points] for r in m.routes],
    }


def build_html(
    payloads: list[dict],
    pool: list[dict],
    missions: list[dict] | None = None,
    models: list[dict] | None = None,
    title: str = "Parkan Terrain Viewer",
) -> str:
    data = json.dumps(
        {"maps": payloads, "pool": pool, "missions": missions or [], "models": models or []},
        separators=(",", ":"),
    )
    return _template().replace("__TITLE__", title).replace('"__DATA__"', data)


def _template() -> str:
    return resources.files(__package__).joinpath("templates/viewer.html").read_text("utf-8")
