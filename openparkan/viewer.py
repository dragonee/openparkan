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

from . import landmesh, materials, mission, objects, texm
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
        """Index in ``self.pool`` for a texture name, with or without its
        ``.0`` member suffix."""
        key = name.upper().split(".")[0]
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
    # Terrain positions are quantised the same way object meshes are: int16
    # over the map's own bounding box, dequantised by a scale and offset that
    # the viewer applies to the mesh object rather than to the geometry.
    tspan = [
        max((maxx - minx) / 2, 1e-6),
        max((maxz - minz) / 2, 1e-6),
        max((maxy - miny) / 2, 1e-6),
    ]
    tmid = ((minx + maxx) / 2 - cx, (minz + maxz) / 2, -((miny + maxy) / 2 - cy))

    pos = bytearray()
    nrm = bytearray()
    uv = bytearray()
    for i in range(nv):
        x, y, z = mesh.positions[i]
        for value, mid, span in zip((x - cx, z, -(y - cy)), tmid, tspan, strict=True):
            q = round((value - mid) / span * 32767)
            pos += struct.pack("<h", max(-32767, min(32767, q)))
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
        "scale": [round(v, 4) for v in tspan],
        "offset": [round(v, 4) for v in tmid],
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


class ModelLibrary:
    """Resolves any placed mission object to renderable geometry.

    Two chains meet here.  Scenery names an ``objects.rlb`` record directly.
    A building or unit names a ``UNITS/**/*.dat`` assembly, whose first
    component names a record -- and for buildings that record is a ``FORT``
    whose first slot points at a second record that finally carries the mesh.

    Only the first component of an assembly is used, so a unit renders as its
    chassis rather than as the full assembled robot: where the other parts
    attach is not yet known.  See docs/07-objects.md.
    """

    MAX_INDIRECTION = 3

    def __init__(self, game: Path, textures: TextureResolver):
        self.game = game
        self.library = objects.ObjectLibrary(game / "objects.rlb")
        self.materials = materials.MaterialLibrary(game / "Material.lib")
        self.textures = textures
        self._archives: dict[str, NResArchive] = {}
        self.models: list[dict] = []
        self._by_ref: dict[tuple[str, str], int] = {}
        self._by_name: dict[str, int | None] = {}

    def _archive(self, name: str) -> NResArchive:
        if name not in self._archives:
            self._archives[name] = NResArchive.open(self.game / name)
        return self._archives[name]

    def _record_mesh(self, record, depth: int = 0):
        if record is None or depth > self.MAX_INDIRECTION:
            return None
        direct = record.mesh
        if direct:
            return direct
        # A FORT record has no geometry of its own; its first slot names
        # another record that does.
        for slot in record.slots:
            if slot and not slot.suffix:
                found = self._record_mesh(self.library.get(slot.member), depth + 1)
                if found:
                    return found
        return None

    def _unit_file(self, path: str) -> Path | None:
        f = self.game / path.replace("\\", "/")
        if f.exists():
            return f
        if f.parent.exists():
            lower = {x.name.lower(): x for x in f.parent.iterdir()}
            return lower.get(f.name.lower())
        return None

    def _pack(self, ref) -> int:
        key = (ref.library, ref.member)
        if key in self._by_ref:
            return self._by_ref[key]
        archive = self._archive(ref.library)
        try:
            wear = objmesh.read_wea(archive.read_name(ref.member.replace(".msh", ".wea")))
        except KeyError:
            wear = []
        m = objmesh.parse(archive.read_name(ref.member), ref.member, wear)

        # A model holds up to three levels of detail at once; draw LOD 0 only.
        # Every node is drawn: a building's tall structure lives in its i*
        # nodes, so filtering them out leaves it far too short.
        wanted = []
        for node in m.nodes:
            for index in node.slots_for_lod(0):
                if index < len(m.slots):
                    slot = m.slots[index]
                    wanted += list(range(slot.first_batch, slot.first_batch + slot.batch_count))
        if not wanted:
            wanted = list(range(len(m.batches)))
        wanted = sorted(set(i for i in wanted if i < len(m.batches)))

        # Compact to just the vertices those batches touch.
        remap: dict[int, int] = {}
        verts: list[int] = []
        groups = []
        idx_values: list[int] = []
        for bi in wanted:
            b = m.batches[bi]
            first, count = b.triangles
            start = len(idx_values)
            for t in range(first, min(first + count, len(m.triangles))):
                for v in m.triangles[t]:
                    if v not in remap:
                        remap[v] = len(verts)
                        verts.append(v)
                    idx_values.append(remap[v])
            texture = None
            if 0 <= b.material < len(wear):
                texture = self.materials.texture_for(wear[b.material])
            groups.append({
                "start": start,
                "count": len(idx_values) - start,
                "material": self.textures.resolve(texture) if texture else -1,
            })
        groups = [g for g in groups if g["count"]]
        if not verts:
            self._by_ref[key] = None
            return None

        # Quantise in the viewer's axis order: game space is Z-up, the viewer
        # is Y-up, and a scale factor can rescale components but not reorder
        # them.
        pts = [m.positions[v] for v in verts]
        lox, hix = min(p[0] for p in pts), max(p[0] for p in pts)
        loy, hiy = min(p[1] for p in pts), max(p[1] for p in pts)
        loz, hiz = min(p[2] for p in pts), max(p[2] for p in pts)
        # Rest the model on its own base rather than its centre.  Buildings
        # are authored symmetric about z = 0 -- fr_m_bunker spans -9.04..9.04,
        # fr_l_plant -32.95..32.95 -- so placing z = 0 at ground level buries
        # half of them, which is what the mission z does.  Units and rocks are
        # already authored base-at-origin, so this shift is a no-op for them.
        # The engine's real datum has not been found; see docs/07-objects.md.
        centre = ((lox + hix) / 2, (hiz - loz) / 2, -(loy + hiy) / 2)
        half = [
            max((hix - lox) / 2, 1e-6),
            max((hiz - loz) / 2, 1e-6),
            max((hiy - loy) / 2, 1e-6),
        ]
        pos = bytearray()
        uv = bytearray()
        for v in verts:
            x, y, z = m.positions[v]
            for value, mid, span in zip((x, z, -y), centre, half, strict=True):
                q = round((value - mid) / span * 32767)
                pos += struct.pack("<h", max(-32767, min(32767, q)))
            u, w = m.uv[v]
            uv += struct.pack(
                "<2H",
                min(0xFFFF, round(u * objmesh.UV_FIXED_POINT_SCALE)),
                min(0xFFFF, round(w * objmesh.UV_FIXED_POINT_SCALE)),
            )

        wide = len(verts) > 0xFFFF
        idx = bytearray()
        for v in idx_values:
            idx += struct.pack("<I" if wide else "<H", v)

        self._by_ref[key] = len(self.models)
        self.models.append({
            "name": ref.member,
            "wide": wide,
            "position": _b64(bytes(pos)),
            "uv": _b64(bytes(uv)),
            "index": _b64(bytes(idx)),
            "scale": [round(v, 4) for v in half],
            "offset": [round(v, 4) for v in centre],
            "groups": groups,
            "tris": len(idx_values) // 3,
        })
        return self._by_ref[key]

    def resolve(self, obj: mission.MissionObject) -> int | None:
        """Index into ``self.models``, or None when nothing resolves."""
        if obj.path in self._by_name:
            return self._by_name[obj.path]
        ref = None
        if obj.is_static:
            ref = self._record_mesh(self.library.get(obj.path))
        else:
            f = self._unit_file(obj.path)
            if f is not None:
                unit = objects.load_unit(f)
                if unit.components:
                    ref = self._record_mesh(
                        self.library.get(unit.components[0].ref.member)
                    )
        slot = self._pack(ref) if ref else None
        self._by_name[obj.path] = slot
        return slot


def build_mission_payload(
    m: mission.Mission, map_index: int, models: ModelLibrary | None = None
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
        if models is not None:
            slot = models.resolve(o)
            if slot is not None:
                entry["m"] = slot
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
