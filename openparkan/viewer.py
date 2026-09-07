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

from . import landmesh, materials, mission, objects, sky, texm
from . import mesh as objmesh
from .nres import NResArchive
from .png import _chunk

#: Last resort for a name that reaches neither Material.lib nor Textures.lib.
_FALLBACK_COLOUR = (0x80, 0x80, 0x80, 1.0)


class _Image:
    """The width and height ``_downsample`` needs, for already-cropped pixels."""

    def __init__(self, width: int, height: int):
        self.width = width
        self.height = height


def _png_data_uri(width: int, height: int, pixels: bytes, alpha: bool = False) -> str:
    stride = 4 if alpha else 3
    raw = bytearray()
    row = width * stride
    for y in range(height):
        raw.append(0)
        raw += pixels[y * row : (y + 1) * row]
    blob = (
        b"\x89PNG\r\n\x1a\n"
        + _chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6 if alpha else 2, 0, 0, 0))
        + _chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + _chunk(b"IEND", b"")
    )
    return "data:image/png;base64," + base64.b64encode(blob).decode("ascii")


def _b64(arr: bytes) -> str:
    """Deflate a geometry buffer and base64 it.

    Base64 costs a third on top of the bytes; deflating first pays that back
    twice over, because quantised positions and indices are highly
    compressible.  Across the 33 maps it halves the page.
    """
    return base64.b64encode(zlib.compress(arr, 9)).decode("ascii")


class TextureResolver:
    """Resolves a material name into a pool of images shared by every map.

    Maps reuse the same handful of ground textures, so the pool is what keeps a
    33-map viewer from embedding the same PNG thirty-three times.

    A name from ``Land1.wea`` or a model's wear is a **material** name, and it
    goes through ``Material.lib`` first.  That is what makes ``WATER``,
    ``B_S0`` and ``ENV_NLAVA`` resolve: they are not in ``Textures.lib`` under
    those names, but their materials point at ``WATER0.0``, ``B_FOUND.0`` and
    ``LAV00.0``.  A direct lookup remains the fallback, since most material
    names and texture names coincide.
    """

    def __init__(self, game: Path, max_size: int = 128):
        self.archive = NResArchive.open(game / "Textures.lib")
        self.materials = materials.MaterialLibrary(game / "Material.lib")
        self.lightmaps = NResArchive.open(game / "lightmap.lib")
        self._lightmap_index = {e.name.upper(): e for e in self.lightmaps}
        self._by_lightmap: dict[str, int] = {}
        self.max_size = max_size
        self.index = {}
        for e in self.archive:
            # entries are named like "L04.0" -- strip the trailing sub-index
            self.index.setdefault(e.name.split(".")[0].upper(), e)
        self.pool: list[dict] = []
        self._by_name: dict[str, int] = {}
        self._frames: dict[str, list[int]] = {}

    def _downsample(
        self, tex, pixels: bytes, stride: int = 3, max_size: int | None = None
    ) -> tuple[int, int, bytes]:
        """Halve until within ``max_size``; terrain tiles do not need mip 0."""
        limit = self.max_size if max_size is None else max_size
        w, h = tex.width, tex.height
        while max(w, h) > limit and w % 2 == 0 and h % 2 == 0:
            nw, nh = w // 2, h // 2
            out = bytearray(nw * nh * stride)
            for y in range(nh):
                for x in range(nw):
                    o = (y * nw + x) * stride
                    for c in range(stride):
                        a = pixels[((2 * y) * w + 2 * x) * stride + c]
                        b = pixels[((2 * y) * w + 2 * x + 1) * stride + c]
                        d = pixels[((2 * y + 1) * w + 2 * x) * stride + c]
                        e = pixels[((2 * y + 1) * w + 2 * x + 1) * stride + c]
                        out[o + c] = (a + b + d + e) // 4
            pixels, w, h = bytes(out), nw, nh
        return w, h, pixels

    def sprite(self, name: str, max_size: int = 256) -> int | None:
        """Pool index for a material's texture, cropped to its own cell.

        A material names a texture *and* a cell of it -- ``SUN.0`` is a 2 x 2
        sheet holding a sun corona and a moon, and ``ENV_SUN`` and
        ``ENV_MOON`` are the same texture with different cells.
        """
        key = f"sprite:{name.upper()}:{max_size}"
        if key in self._by_name:
            return self._by_name[key]
        material = self.materials.get(name)
        if material is None or not material.textures:
            return None
        entry = self.index.get(material.textures[0].upper().split(".")[0])
        if entry is None:
            return None
        tex = texm.decode(self.archive.read(entry))
        pixels, width, height = tex.rgba, tex.width, tex.height
        box = material.cell_uv
        if box is not None:
            u0, v0, u1, v1 = box
            x0, y0 = round(u0 * width), round(v0 * height)
            x1, y1 = round(u1 * width), round(v1 * height)
            cropped = bytearray()
            for y in range(y0, y1):
                cropped += pixels[(y * width + x0) * 4 : (y * width + x1) * 4]
            pixels, width, height = bytes(cropped), x1 - x0, y1 - y0
        shrunk = _Image(width, height)
        w, h, pixels = self._downsample(shrunk, pixels, 4, max_size)
        self._by_name[key] = len(self.pool)
        self.pool.append({
            "kind": "texture",
            "name": name,
            "url": _png_data_uri(w, h, pixels, True),
            "opacity": 1.0,
            "cutout": False,
            "graded": True,
        })
        return self._by_name[key]

    def lightmap(self, name: str) -> int | None:
        """Pool index for a ``lightmap.lib`` member, or None if it has none.

        Baked lighting is kept at full size: it is one page for a whole
        building, so halving it costs much more than it saves.
        """
        key = name.upper()
        if key not in self._by_lightmap:
            entry = self._lightmap_index.get(key)
            if entry is None:
                self._by_lightmap[key] = None
            else:
                tex = texm.decode(self.lightmaps.read(entry))
                self._by_lightmap[key] = len(self.pool)
                self.pool.append({
                    "kind": "texture",
                    "name": name,
                    "url": _png_data_uri(tex.width, tex.height, texm.to_rgb(tex)),
                    "opacity": 1.0,
                    "cutout": False,
                    "graded": False,
                })
        return self._by_lightmap[key]

    def tint(self, name: str) -> int:
        """The material's diffuse colour, packed for the viewer."""
        r, g, b = self.materials.colour_for(name.upper().split(".")[0])
        return (r << 16) | (g << 8) | b

    def frames(self, name: str) -> list[int]:
        """Pool indices for every animation frame of a material.

        One entry for a still surface; ten for ``WATER_M``, which is what a
        Parkan lake actually is.
        """
        key = name.upper().split(".")[0]
        if key not in self._frames:
            names = [
                n for n in self.materials.frames_for(key)
                if n.upper().split(".")[0] in self.index
            ]
            pool = [self._image(n) for n in names]
            # ENV_NLAVA names LAV00.0 eight times and FIRE_SMOKE_W resolves
            # every frame to the one FAIR.0 that shipped: a flip-book of one
            # image is not an animation.
            if len(set(pool)) > 1:
                self._frames[key] = pool
            else:
                self._frames[key] = [self.resolve(name)]
        return self._frames[key]

    def resolve(self, name: str) -> int:
        """Index in ``self.pool`` for a material or texture name.

        Materials come first: a terrain layer names a material, and eight of
        them have no same-named texture at all.
        """
        key = name.upper().split(".")[0]
        if key in self._by_name:
            return self._by_name[key]
        material = self.materials.get(key)
        if material:
            # The first texture the material names that is actually shipped.
            # Eight materials point at frames that are not in Textures.lib --
            # the FIRE_SMOKE animations name 0FAIR.0 upwards and only FAIR.0
            # exists -- so falling through the list rescues the mixed ones.
            for candidate in material.textures:
                if candidate.upper().split(".")[0] in self.index:
                    index = self._image(candidate)
                    self._by_name[key] = index
                    return index
        return self._image(name)

    def _image(self, name: str) -> int:
        """Index in ``self.pool`` for a name looked up in Textures.lib."""
        key = name.upper().split(".")[0]
        if key in self._by_name:
            return self._by_name[key]
        entry = self.index.get(key)
        if entry is not None:
            tex = texm.decode(self.archive.read(entry))
            # 241 of the 393 shipped textures carry alpha, and the foliage is
            # among them: a tree is a pair of crossed planes that only reads
            # as a tree once the texture cuts its own silhouette out.
            alpha = tex.rgba[3::4]
            cutout = any(v < 255 for v in alpha)
            if cutout:
                w, h, pixels = self._downsample(tex, tex.rgba, 4)
            else:
                w, h, pixels = self._downsample(tex, texm.to_rgb(tex, (90, 90, 90)))
            mat = {
                "kind": "texture",
                "name": name,
                "url": _png_data_uri(w, h, pixels, cutout),
                "opacity": 1.0,
                # A cutout is drawn with an alpha test rather than blending:
                # it needs no depth sorting, and it is what the fixed-function
                # hardware this was written for could do.
                "cutout": cutout,
                "graded": cutout and any(0 < v < 255 for v in alpha),
            }
        else:
            r, g, b, a = _FALLBACK_COLOUR
            mat = {
                "kind": "colour",
                "name": name,
                "colour": (r << 16) | (g << 8) | b,
                "opacity": a,
                "cutout": False,
                "graded": False,
            }
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
    uv2 = bytearray()
    blend = bytearray()
    for i in range(nv):
        x, y, z = mesh.positions[i]
        for value, mid, span in zip((x - cx, z, -(y - cy)), tmid, tspan, strict=True):
            q = round((value - mid) / span * 32767)
            pos += struct.pack("<h", max(-32767, min(32767, q)))
        # Normals are int8 in the file; keep them there rather than widening
        # to int16 for no gain in precision.
        a, b, c = mesh.normals[i]
        nrm += struct.pack(
            "<3b",
            max(-127, min(127, round(a * 127))),
            max(-127, min(127, round(c * 127))),
            max(-127, min(127, round(-b * 127))),
        )
        for target, source in ((uv, mesh.uv1), (uv2, mesh.uv2)):
            u, v = source[i]
            target += struct.pack(
                "<2H",
                min(0xFFFF, round(u * landmesh.UV_FIXED_POINT_SCALE)),
                min(0xFFFF, round(v * landmesh.UV_FIXED_POINT_SCALE)),
            )
        # Stream 14 is the weight of layer 1: it is exactly 1.0 on every
        # vertex that no layer-2 face touches, and drops below it on 46% of
        # the ones that do.  The viewer draws layer 2 over layer 1 with
        # alpha 1 - blend, which is the same mix.
        blend += struct.pack("B", max(0, min(255, round(mesh.blend[i] * 255))))

    # One draw group per (layer 1, layer 2, water) combination -- 5 to 8 per
    # map -- so both ground layers are drawn in a single pass.  Drawing layer
    # 2 as a second, coplanar mesh made the walkable ground flicker: the two
    # passes compile to different shader programs, and their depths come out
    # a hair apart.
    buckets: dict[tuple[int, int, bool], list[int]] = {}
    for fi in range(mesh.face_count):
        key = (mesh.face_tex1[fi], mesh.face_tex2[fi], mesh.is_water(fi))
        buckets.setdefault(key, []).append(fi)

    wide = nv > 0xFFFF
    idx = bytearray()
    groups = []
    materials = []
    for (tex1, tex2, is_water), face_ids in sorted(buckets.items()):
        start = len(idx) // (4 if wide else 2)
        for fi in face_ids:
            a, b, c = mesh.faces[fi]
            idx += struct.pack("<3I" if wide else "<3H", a, b, c)
        name = mesh.texture_name(1, tex1) or "?"
        frames = resolver.frames(name)
        groups.append({"start": start, "count": len(face_ids) * 3, "material": len(materials)})
        entry = {"pool": frames[0], "water": is_water, "tint": resolver.tint(name)}
        if len(frames) > 1:
            entry["frames"] = frames
        second = mesh.texture_name(2, tex2)
        if second:
            over = resolver.frames(second)
            entry["pool2"] = over[0]
            entry["tint2"] = resolver.tint(second)
            if len(over) > 1:
                entry["frames2"] = over
        materials.append(entry)

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
        "uv2": _b64(bytes(uv2)),
        "blend": _b64(bytes(blend)),
        "index": _b64(bytes(idx)),
        "groups": groups,
        "materials": materials,
        "layer2Faces": sum(
            1 for i in range(mesh.face_count)
            if mesh.face_tex2[i] != landmesh.NO_TEXTURE
        ),
        "layer1": mesh.layer1_names,
        "layer2": mesh.layer2_names,
    }


#: Clan colours, chosen to stay distinguishable on both the light and dark ground.
CLAN_COLOURS = [0x3E7CB1, 0xC1453C, 0x4E9A51, 0xB07A2A, 0x7B5EA7, 0x2E9A96]
NEUTRAL_COLOUR = 0x8A8A85


class ModelLibrary:
    """Resolves any placed mission object to renderable geometry.

    Two chains meet here.  Scenery names an ``objects.rlb`` record directly.
    A building or unit names a ``UNITS/**/*.dat`` assembly, whose components
    form a tree -- and for buildings the root record is a ``FORT`` whose first
    slot points at a second record that finally carries the mesh.

    An assembly is drawn whole.  Each component says which node of its
    parent's mesh it bolts onto, so a robot is built by composing chassis,
    turret and guns down that tree; the internal systems and ammunition are
    modelled but never visible from outside, so they are skipped.  See
    docs/07-objects.md.
    """

    MAX_INDIRECTION = 3

    def __init__(self, game: Path, textures: TextureResolver):
        self.game = game
        self.library = objects.ObjectLibrary(game / "objects.rlb")
        self.materials = textures.materials
        self.textures = textures
        self._archives: dict[str, NResArchive] = {}
        self.models: list[dict] = []
        self._meshes: dict[tuple[str, str], objmesh.ObjectMesh | None] = {}
        self._wears: dict[tuple[str, str], objmesh.Wear] = {}
        self._by_name: dict[str, int | None] = {}
        self._by_parts: dict[tuple, int | None] = {}

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

    def _mesh(self, ref) -> objmesh.ObjectMesh | None:
        """Parse a mesh once, wear and all."""
        key = (ref.library, ref.member)
        if key not in self._meshes:
            try:
                archive = self._archive(ref.library)
                try:
                    wear = objmesh.parse_wear(
                        archive.read_name(ref.member.replace(".msh", ".wea"))
                    )
                except KeyError:
                    wear = objmesh.Wear()
                self._wears[key] = wear
                self._meshes[key] = objmesh.parse(
                    archive.read_name(ref.member), ref.member, wear.materials
                )
            except (KeyError, ValueError, struct.error):
                self._meshes[key] = None
        return self._meshes[key]

    def _parts(self, obj: mission.MissionObject) -> list[tuple[object, objmesh.Pose]]:
        """Every visible mesh of a placed object, with where it sits."""
        if obj.is_static:
            ref = self._record_mesh(self.library.get(obj.path))
            return [(ref, objmesh.IDENTITY_POSE)] if ref else []

        f = self._unit_file(obj.path)
        if f is None:
            return []
        try:
            unit = objects.load_unit(f)
            parents = unit.parents()
        except (objects.ObjectFormatError, OSError, struct.error):
            return []

        refs: list[object] = []
        poses: list[objmesh.Pose] = []
        out = []
        for i, component in enumerate(unit.components):
            ref = self._record_mesh(self.library.get(component.ref.member))
            refs.append(ref)
            parent = parents[i]
            pose = objmesh.IDENTITY_POSE
            if parent >= 0:
                pose = poses[parent]
                host = self._mesh(refs[parent]) if refs[parent] else None
                if host and 0 <= component.attach_node < len(host.nodes):
                    # Position only.  A socket carries the same rotation as
                    # the root node of the part that plugs into it -- on 1306
                    # of the 1414 attachments the game ships -- so composing
                    # the two applies the turn twice and guns end up pointing
                    # sideways.  Cancelling it instead fixes those but flips
                    # the other 108 upside down, since their socket and root
                    # disagree by 180 degrees.  Taking the socket's position
                    # and leaving the part in its own orientation is right in
                    # both groups.  See docs/07-objects.md.
                    socket = host.world_pose(component.attach_node)[0]
                    pose = objmesh.compose(pose, (socket, objmesh.IDENTITY_POSE[1]))
            poses.append(pose)
            if ref and component.is_external:
                out.append((ref, pose))
        return out

    def _pack(self, parts: list[tuple[object, objmesh.Pose]]) -> int | None:
        """Turn a list of posed meshes into one payload the viewer can draw."""
        key = tuple(
            (r.library, r.member, tuple(round(v, 4) for v in p[0] + p[1]))
            for r, p in parts
        )
        if key in self._by_parts:
            return self._by_parts[key]

        points: list[tuple[float, float, float]] = []
        uvs: list[tuple[float, float]] = []
        lightmap_uvs: list[tuple[float, float]] = []
        groups: list[dict] = []
        idx_values: list[int] = []
        for ref, pose in parts:
            m = self._mesh(ref)
            if m is None:
                continue
            wear_record = self._wears.get((ref.library, ref.member))
            baked = None
            if wear_record and wear_record.lightmaps and m.lightmap_uv:
                baked = self.textures.lightmap(wear_record.lightmaps[0])
            # Vertices are authored in their own node's frame; without the
            # node poses applied a multi-part model draws every part piled on
            # the origin.  See docs/07-objects.md.
            positions = m.posed_positions(0)
            wear = m.texture_names

            # A model holds up to three levels of detail at once; draw LOD 0
            # only.  Every node is drawn: a building's tall structure lives in
            # its i* nodes, so filtering them out leaves it far too short.
            wanted: list[int] = []
            for node in m.nodes:
                for index in node.slots_for_lod(0):
                    if index < len(m.slots):
                        slot = m.slots[index]
                        wanted += list(
                            range(slot.first_batch, slot.first_batch + slot.batch_count)
                        )
            if not wanted:
                wanted = list(range(len(m.batches)))
            wanted = sorted({i for i in wanted if i < len(m.batches)})

            # Compact to just the vertices those batches touch.
            remap: dict[int, int] = {}
            for bi in wanted:
                b = m.batches[bi]
                first, count = b.triangles
                start = len(idx_values)
                for t in range(first, min(first + count, len(m.triangles))):
                    for v in m.triangles[t]:
                        if v not in remap:
                            remap[v] = len(points)
                            points.append(objmesh.apply(pose, positions[v]))
                            uvs.append(m.uv[v] if v < len(m.uv) else (0.0, 0.0))
                            lightmap_uvs.append(
                                m.lightmap_uv[v] if v < len(m.lightmap_uv) else (0.0, 0.0)
                            )
                        idx_values.append(remap[v])
                name = wear[b.material] if 0 <= b.material < len(wear) else None
                frames = self.textures.frames(name) if name else []
                group = {
                    "start": start,
                    "count": len(idx_values) - start,
                    "material": frames[0] if frames else -1,
                    "tint": self.textures.tint(name) if name else 0xFFFFFF,
                }
                if baked is not None and b.is_lit:
                    group["lightmap"] = baked
                if len(frames) > 1:
                    group["frames"] = frames
                groups.append(group)
        groups = [g for g in groups if g["count"]]
        if not points:
            self._by_parts[key] = None
            return None

        # Quantise in the viewer's axis order: game space is Z-up, the viewer
        # is Y-up, and a scale factor can rescale components but not reorder
        # them.
        lox, hix = min(p[0] for p in points), max(p[0] for p in points)
        loy, hiy = min(p[1] for p in points), max(p[1] for p in points)
        loz, hiz = min(p[2] for p in points), max(p[2] for p in points)
        # No vertical fudge: a model's own z = 0 is its ground contact point.
        # Over 864 shipped placements a building's origin lands within a
        # median 0.00 of the terrain height under it, and a unit's lowest
        # exterior vertex within 0.08 -- see docs/07-objects.md.  This only
        # reads that way once node poses are applied; before that a building's
        # parts pile up on the origin and it looks half-buried.
        centre = ((lox + hix) / 2, (loz + hiz) / 2, -(loy + hiy) / 2)
        half = [
            max((hix - lox) / 2, 1e-6),
            max((hiz - loz) / 2, 1e-6),
            max((hiy - loy) / 2, 1e-6),
        ]
        pos = bytearray()
        uv = bytearray()
        uv2 = bytearray()
        baked_any = any("lightmap" in g for g in groups)
        for (x, y, z), (u, w), (lu, lw) in zip(points, uvs, lightmap_uvs, strict=True):
            for value, mid, span in zip((x, z, -y), centre, half, strict=True):
                q = round((value - mid) / span * 32767)
                pos += struct.pack("<h", max(-32767, min(32767, q)))
            uv += struct.pack(
                "<2H",
                min(0xFFFF, max(0, round(u * objmesh.UV_FIXED_POINT_SCALE))),
                min(0xFFFF, max(0, round(w * objmesh.UV_FIXED_POINT_SCALE))),
            )
            if baked_any:
                uv2 += struct.pack(
                    "<2H",
                    min(0xFFFF, max(0, round(lu * objmesh.LIGHTMAP_UV_SCALE))),
                    min(0xFFFF, max(0, round(lw * objmesh.LIGHTMAP_UV_SCALE))),
                )

        wide = len(points) > 0xFFFF
        idx = bytearray()
        for v in idx_values:
            idx += struct.pack("<I" if wide else "<H", v)

        self._by_parts[key] = len(self.models)
        self.models.append({
            "name": parts[0][0].member,
            "parts": len(parts),
            "wide": wide,
            "position": _b64(bytes(pos)),
            "uv": _b64(bytes(uv)),
            "uv2": _b64(bytes(uv2)) if baked_any else None,
            "index": _b64(bytes(idx)),
            "scale": [round(v, 4) for v in half],
            "offset": [round(v, 4) for v in centre],
            "groups": groups,
            "tris": len(idx_values) // 3,
        })
        return self._by_parts[key]

    def resolve(self, obj: mission.MissionObject) -> int | None:
        """Index into ``self.models``, or None when nothing resolves."""
        if obj.path in self._by_name:
            return self._by_name[obj.path]
        parts = self._parts(obj)
        slot = self._pack(parts) if parts else None
        self._by_name[obj.path] = slot
        return slot


#: Slots of a sky keyframe the viewer draws with.  Which of the three colour
#: groups is the dome and which are fog and ambient is not established, but
#: the first group is the one that tracks the day and the second is
#: consistently lighter, so they read as zenith and horizon.  See
#: docs/10-sky.md.
SKY_ZENITH_SLOT = 1
SKY_HORIZON_SLOT = 7
SKY_SUN_SLOT = 18


def _pack_colour(rgba: tuple[int, int, int, int]) -> int:
    r, g, b, _a = rgba
    return (r << 16) | (g << 8) | b


#: The sky.wea roles the viewer draws, and how far to shrink each.
SKY_LAYERS = (("nebula", 256), ("stars", 256), ("clouds", 256),
              ("sun", 128), ("moon", 128))


def build_sky_payload(folder: Path, resolver: TextureResolver | None = None) -> dict | None:
    """The mission's atmosphere: its whole day cycle, and its sky textures."""
    path = folder / "sky.ske"
    if not path.exists():
        return None
    try:
        atmosphere = sky.load(path)
    except (sky.SkyFormatError, OSError):
        return None
    brightest = atmosphere.brightest()
    if brightest is None:
        return None

    # Every keyframe of the first section, in time order; that is the day and
    # the viewer's time control walks it.
    frames = sorted(
        (k for k in atmosphere.keyframes if k.section == 0), key=lambda k: k.minutes
    )
    peak = max((k.light for k in frames), default=1.0) or 1.0
    payload = {
        "zenith": _pack_colour(brightest.colour(SKY_ZENITH_SLOT)),
        "horizon": _pack_colour(brightest.colour(SKY_HORIZON_SLOT)),
        "sunColour": _pack_colour(brightest.colour(SKY_SUN_SLOT)),
        "light": round(brightest.light, 3),
        "time": f"{brightest.hour:02d}:{brightest.minute:02d}",
        "keyframes": len(atmosphere),
        "textures": [t for t in atmosphere.textures if t],
        "day": [
            {
                "t": f"{k.hour:02d}:{k.minute:02d}",
                "z": _pack_colour(k.colour(SKY_ZENITH_SLOT)),
                "h": _pack_colour(k.colour(SKY_HORIZON_SLOT)),
                "s": _pack_colour(k.colour(SKY_SUN_SLOT)),
                "l": round(k.light, 3),
                # How dark this keyframe is against the day's peak, which is
                # what decides whether the stars show.
                "n": round(1.0 - min(1.0, k.light / peak), 3),
            }
            for k in frames
        ],
        "peak": frames.index(brightest) if brightest in frames else 0,
    }
    if resolver is not None:
        for role, size in SKY_LAYERS:
            name = atmosphere.texture(role)
            index = resolver.sprite(name, size) if name else None
            if index is not None:
                payload[role] = index
                payload[role + "Name"] = name
    return payload


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
        "sky": build_sky_payload(folder, models.textures if models else None),
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
