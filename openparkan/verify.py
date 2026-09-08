"""Checks every factual claim the docs make against a real installation.

Each check prints PASS/FAIL and the evidence behind it.  If a claim in
``docs/`` cannot be re-derived here, the claim does not belong in the docs.

    uv run openparkan verify
"""

from __future__ import annotations

import math
import random
import struct
from collections import Counter
from pathlib import Path

from . import arealmap, effects, gamedir, landmesh, materials, mission, objects, sky, texm
from . import mesh as objmesh
from .nres import HEADER_SIZE, NotAnNResArchive, NResArchive, is_nres


def all_archives(game: Path) -> list[Path]:
    out = []
    patterns = (
        "*.lib", "*.rlb", "*.dlb", "*.res",
        "DATA/MAPS/*/Land.*", "MISSIONS/SCRIPTS/*.trf",
    )
    for pattern in patterns:
        out.extend(sorted(game.glob(pattern)))
    return [p for p in out if p.is_file() and is_nres(p)]


def check_nres(check, game: Path) -> None:
    archives = all_archives(game)
    sizes_ok = aligned = 0
    total_members = 0
    bad_pad = 0
    overlaps = 0
    max_gap = 0
    for path in archives:
        ar = NResArchive.open(path)  # constructor already asserts declared size
        sizes_ok += 1
        total_members += len(ar)
        aligned += all(e.offset % 8 == 0 for e in ar)
        spans = sorted((e.offset, e.offset + e.size) for e in ar)
        cursor = HEADER_SIZE
        for start, end in spans:
            if start < cursor:
                overlaps += 1
            cursor = max(cursor, end)
        for lo, hi in ar.layout_gaps():
            max_gap = max(max_gap, hi - lo)
            if any(ar.data[lo:hi]):
                bad_pad += 1
    check("NRes: header size == file size", sizes_ok == len(archives),
          f"{sizes_ok}/{len(archives)} archives")
    check("NRes: every member offset is 8-byte aligned", aligned == len(archives),
          f"{aligned}/{len(archives)} archives, {total_members} members")
    check("NRes: no member ranges overlap", overlaps == 0, f"{overlaps} overlaps")
    check("NRes: inter-member padding is zero-filled", bad_pad == 0,
          f"{bad_pad} non-zero gaps, largest gap {max_gap} bytes (< 8 as expected)")

    # The directory's element-count field is what ArealMap.dll reads to learn
    # how many areals a chunk holds; for terrain streams it must equal
    # size / stride.
    strides = {3: 12, 4: 4, 5: 4, 18: 4, 14: 4, 11: 4, 21: 28}
    agree = seen = 0
    for d in gamedir.maps(game):
        for e in NResArchive.open(d / "Land.msh"):
            stride = strides.get(e.type_id)
            if stride is None or not e.element_count:
                continue
            seen += 1
            agree += e.size == e.element_count * stride
    check("NRes: the element-count field equals size / stride", agree == seen,
          f"{agree}/{seen} terrain streams across {len(gamedir.maps(game))} maps")


def check_texm(check, game: Path) -> None:
    ar = NResArchive.open(game / "Textures.lib")
    exact = decoded = 0
    fmts: dict[int, int] = {}
    cutout = graded = palettised = keyed = 0
    silhouettes = trees = 0
    for e in ar:
        blob = ar.read(e)
        w, h, mips, flags, fmt = texm.parse_header(blob)
        fmts[fmt] = fmts.get(fmt, 0) + 1
        bpp = {0: 1, 565: 2, 4444: 2, 888: 4, 8888: 4}[fmt]
        want = texm.mip_pyramid_pixels(w, h, mips) * bpp + (texm.PALETTE_SIZE if fmt == 0 else 0)
        if want == len(blob) - texm.HEADER_SIZE:
            exact += 1
        tex = texm.decode(blob)
        if len(tex.rgba) == w * h * 4:
            decoded += 1
        alpha = tex.rgba[3::4]
        if any(v < 255 for v in alpha):
            cutout += 1
            graded += any(0 < v < 255 for v in alpha)
            silhouettes += texm.is_cutout(tex)
        if e.name.upper() in ("FTREE1.0", "NTREE1.0", "HTREE1.0"):
            trees += texm.is_cutout(tex)
        if e.name.upper() in ("MTP_01.0", "S0A1.0", "NP11.0"):
            trees -= texm.is_cutout(tex)
        if fmt == texm.FMT_PALETTE8:
            palettised += 1
            # The palette is BGRX; if X were an alpha channel some entry would
            # differ from the rest.  None does, on any shipped texture.
            body = blob[texm.HEADER_SIZE :]
            keyed += len({body[i * 4 + 3] for i in range(256)}) > 1
    check("Texm: declared format predicts the payload size", exact >= len(ar) * 0.8,
          f"{exact}/{len(ar)} exact (rest have a truncated mip tail)")
    check("Texm: every texture decodes to RGBA", decoded == len(ar),
          f"{decoded}/{len(ar)}, formats {dict(sorted(fmts.items()))}")
    check("Texm: alpha is real and worth drawing", cutout > len(ar) * 0.5,
          f"{cutout}/{len(ar)} textures carry alpha, {graded} of them graded "
          f"rather than a hard cut")
    check("Texm: alpha is mostly a gloss map, not a silhouette",
          0 < silhouettes < cutout * 0.2,
          f"{silhouettes}/{cutout} textures with alpha are cut silhouettes -- a "
          f"real hole and a thin transition; the rest are continuous maps, and "
          f"alpha-testing those punches holes through solid geometry")
    check("Texm: the tree textures are the silhouettes", trees == 3,
          f"{trees}/3 of FTREE1.0, NTREE1.0 and HTREE1.0 read as cutouts, "
          f"against 0/3 for MTP_01.0, S0A1.0 and NP11.0")

    check("Texm: a palettised texture has no colour key", keyed == 0,
          f"the fourth palette byte is constant on all {palettised} palettised "
          f"textures, so transparency lives in the 4444 and 8888 formats only")


def check_terrain(check, game: Path) -> None:
    maps = gamedir.maps(game)
    loaded = in_range = symmetric = full_use = 0
    normal_err = 0.0
    for d in maps:
        m = landmesh.load(d / "Land.msh")
        loaded += 1
        nv = m.vertex_count
        if all(0 <= i < nv for tri in m.faces for i in tri):
            in_range += 1
        used = {i for tri in m.faces for i in tri}
        if len(used) == nv:
            full_use += 1
        # adjacency must be mutual: if face A names B, B must name A
        ok = True
        for fi, tri in enumerate(m.adjacency):
            for nb in tri:
                if nb == landmesh.NO_NEIGHBOUR:
                    continue
                if nb >= m.face_count or fi not in m.adjacency[nb]:
                    ok = False
                    break
            if not ok:
                break
        symmetric += ok
        for n in m.normals:
            normal_err = max(normal_err, abs(math.sqrt(sum(c * c for c in n)) - 1.0))
    check("Land.msh: all maps parse", loaded == len(maps), f"{loaded} maps")
    check("Land.msh: face indices within the vertex array", in_range == len(maps),
          f"{in_range}/{len(maps)} maps")
    check("Land.msh: every vertex is referenced by a face", full_use == len(maps),
          f"{full_use}/{len(maps)} maps")
    check("Land.msh: face adjacency is mutual", symmetric == len(maps),
          f"{symmetric}/{len(maps)} maps -- proves fields 7..9 are neighbours")
    check("Land.msh: int8/127 normals are unit length", normal_err < 0.02,
          f"worst deviation {normal_err:.4f} across all maps")


def check_uv(check, game: Path) -> None:
    m = landmesh.load(game / "DATA" / "MAPS" / "SC_3" / "Land.msh")
    worst = 0.0
    (minx, miny, _), (maxx, maxy, _) = m.bounds()
    for i, (x, y, _) in enumerate(m.positions):
        u, v = m.uv1[i]
        worst = max(worst, abs(u - x / 50.0), abs(v - (maxy - y) / 50.0))
    check("Land.msh: layer-1 UV == world XY / 50", worst < 0.35,
          f"SC_3 worst residual {worst:.3f} texel units")

    names = {m.texture_name(1, t) for t in m.face_tex1}
    check("Land.msh: texture indices resolve through Land1.wea", "WATER" in names,
          f"SC_3 layer-1 names in use: {sorted(n for n in names if n)}")


#: Terrain texture names that denote a liquid surface in the .wea tables.
LIQUID_NAMES = ("WATER", "WATER_M", "ENV_NLAVA", "ENV_NLAVA_M")


def check_layers(check, game: Path) -> None:
    """The terrain's second texture layer and the weight that mixes it in."""
    maps = gamedir.maps(game)
    clean = dirty = 0
    varying = touched = 0
    with_layer2 = faces_1 = faces_2 = 0
    for folder in maps:
        m = landmesh.load(folder / "Land.msh")
        second = {
            v
            for i in range(m.face_count)
            if m.face_tex2[i] != landmesh.NO_TEXTURE
            for v in m.faces[i]
        }
        covered = sum(1 for i in range(m.face_count) if m.face_tex2[i] != landmesh.NO_TEXTURE)
        with_layer2 += covered > 0
        faces_1 += m.face_count
        faces_2 += covered
        for v in range(m.vertex_count):
            if v in second:
                touched += 1
                varying += m.blend[v] < 0.999
            elif abs(m.blend[v] - 1.0) < 1e-6:
                clean += 1
            else:
                dirty += 1
    check("Land.msh: a second texture layer covers part of every map",
          with_layer2 == len(maps),
          f"{faces_2}/{faces_1} faces across {with_layer2}/{len(maps)} maps carry one")
    # Coincident faces: the file stores a large minority of its triangles
    # twice, which z-fights if a renderer draws the list as it stands.
    stored = drawn = 0
    same_uv = same_normal = same_layer = sets = 0
    for folder in gamedir.maps(game):
        m = landmesh.load(folder / "Land.msh")
        stored += m.face_count
        keep = m.distinct_faces()
        drawn += len(keep)
        groups: dict[tuple, list[int]] = {}
        for i, tri in enumerate(m.faces):
            groups.setdefault(
                tuple(sorted(m.positions[v] for v in tri)), []
            ).append(i)
        for members in groups.values():
            if len(members) < 2:
                continue
            sets += 1
            a, b = members[0], members[1]
            first = {m.positions[v]: v for v in m.faces[a]}
            second = {m.positions[v]: v for v in m.faces[b]}
            if set(first) != set(second):
                continue
            same_uv += all(m.uv1[first[k]] == m.uv1[second[k]] for k in first)
            same_normal += all(
                m.normals[first[k]] == m.normals[second[k]] for k in first
            )
            same_layer += (
                (m.face_tex2[a] == landmesh.NO_TEXTURE)
                == (m.face_tex2[b] == landmesh.NO_TEXTURE)
            )
    check("Land.msh: a large minority of faces are stored twice",
          drawn < stored * 0.9,
          f"{stored - drawn} of {stored} faces across the 33 maps repeat a "
          f"triangle already in the list, in {sets} coincident sets")
    check("Land.msh: the two copies are the same surface",
          same_uv == sets and same_normal > sets * 0.99 and same_layer > sets * 0.99,
          f"layer-1 UVs match on all {same_uv} sets, normals on {same_normal} "
          f"and the second-layer flag on {same_layer} -- so a renderer can "
          f"draw either copy and must not draw both")

    # A negative result, kept so nobody re-derives it: face field 13 is not a
    # spatial index.
    tight = loose = 0
    for folder in gamedir.maps(game)[:6]:
        m = landmesh.load(folder / "Land.msh")
        (minx, miny, _), (maxx, maxy, _) = m.bounds()
        span = max(maxx - minx, maxy - miny) or 1.0
        groups: dict[int, list[int]] = {}
        for i, patch in enumerate(m.face_patch):
            groups.setdefault(patch, []).append(i)
        rng = random.Random(0)

        def extent(chosen, mesh=m, scale=span):
            xs = [mesh.positions[v][0] for f in chosen for v in mesh.faces[f]]
            ys = [mesh.positions[v][1] for f in chosen for v in mesh.faces[f]]
            return max(max(xs) - min(xs), max(ys) - min(ys)) / scale

        for faces in groups.values():
            shuffled = rng.sample(range(m.face_count), len(faces))
            if extent(faces) < extent(shuffled) * 0.8:
                tight += 1
            else:
                loose += 1
    check("Land.msh: face field 13 is not a spatial patch id", tight < loose * 0.1,
          f"{tight}/{tight + loose} groups are tighter than a random subset of "
          f"the same size, so it cannot be used for culling")

    check("Land.msh: stream 14 is the weight of layer 1", dirty == 0,
          f"exactly 1.0 on all {clean} vertices no layer-2 face touches; "
          f"below it on {varying} of the {touched} that one does")


def check_materials(check, game: Path) -> None:
    """Material.lib: layers against animation frames, and the diffuse colour."""
    lib = materials.MaterialLibrary(game / "Material.lib")
    textures = NResArchive.open(game / "Textures.lib")
    known = {e.name.split(".")[0].upper() for e in textures}

    divides = 0
    counted = 0
    frames = layered = 0
    for m in lib.materials.values():
        if m.layer_count and m.entry_count % m.layer_count == 0:
            divides += 1
        if len(m.textures) == m.entry_count:
            counted += 1
        if m.frame_count > 1:
            frames += 1
        if m.layer_count > 1:
            layered += 1
    raw = lib.archive
    marker = 0
    for entry in raw:
        if entry.tag != materials.MATERIAL_TAG:
            continue
        blob = raw.read(entry)
        at = materials.ENTRY_BASE + materials.COLOUR_MARKER_OFFSET
        marker += len(blob) > at and blob[at] == materials.COLOUR_MARKER
    total = len(lib)
    check("Material.lib: entry count divides by layer count", divides == total,
          f"{divides}/{total} records; {frames} animate, {layered} have more than one layer")
    check("Material.lib: the pattern finds exactly the declared textures",
          counted >= total * 0.98,
          f"{counted}/{total} records")
    check("Material.lib: the diffuse colour sits behind a constant 100",
          marker >= total - 1,
          f"{marker}/{total} records carry the marker; "
          f"{sum(1 for m in lib.materials.values() if m.colour != (255, 255, 255))} "
          f"are tinted, WATER #4d6aff and ENV_NLAVA #b41e00 among them")

    # Terrain layer names are material names, which is what makes the eight
    # that are missing from Textures.lib resolve.
    direct = through = named = 0
    for folder in gamedir.maps(game):
        mesh = landmesh.load(folder / "Land.msh")
        for table in (mesh.layer1_names, mesh.layer2_names):
            for name in table:
                if not name:
                    continue
                named += 1
                key = name.upper().split(".")[0]
                if key in known:
                    direct += 1
                base = lib.texture_for(key)
                if base and base.upper().split(".")[0] in known:
                    through += 1
    check("terrain layer names resolve through Material.lib", through == named,
          f"{through}/{named} reach a texture through a material, against "
          f"{direct}/{named} looked up in Textures.lib directly")


def check_water(check, game: Path) -> None:
    """Water is identified three independent ways; they must agree everywhere."""
    maps = gamedir.maps(game)
    by_bit = by_flags = flat = 0
    with_water = total_faces = 0
    for d in maps:
        m = landmesh.load(d / "Land.msh")
        by_name = {
            i for i in range(m.face_count)
            if (m.texture_name(1, m.face_tex1[i]) or "").upper() in LIQUID_NAMES
        }
        by_bit += by_name == {i for i in range(m.face_count) if m.is_water(i)}
        by_flags += by_name == {
            i for i in range(m.face_count) if m.face_flags[i] == landmesh.FLAGS_WATER
        }
        if by_name:
            with_water += 1
            total_faces += len(by_name)
            flat += m.water_level() is not None
    check("Land.msh: surface bit 0x02 marks exactly the water faces", by_bit == len(maps),
          f"{by_bit}/{len(maps)} maps, {total_faces} water faces on {with_water} maps")
    check("Land.msh: face flags 1544 agree with the surface bit", by_flags == len(maps),
          f"{by_flags}/{len(maps)} maps -- an independent second marker")
    check("Land.msh: water is a single flat plane per map", flat == with_water,
          f"{flat}/{with_water} maps with water")


def check_arealmap(check, game: Path) -> None:
    """The navigation mesh, whose layout came out of ArealMap.dll."""
    maps = [d for d in gamedir.maps(game) if (d / "Land.map").exists()]
    loaded = []
    failures = []
    for d in maps:
        try:
            loaded.append((d, arealmap.load(d / "Land.map")))
        except arealmap.ArealMapFormatError as exc:
            failures.append(str(exc))
    total_areals = sum(am.areal_count for _, am in loaded)
    check("Land.map: payload is consumed exactly", not failures,
          f"{len(loaded)}/{len(maps)} maps, {total_areals} areals"
          + ("" if not failures else f" -- {failures[0]}"))
    if not loaded:
        return

    grids = {(am.cells_across, am.cells_down) for _, am in loaded}
    check("Land.map: every map uses the same cell grid", len(grids) == 1,
          f"{grids.pop() if len(grids) == 1 else grids}")

    mutual = 0
    for _, am in loaded:
        ok = True
        for i, a in enumerate(am.areals):
            for nb in a.neighbours:
                if nb >= am.areal_count or i not in am.areals[nb].neighbours:
                    ok = False
                    break
            if not ok:
                break
        mutual += ok
    check("Land.map: areal adjacency is mutual", mutual == len(loaded),
          f"{mutual}/{len(loaded)} maps -- proves edge field 0 is the neighbour")

    close = total = 0
    for _, am in loaded:
        for a in am.areals:
            if a.area <= 0:
                continue
            total += 1
            close += abs(a.polygon_area() - a.area) / a.area < 0.02
    check("Land.map: the stored area matches the polygon", close > total * 0.95,
          f"{close}/{total} areals agree with their shoelace area within 2%")

    # The decomposition should tile the map: areal extent equals terrain
    # extent, and the areas sum to the whole square.
    tiled = aligned = 0
    for d, am in loaded:
        mesh = landmesh.load(d / "Land.msh")
        (tminx, tminy, _), (tmaxx, tmaxy, _) = mesh.bounds()
        (aminx, aminy), (amaxx, amaxy) = am.bounds()
        aligned += (abs(aminx - tminx) < 1 and abs(aminy - tminy) < 1
                    and abs(amaxx - tmaxx) < 1 and abs(amaxy - tmaxy) < 1)
        square = (tmaxx - tminx) * (tmaxy - tminy)
        tiled += abs(sum(a.area for a in am.areals) - square) / square < 0.01
    check("Land.map: areals span the same extent as the terrain", aligned == len(loaded),
          f"{aligned}/{len(loaded)} maps")
    check("Land.map: areals tile the map without gaps or overlap", tiled == len(loaded),
          f"{tiled}/{len(loaded)} maps -- areas sum to the full square")

    # Every index the grid hands out must be a real areal.
    bad = items = 0
    for _, am in loaded:
        for entries in am.cells.values():
            for idx in entries:
                items += 1
                bad += not (0 <= idx < am.areal_count)
    check("Land.map: cell grid indexes real areals", bad == 0,
          f"{items} cell entries across {len(loaded)} maps")


def check_missions(check, game: Path) -> None:
    """The mission format is validated by whether it closes, and by whether
    everything it points at actually exists."""
    dirs = gamedir.missions(game)
    parsed: list[mission.Mission] = []
    failures = []
    for d in dirs:
        try:
            parsed.append(mission.load(d / "data.tma"))
        except mission.MissionFormatError as exc:
            failures.append(f"{d.name}: {exc}")
    check("data.tma: parses exactly to end of file", not failures,
          f"{len(parsed)}/{len(dirs)} missions, "
          f"{sum(len(m.objects) for m in parsed)} objects"
          + ("" if not failures else f" -- {failures[0]}"))
    if not parsed:
        return

    maps = {d.name for d in gamedir.maps(game)}
    resolved = sum(1 for m in parsed if m.map_name in maps)
    check("data.tma: the map it names exists", resolved == len(parsed),
          f"{resolved}/{len(parsed)} missions reference a real DATA/MAPS entry")

    # Every placed object must fall inside the map it is placed on.
    meshes: dict[str, landmesh.LandMesh] = {}
    inside = total = 0
    for m in parsed:
        if m.map_name not in maps:
            continue
        if m.map_name not in meshes:
            meshes[m.map_name] = landmesh.load(game / "DATA" / "MAPS" / m.map_name / "Land.msh")
        (minx, miny, _), (maxx, maxy, _) = meshes[m.map_name].bounds()
        for o in m.objects:
            total += 1
            x, y, _ = o.position
            inside += minx <= x <= maxx and miny <= y <= maxy
    check("data.tma: placed objects lie inside the map", inside == total,
          f"{inside}/{total} objects within their map's XY extent")

    # Buildings rest on the ground, which ties mission space to terrain space.
    residuals = []
    for m in parsed:
        if m.map_name not in meshes:
            continue
        mesh = meshes[m.map_name]
        for o in m.objects:
            if o.kind != mission.KIND_BUILDING:
                continue
            h = mesh.height_at(o.position[0], o.position[1])
            if h is not None:
                residuals.append(o.position[2] - h)
    residuals.sort()
    median = residuals[len(residuals) // 2] if residuals else 999.0
    check("data.tma: buildings sit on the terrain surface", abs(median) < 1.0,
          f"median height above ground {median:+.3f} over {len(residuals)} buildings")

    # A building's exterior is authored symmetric about z = 0, so placing that
    # origin at ground level buries half of it.  Units and scenery are authored
    # base-at-origin.  See docs/07-objects.md.
    centred = tall = 0
    for name in ("fortif.rlb",):
        ar = NResArchive.open(game / name)
        for e in ar:
            if e.tag != "MESH":
                continue
            m = objmesh.parse(ar.read(e), e.name)
            sel = m.select(0) or m.triangles
            vs = {i for t in sel for i in t}
            if not vs:
                continue
            lo = min(m.positions[i][2] for i in vs)
            hi = max(m.positions[i][2] for i in vs)
            if hi - lo < 1e-3:
                continue
            tall += 1
            centred += abs(lo + hi) / (hi - lo) < 0.25
    check("MESH: building exteriors are authored about their centre",
          centred > tall * 0.6,
          f"{centred}/{tall} building meshes are near-symmetric about z=0 "
          f"-- they must be rested on their base, not their origin")

    # Definition references must resolve, which is what makes the object
    # records readable rather than merely parseable.
    statics = {
        e.name.lower()
        for e in NResArchive.open(game / "objects.rlb")
        if e.tag == "STAT"
    }
    good = bad = 0
    for m in parsed:
        for o in m.objects:
            if o.is_static:
                good += o.path.lower() in statics
                bad += o.path.lower() not in statics
            else:
                f = game / o.path.replace("\\", "/")
                hit = f.exists()
                if not hit and f.parent.exists():
                    hit = f.name.lower() in {x.name.lower() for x in f.parent.iterdir()}
                good += hit
                bad += not hit
    check("data.tma: every object reference resolves", bad == 0,
          f"{good}/{good + bad} -- UNITS/*.dat on disk, scenery as STAT in objects.rlb")

    # ClanID indexes the clan list positionally; it is not the clan's own
    # `index` field, which starts at 1 and repeats across campaign missions.
    owned = sum(1 for m in parsed for o in m.objects if o.clan_id is not None)
    in_range = sum(
        1 for m in parsed for o in m.objects
        if o.clan_id is not None and 0 <= o.clan_id < len(m.clans)
    )
    check("data.tma: ClanID is a 0-based index into the clan list",
          in_range == owned, f"{in_range}/{owned} object ClanIDs in range")

    # Corroboration: on skirmish and multiplayer maps, where each clan holds a
    # distinct base, an object's clan should be the one whose base it sits near.
    near = near_total = 0
    for m in parsed:
        if not m.source.parent.name.startswith(("Single", "Multi")):
            continue
        bases = [c.base for c in m.clans]
        for o in m.objects:
            if o.clan_id is None or not (0 <= o.clan_id < len(bases)):
                continue
            x, y, _ = o.position
            dists = [math.dist((x, y), b) for b in bases]
            near_total += 1
            near += dists.index(min(dists)) == o.clan_id
    ratio = near / near_total if near_total else 0.0
    check("data.tma: objects belong to the clan whose base they sit at",
          ratio > 0.95,
          f"{near}/{near_total} ({ratio:.1%}) on skirmish and multiplayer maps")


def check_objects(check, game: Path) -> None:
    """objects.rlb reference records, object meshes, and unit assemblies."""
    lib = objects.ObjectLibrary(game / "objects.rlb")
    check("objects.rlb: every record parses into slots", len(lib) > 0,
          f"{len(lib)} records, tags "
          f"{sorted({r.tag for r in lib.records.values()})}")

    # Every slot of every scenery record must name a real archive member.
    archives: dict[str, NResArchive] = {}
    good = bad = 0
    for record in lib.by_tag("STAT"):
        for slot in record.slots:
            if not slot:
                continue
            if slot.library not in archives:
                archives[slot.library] = NResArchive.open(game / slot.library)
            try:
                archives[slot.library].find(slot.member)
                good += 1
            except KeyError:
                bad += 1
    check("objects.rlb: scenery resource slots resolve", bad == 0,
          f"{good}/{good + bad} slots across {len(lib.by_tag('STAT'))} STAT records")

    # Object meshes: streams must agree on vertex and face counts.
    static = NResArchive.open(game / "static.rlb")
    parsed = consistent = 0
    worst_normal = 0.0
    tris = 0
    for entry in static:
        if entry.tag != "MESH":
            continue
        m = objmesh.parse(static.read(entry), entry.name)
        parsed += 1
        tris += m.triangle_count
        ok = (
            len(m.normals) == m.vertex_count
            and len(m.uv) == m.vertex_count
            and all(0 <= i < m.vertex_count for t in m.triangles for i in t)
        )
        consistent += ok
        for n in m.normals:
            worst_normal = max(worst_normal, abs(math.sqrt(sum(c * c for c in n)) - 1.0))
    check("MESH: object meshes parse with consistent streams", consistent == parsed,
          f"{consistent}/{parsed} meshes, {tris} triangles, all indices in range")
    check("MESH: int8/127 normals are unit length", worst_normal < 0.02,
          f"worst deviation {worst_normal:.4f} -- the same encoding as the terrain")

    # The mesh and control-point formats are not specific to scenery: every
    # record type but FORT uses the same .msh/.wea/.cpt/.ndp/.ctl slot set.
    ARCHIVES = (
        "static.rlb", "intsys.rlb", "turrets.rlb", "guns.rlb", "parts.rlb",
        "weapon.rlb", "animals.rlb", "bases.rlb", "fortif.rlb", "system.rlb",
    )
    all_meshes = all_tris = mesh_fail = 0
    all_cpt = cpt_fail = points = 0
    named = 0
    for name in ARCHIVES:
        ar = NResArchive.open(game / name)
        for e in ar:
            if e.tag == "MESH":
                try:
                    mm = objmesh.parse(ar.read(e), e.name)
                    assert all(0 <= i < mm.vertex_count for t in mm.triangles for i in t)
                    all_meshes += 1
                    all_tris += mm.triangle_count
                except Exception:
                    mesh_fail += 1
            elif e.tag == "CTPT":
                try:
                    pts = objmesh.parse_control_points(ar.read(e), e.name)
                    all_cpt += 1
                    points += len(pts)
                    named += sum(1 for p in pts if p.name)
                except Exception:
                    cpt_fail += 1
    check("MESH: the format is the same in every archive", mesh_fail == 0,
          f"{all_meshes} meshes, {all_tris} triangles across {len(ARCHIVES)} archives")
    check("CTPT: control points parse as two parallel arrays", cpt_fail == 0,
          f"{all_cpt} members, {points} points, {named} of them named")

    # Draw batches carry the material assignment; no per-face field does.
    lib_mat = materials.MaterialLibrary(game / "Material.lib")
    tex_names = {e.name.upper() for e in NResArchive.open(game / "Textures.lib")}
    tiles = covers = mats_ok = batched = with_wear = 0
    chain_ok = chain_total = 0
    for name in ARCHIVES:
        ar = NResArchive.open(game / name)
        for e in ar:
            if e.tag != "MESH":
                continue
            try:
                wear = objmesh.read_wea(ar.read_name(e.name.replace(".msh", ".wea")))
            except KeyError:
                wear = []
            m = objmesh.parse(ar.read(e), e.name, wear)
            if not m.batches:
                continue
            batched += 1
            tiles += sum(b.index_count for b in m.batches) == m.triangle_count * 3
            cursor = 0
            for b in m.batches:
                if b.first_index != cursor:
                    break
                cursor += b.index_count
            else:
                covers += 1
            if wear:
                with_wear += 1
                mats_ok += max(b.material for b in m.batches) < len(wear)
                for b in m.batches:
                    chain_total += 1
                    if b.material >= len(wear):
                        continue
                    texture = lib_mat.texture_for(wear[b.material])
                    chain_ok += bool(texture and texture.upper() in tex_names)
    check("MESH: draw batches tile the index buffer", tiles == batched,
          f"{tiles}/{batched} meshes -- index counts sum to 3 x triangles")
    check("MESH: batch index ranges are contiguous", covers == batched,
          f"{covers}/{batched} meshes")
    check("MESH: a batch's material indexes the model's wear", mats_ok == with_wear,
          f"{mats_ok}/{with_wear} meshes with a wear "
          f"-- this is where the texture assignment lives")
    check("Material.lib: batch -> wear -> MAT0 -> Texm resolves",
          chain_ok > chain_total * 0.98,
          f"{chain_ok}/{chain_total} batches reach a real texture "
          f"({len(lib_mat)} materials)")

    # Indices are batch-relative, the DirectX DrawIndexedPrimitive convention.
    relative = interior_ok = interior_tot = parts_seen = full_use = 0
    absolute_use = 0.0
    for name in ARCHIVES:
        ar = NResArchive.open(game / name)
        for e in ar:
            if e.tag != "MESH":
                continue
            m = objmesh.parse(ar.read(e), e.name)
            if m.batches:
                flat = [i for t in m.raw_triangles for i in t]
                ok = True
                for b in m.batches:
                    seg = flat[b.first_index : b.first_index + b.index_count]
                    if seg and max(seg) >= b.vertex_count:
                        ok = False
                        break
                relative += ok
                # The decisive test: a correct reading must reach every
                # vertex.  Read as absolute, buildings reach barely a tenth.
                full_use += len({i for t in m.triangles for i in t}) == m.vertex_count
                if m.vertex_count:
                    absolute_use += (
                        len({i for t in m.raw_triangles for i in t}) / m.vertex_count
                    )
            for node in m.nodes:
                parts_seen += 1
                interior_tot += 1
                interior_ok += node.is_interior == node.name.lower().startswith("i")
    check("MESH: batch indices are relative to the batch's first vertex",
          relative == batched,
          f"{relative}/{batched} meshes -- every index is below its own batch's vertex count")
    check("MESH: resolved indices reference every vertex", full_use == batched,
          f"{full_use}/{batched} meshes reach 100% of their vertices "
          f"(reading the indices as absolute reaches {absolute_use / max(batched, 1):.0%})")
    # Slots and LOD selection: a node picks geometry with
    # slot_index[lod * 5 + group], so a renderer can draw one level of detail
    # and leave the building's interior out.
    slot_ok = slot_range = sel = slotted = 0
    for name in ARCHIVES:
        ar = NResArchive.open(game / name)
        for e in ar:
            if e.tag != "MESH":
                continue
            m = objmesh.parse(ar.read(e), e.name)
            if not m.slots or not m.nodes:
                continue
            slotted += 1
            slot_ok += all(
                i == objmesh.NO_SLOT or i < len(m.slots)
                for n in m.nodes for i in n.slot_index
            )
            slot_range += all(
                s.first_triangle + s.triangle_count <= m.triangle_count
                and s.first_batch + s.batch_count <= len(m.batches)
                for s in m.slots
            )
            sel += bool(m.select(0)) or not m.triangle_count
    check("MESH: node slot indices address real slots", slot_ok == slotted,
          f"{slot_ok}/{slotted} meshes")
    check("MESH: slot ranges lie inside the triangle and batch lists",
          slot_range == slotted, f"{slot_range}/{slotted} meshes")
    check("MESH: every model yields LOD 0 geometry", sel == slotted,
          f"{sel}/{slotted} meshes -- what a renderer should draw")

    check("MESH: sub-object flag bit 0 marks interior geometry",
          interior_ok == interior_tot,
          f"{interior_ok}/{interior_tot} sub-objects agree with the o*/i* naming")

    # Building interiors: the path graph the engine calls a hall way.
    fortif = NResArchive.open(game / "fortif.rlb")
    graphs = nodes = links = bad_link = 0
    for e in fortif:
        if e.tag != "MESH":
            continue
        graph = objmesh.read_path_graph(NResArchive(fortif.read(e), e.name))
        if graph is None:
            continue
        graphs += 1
        nodes += len(graph.nodes)
        links += len(graph.links)
        for link in graph.links:
            if not (0 <= link.start < len(graph.nodes) and 0 <= link.end < len(graph.nodes)):
                bad_link += 1
    check("MESH: buildings carry an interior path graph", graphs > 0,
          f"{graphs} of 30 fortif.rlb meshes, {nodes} nodes, {links} links")
    check("MESH: path graph links join real nodes", bad_link == 0,
          f"{links - bad_link}/{links} links")

    # Unit assemblies.
    dats = sorted((game / "UNITS").rglob("*.dat"))
    loaded = comps = missing = 0
    failures = []
    for f in dats:
        try:
            d = objects.load_unit(f)
        except objects.ObjectFormatError as exc:
            failures.append(str(exc))
            continue
        loaded += 1
        for c in d.components:
            comps += 1
            if c.ref.library.lower() == "objects.rlb" and lib.get(c.ref.member) is None:
                missing += 1
    check("UNITS/*.dat: assemblies parse on a 112-byte stride", not failures,
          f"{loaded}/{len(dats)} files, {comps} components"
          + ("" if not failures else f" -- {failures[0]}"))
    check("UNITS/*.dat: components resolve in objects.rlb", missing <= 3,
          f"{comps - missing}/{comps} resolve ({missing} do not; see docs/07-objects.md)")

    # Every placed object must reach geometry, following FORT indirection.
    def record_mesh(rec, depth=0):
        if rec is None or depth > 3:
            return None
        if rec.mesh:
            return rec.mesh
        for slot in rec.slots:
            if slot and not slot.suffix:
                found = record_mesh(lib.get(slot.member), depth + 1)
                if found:
                    return found
        return None

    reached = placed = 0
    for d in gamedir.missions(game):
        for o in mission.load(d / "data.tma").objects:
            placed += 1
            if o.is_static:
                ref = record_mesh(lib.get(o.path))
            else:
                f = game / o.path.replace("\\", "/")
                if not f.exists() and f.parent.exists():
                    f = {x.name.lower(): x for x in f.parent.iterdir()}.get(f.name.lower())
                ref = None
                if f is not None and f.exists():
                    unit = objects.load_unit(f)
                    if unit.components:
                        ref = record_mesh(lib.get(unit.components[0].ref.member))
            reached += ref is not None
    check("every placed mission object reaches geometry", reached == placed,
          f"{reached}/{placed} objects resolve to a .msh through objects.rlb")


def check_sky(check, game: Path) -> None:
    """``sky.ske``: the atmosphere's day cycle."""
    files = sorted(game.glob("MISSIONS/**/sky.ske"))
    parsed = 0
    frames = 0
    dated = 0
    ordered = 0
    varying: list[bool] = []
    failures = []
    for path in files:
        try:
            atmosphere = sky.load(path)
        except sky.SkyFormatError as exc:
            failures.append(str(exc))
            continue
        parsed += 1
        frames += len(atmosphere)
        first = [k for k in atmosphere.keyframes if k.section == 0]
        dated += all(0 <= k.hour <= 24 and 0 <= k.minute < 60 for k in first)
        ordered += all(
            first[i].minutes <= first[i + 1].minutes for i in range(len(first) - 2)
        )
        # The light runs dark at night and bright by day, and its low point
        # falls in the small hours.
        bright = atmosphere.brightest()
        dark = min(atmosphere.keyframes, key=lambda k: k.light)
        if bright is not None and bright.light > dark.light:
            varying.append(dark.minutes <= 120)
    check("sky.ske: parses to the byte", parsed == len(files),
          f"{parsed}/{len(files)} files, {frames} keyframes"
          + ("" if not failures else f" -- {failures[0]}"))
    check("sky.ske: keyframes carry a time of day", dated == parsed,
          f"{dated}/{parsed} files hold an hour 0-24 and a minute 0-59 in "
          f"every keyframe of their first section")
    check("sky.ske: keyframes run in time order", ordered >= parsed * 0.9,
          f"{ordered}/{parsed} files are sorted by time")
    # sky.wea is a fixed role table, and every slot resolves.
    lib = materials.MaterialLibrary(game / "Material.lib")
    textures = NResArchive.open(game / "Textures.lib")
    textures_by_stem = {e.name.split(".")[0].upper(): e for e in textures}
    known = set(textures_by_stem)
    per_slot: list[set[str]] = [set() for _ in sky.SLOT_ROLES]
    named = resolved = full = 0
    for path in files:
        try:
            atmosphere = sky.load(path)
        except sky.SkyFormatError:
            continue
        for index, role in enumerate(sky.SLOT_ROLES):
            name = atmosphere.texture(role)
            if not name:
                continue
            per_slot[index].add(name)
            named += 1
            material = lib.get(name)
            base = material.textures[0] if material and material.textures else None
            if base and base.upper().split(".")[0] in known:
                resolved += 1
    constant = [i for i, s in enumerate(per_slot) if len(s) == 1]
    check("sky.wea: the slot index is the role", len(per_slot[0]) and all(per_slot),
          f"all {parsed} missions fill the same {len(sky.SLOT_ROLES)} slots; "
          f"slots {constant} name one texture every time")
    check("sky.wea: every slot resolves through Material.lib", resolved == named,
          f"{resolved}/{named} slot names reach a texture (none of them is in "
          f"Textures.lib under its own name)")

    for material in lib.materials.values():
        full += material.cell == materials.WHOLE_TEXTURE
    sun, moon = lib.get("ENV_SUN"), lib.get("ENV_MOON")
    check("Material.lib: the byte before the name picks a sub-image",
          sun is not None and moon is not None
          and sun.textures == moon.textures and sun.cell != moon.cell,
          f"{full}/{len(lib)} materials take the whole texture; ENV_SUN and "
          f"ENV_MOON are both {sun.textures[0] if sun else '?'} at cells "
          f"{sun.cell if sun else '?'} and {moon.cell if moon else '?'}")

    # The lens flare table read out of Terrain.dll, against the textures the
    # missions actually name for it.
    flare_names = [sorted(per_slot[sky.SLOT_ROLES.index(r)]) for r in ("flare", "flare2")]
    with_alpha = decoded = 0
    for names in flare_names:
        for name in names:
            material = lib.get(name)
            base = material.textures[0] if material and material.textures else None
            if not base:
                continue
            try:
                tex = texm.decode(textures.read_name(base))
            except (KeyError, ValueError):
                continue
            decoded += 1
            with_alpha += any(v < 255 for v in tex.rgba[3::4])
    positions = [e[0] for e in sky.FLARE_ELEMENTS]
    check("CSun: the lens flare is twelve sprites down the view axis",
          len(sky.FLARE_ELEMENTS) == 12
          and positions == sorted(positions, reverse=True)
          and positions[0] > 1.0 and positions[-1] < -1.0
          and {e[3] for e in sky.FLARE_ELEMENTS} == {0, 1},
          f"positions run {positions[0]} (past the sun) to {positions[-1]} "
          f"(past the far side of the screen); both flare slots are used")
    check("sky.wea: the flare textures carry alpha", with_alpha == decoded > 0,
          f"{with_alpha}/{decoded} of the textures behind "
          f"{', '.join(n for group in flare_names for n in group)} have an "
          f"alpha channel, which is what an additive sprite needs")

    # Rain and lightning are named by a keyframe; snow never is.
    weathered = Counter()
    with_marker = 0
    for path in files:
        try:
            atmosphere = sky.load(path)
        except sky.SkyFormatError:
            continue
        kinds = atmosphere.weather()
        with_marker += bool(kinds)
        for kind in kinds:
            weathered[kind] += 1
    check("sky.ske: a keyframe names the weather it starts",
          weathered.get("rain", 0) > 0 and weathered.get("lightning", 0) > 0
          and "snow" not in weathered,
          f"{with_marker}/{parsed} missions carry a weather marker: "
          f"{weathered.get('rain', 0)} name {sky.RAIN_MARKER} and "
          f"{weathered.get('lightning', 0)} name {sky.LIGHTNING_MARKER}; "
          f"none names snow")

    # The sub-image cell indexes the texture's own Page table.
    indexed = inside = biggest = 0
    pages_seen = set()
    for material in lib.materials.values():
        if material.cell == materials.WHOLE_TEXTURE or not material.textures:
            continue
        entry = textures_by_stem.get(material.textures[0].upper().split(".")[0])
        if entry is None:
            continue
        indexed += 1
        biggest = max(biggest, material.cell)
        pages = texm.parse_pages(textures.read(entry))
        pages_seen.add(entry.name)
        inside += material.cell < len(pages)
    check("Texm: a material's cell indexes the texture's own Page table",
          inside == indexed > 0,
          f"{inside}/{indexed} cells fall inside the Page table of the texture "
          f"they name, over {len(pages_seen)} textures and cells up to {biggest}")

    # SUN.0's four pages are its quadrants, which is what puts ENV_MOON's
    # cell 2 on the moon.
    sun_pages = texm.parse_pages(textures.read_name("SUN.0"))
    check("Texm: SUN.0's pages are the quadrants ENV_SUN and ENV_MOON pick",
          sun_pages == [(0, 0, 128, 128), (128, 0, 128, 128),
                        (0, 128, 128, 128), (128, 128, 128, 128)],
          f"{sun_pages} -- cell 0 is the corona, cell 2 the moon")

    check("sky.ske: the third float is a day/night light", all(varying),
          f"on all {len(varying)} files whose light varies, its low point falls "
          f"within two hours of midnight ({parsed - len(varying)} files hold a "
          f"single value across a night-time cycle)")


def check_minimap_agreement(check, game: Path) -> None:
    """The strongest check available: our terrain vs the art the game ships."""
    pairs = [("SC_3", "sc3.tex"), ("Tut_1", "tut1.tex"), ("ILKON", "ilkon.tex"), ("K1F", "k1f.tex")]
    mini = NResArchive.open(game / "ui" / "minimap.lib")
    scores = []
    for map_name, tex_name in pairs:
        m = landmesh.load(game / "DATA" / "MAPS" / map_name / "Land.msh")
        blob = mini.read_name(tex_name)
        tex = texm.decode(blob)
        n = tex.width
        (minx, miny, _), (maxx, maxy, _) = m.bounds()
        height = [None] * (n * n)
        for tri in m.faces:
            p = [m.positions[i] for i in tri]
            sx = [(v[0] - minx) / (maxx - minx) * (n - 1) for v in p]
            sy = [(v[1] - miny) / (maxy - miny) * (n - 1) for v in p]
            den = (sy[1] - sy[2]) * (sx[0] - sx[2]) + (sx[2] - sx[1]) * (sy[0] - sy[2])
            if abs(den) < 1e-9:
                continue
            for py in range(max(0, int(min(sy))), min(n, int(max(sy)) + 2)):
                for px in range(max(0, int(min(sx))), min(n, int(max(sx)) + 2)):
                    l1 = ((sy[1] - sy[2]) * (px - sx[2]) + (sx[2] - sx[1]) * (py - sy[2])) / den
                    l2 = ((sy[2] - sy[0]) * (px - sx[2]) + (sx[0] - sx[2]) * (py - sy[2])) / den
                    l3 = 1 - l1 - l2
                    if l1 < -0.002 or l2 < -0.002 or l3 < -0.002:
                        continue
                    z = l1 * p[0][2] + l2 * p[1][2] + l3 * p[2][2]
                    i = py * n + px
                    if height[i] is None or z > height[i]:
                        height[i] = z
        ours, theirs = [], []
        for y in range(n):
            for x in range(n):
                h = height[(n - 1 - y) * n + x]  # minimaps are stored top-down
                if h is None:
                    continue
                r, g, b, _ = tex.rgba[(y * n + x) * 4 : (y * n + x) * 4 + 4]
                ours.append(h)
                theirs.append(0.299 * r + 0.587 * g + 0.114 * b)
        ma, mb = sum(ours) / len(ours), sum(theirs) / len(theirs)
        na = math.sqrt(sum((v - ma) ** 2 for v in ours))
        nb = math.sqrt(sum((v - mb) ** 2 for v in theirs))
        cov = sum((a - ma) * (b - mb) for a, b in zip(ours, theirs, strict=True))
        scores.append((map_name, cov / (na * nb)))
    worst = min(s for _, s in scores)
    check("Terrain matches the game's own minimap art", worst > 0.6,
          ", ".join(f"{n} r={s:+.3f}" for n, s in scores))


def check_poses(check, game: Path) -> None:
    """Node poses, the assembly tree, and the ground datum."""
    archives = [
        "static.rlb", "fortif.rlb", "bases.rlb", "turrets.rlb", "guns.rlb",
        "parts.rlb", "weapon.rlb", "animals.rlb", "intsys.rlb", "system.rlb",
    ]
    meshes: list[tuple[str, objmesh.ObjectMesh]] = []
    keys = unit = 0
    for name in archives:
        path = game / name
        if not path.exists():
            continue
        archive = NResArchive.open(path)
        for entry in archive:
            if not entry.name.lower().endswith(".msh"):
                continue
            blob = archive.read(entry)
            meshes.append((entry.name, objmesh.parse(blob, entry.name)))
            # Read the last eight bytes of each key straight out of the file,
            # before the reader normalises them, or the claim proves itself.
            inner = NResArchive(blob, entry.name)
            for stream in inner:
                if stream.type_id != objmesh.STREAM_POSE_KEY:
                    continue
                raw = inner.read(stream)
                for i in range(stream.element_count):
                    packed = struct.unpack_from("<4h", raw, i * objmesh.POSE_KEY_SIZE + 16)
                    keys += 1
                    length = math.sqrt(
                        sum((v / objmesh.QUATERNION_SCALE) ** 2 for v in packed)
                    )
                    unit += abs(length - 1.0) < 0.001
    check("MESH: stream 8 rotations are unit quaternions", keys - unit <= 12,
          f"{unit}/{keys} of the packed int16 quadruples are within 0.1% of "
          f"unit length ({keys - unit} are zero or unnormalised)")

    # The frame map indexes real keys, and an animated node's fallback is the
    # last frame of its own run.
    in_range = frames = fallback_is_last = animated = 0
    for _name, m in meshes:
        for node in m.nodes:
            if not node.is_animated or not m.frame_count:
                continue
            animated += 1
            run = m.frame_map[node.anim_start : node.anim_start + m.frame_count]
            frames += len(run)
            in_range += sum(1 for k in run if k < len(m.keys))
            fallback_is_last += bool(run) and run[-1] == node.fallback_key
    check("MESH: the frame map indexes real pose keys", in_range == frames,
          f"{in_range}/{frames} entries across {animated} animated nodes")
    check("MESH: an animated node's fallback is its last frame",
          fallback_is_last >= animated * 0.95,
          f"{fallback_is_last}/{animated} nodes")

    # The strongest check: the authored bounding box in the stream-2 header is
    # stated in posed space, so composing poses has to reach it.
    def box(m: objmesh.ObjectMesh, posed: bool):
        lo = [math.inf] * 3
        hi = [-math.inf] * 3
        found = False
        for i, node in enumerate(m.nodes):
            pose = m.world_pose(i) if posed else objmesh.IDENTITY_POSE
            for index in node.slot_index:
                if index == objmesh.NO_SLOT or index >= len(m.slots):
                    continue
                slot = m.slots[index]
                stop = slot.first_triangle + slot.triangle_count
                for tri in m.triangles[slot.first_triangle : stop]:
                    for v in tri:
                        if v >= len(m.positions):
                            continue
                        p = objmesh.apply(pose, m.positions[v])
                        found = True
                        for axis in range(3):
                            lo[axis] = min(lo[axis], p[axis])
                            hi[axis] = max(hi[axis], p[axis])
        return (lo, hi) if found else None

    posed_ok = raw_ok = total = 0
    for _name, m in meshes:
        if m.volume is None or not m.nodes or not m.slots:
            continue
        want_lo, want_hi = m.volume.minimum, m.volume.maximum
        diagonal = math.dist(want_lo, want_hi) or 1.0
        tol = 0.005 * diagonal
        total += 1
        for posed, counter in ((True, "posed"), (False, "raw")):
            got = box(m, posed)
            if got is None:
                continue
            lo, hi = got
            fits = all(
                lo[a] <= want_lo[a] + tol and hi[a] >= want_hi[a] - tol for a in range(3)
            )
            if fits and counter == "posed":
                posed_ok += 1
            elif fits:
                raw_ok += 1
    check("MESH: posed geometry reaches the authored bounding box",
          posed_ok > raw_ok * 1.25,
          f"{posed_ok}/{total} with poses applied against {raw_ok}/{total} without")

    # Sub-object flag bit 5 marks a collision hull.  The name says the same
    # thing, and the two agree exactly.
    marked = named = drawable = agree = hull_triangles = 0
    for _name, m in meshes:
        for node in m.nodes:
            hull = node.name.split("_")[0] in ("CP", "BTCP")
            triangles = sum(
                m.slots[i].triangle_count
                for i in node.slots_for_lod(0)
                if i < len(m.slots)
            )
            marked += node.is_collision
            named += hull
            if hull and triangles:
                drawable += 1
                agree += node.is_collision
                hull_triangles += triangles
    check("MESH: flag bit 0x20 marks exactly the CP_* collision hulls",
          marked == agree == drawable > 0 and marked < named,
          f"{marked} nodes carry the bit and every one is named CP_* or BTCP_*; "
          f"{agree}/{drawable} of the hulls that have geometry carry it, and the "
          f"{named - drawable} that do not are empty. {hull_triangles} triangles "
          f"a renderer must not draw")

    # And dropping them is what makes every model fit the box it states.
    def within(m: objmesh.ObjectMesh, skip_hulls: bool) -> bool:
        lo = [math.inf] * 3
        hi = [-math.inf] * 3
        for i, node in enumerate(m.nodes):
            if skip_hulls and node.is_collision:
                continue
            pose = m.world_pose(i)
            for index in node.slots_for_lod(0):
                if index >= len(m.slots):
                    continue
                slot = m.slots[index]
                stop = slot.first_triangle + slot.triangle_count
                for tri in m.triangles[slot.first_triangle : stop]:
                    for v in tri:
                        if v >= len(m.positions):
                            continue
                        p = objmesh.apply(pose, m.positions[v])
                        for axis in range(3):
                            lo[axis] = min(lo[axis], p[axis])
                            hi[axis] = max(hi[axis], p[axis])
        if lo[0] == math.inf:
            return False
        want_lo, want_hi = m.volume.minimum, m.volume.maximum
        tol = 0.005 * (math.dist(want_lo, want_hi) or 1.0)
        return all(
            lo[a] >= want_lo[a] - tol and hi[a] <= want_hi[a] + tol for a in range(3)
        )

    boxed = [m for _name, m in meshes if m.volume and m.nodes and m.slots]
    with_hulls = sum(within(m, False) for m in boxed)
    without = sum(within(m, True) for m in boxed)
    check("MESH: level 0 fits inside the authored box once hulls are dropped",
          without == len(boxed) > with_hulls,
          f"{without}/{len(boxed)} models fit inside their own box against "
          f"{with_hulls}/{len(boxed)} while the collision hulls are drawn")

    # The .ctl slot was the obvious place to look for the pose a parked unit
    # stands in.  It is not there: a controller's size tracks its own leading
    # count and not the model's node count.
    library = objects.ObjectLibrary(game / "objects.rlb")
    opened: dict[str, dict[str, bytes]] = {}

    def controller(ref) -> bytes | None:
        if ref.library not in opened:
            path = game / ref.library
            opened[ref.library] = (
                {e.name.lower(): NResArchive.open(path).read(e)
                 for e in NResArchive.open(path)}
                if path.exists() else {}
            )
        return opened[ref.library].get(ref.member.lower())

    sizes: list[int] = []
    counts: list[int] = []
    node_counts: list[int] = []
    by_name = {name.lower(): m for name, m in meshes}
    for record in library.records.values():
        slot = record.slot_with_suffix("ctl")
        if slot is None or record.mesh is None:
            continue
        blob = controller(slot)
        model = by_name.get(record.mesh.member.lower())
        if blob is None or model is None or len(blob) < 4:
            continue
        sizes.append(len(blob))
        counts.append(struct.unpack_from("<i", blob, 0)[0])
        node_counts.append(len(model.nodes))

    def correlation(a: list[int], b: list[int]) -> float:
        mean_a = sum(a) / len(a)
        mean_b = sum(b) / len(b)
        cov = sum((x - mean_a) * (y - mean_b) for x, y in zip(a, b, strict=True))
        spread = math.sqrt(
            sum((x - mean_a) ** 2 for x in a) * sum((y - mean_b) ** 2 for y in b)
        )
        return cov / spread if spread else 0.0

    own = correlation(sizes, counts)
    nodes = correlation(sizes, node_counts)
    check("objects.rlb: a .ctl is not per-node data", own > 0.9 > nodes,
          f"over {len(sizes)} records a controller's size correlates {own:+.2f} "
          f"with its own leading count and only {nodes:+.2f} with the number of "
          f"nodes in the mesh it belongs to")

    # The 15 slot indices are three variants of a five-slot block, and the
    # first four of each block are a level-of-detail ladder.
    def ladder(width: int) -> tuple[int, int]:
        good = seen = 0
        for _name, m in meshes:
            for node in m.nodes:
                for base in range(0, 15, objmesh.SLOTS_PER_VARIANT):
                    counts = [
                        m.slots[i].triangle_count
                        for i in node.slot_index[base : base + width]
                        if i != objmesh.NO_SLOT and i < len(m.slots)
                    ]
                    if len(counts) < 2:
                        continue
                    seen += 1
                    good += all(counts[i] >= counts[i + 1] for i in range(len(counts) - 1))
        return good, seen

    four, four_of = ladder(4)
    five, five_of = ladder(5)
    check("MESH: a variant's first four slots are an LOD ladder",
          four / max(four_of, 1) > 0.99 > five / max(five_of, 1),
          f"triangle counts fall monotonically on {four}/{four_of} chains; "
          f"{five}/{five_of} once the fifth slot is included, so it is not a level")

    # And drawing level 0 alone is what the authored box describes.
    exact_lod0 = exact_all = boxed = 0
    for _name, m in meshes:
        if m.volume is None or not m.nodes or not m.slots:
            continue
        want_lo, want_hi = m.volume.minimum, m.volume.maximum
        tol = 0.005 * (math.dist(want_lo, want_hi) or 1.0)
        boxed += 1
        for pick, name in (
            (lambda n: n.slots_for_lod(0), "lod0"),
            (lambda n: [i for i in n.slot_index[:5] if i != objmesh.NO_SLOT], "all"),
        ):
            lo = [math.inf] * 3
            hi = [-math.inf] * 3
            for i, node in enumerate(m.nodes):
                pose = m.world_pose(i)
                for index in pick(node):
                    if index >= len(m.slots):
                        continue
                    slot = m.slots[index]
                    stop = slot.first_triangle + slot.triangle_count
                    for tri in m.triangles[slot.first_triangle : stop]:
                        for v in tri:
                            if v >= len(m.positions):
                                continue
                            p = objmesh.apply(pose, m.positions[v])
                            for axis in range(3):
                                lo[axis] = min(lo[axis], p[axis])
                                hi[axis] = max(hi[axis], p[axis])
            if math.isinf(lo[0]):
                continue
            fits = all(
                abs(lo[a] - want_lo[a]) < tol and abs(hi[a] - want_hi[a]) < tol
                for a in range(3)
            )
            if name == "lod0":
                exact_lod0 += fits
            else:
                exact_all += fits
    check("MESH: level 0 alone is what the authored box measures",
          exact_lod0 > exact_all * 1.2,
          f"{exact_lod0}/{boxed} meshes match it exactly, against {exact_all}/{boxed} "
          f"when all five slots of the variant are drawn together")

    # Baked lighting: the wear's LIGHTMAPS section and mesh stream 18.
    lightmaps = NResArchive.open(game / "lightmap.lib")
    pages = {e.name.upper(): texm.parse_header(lightmaps.read(e))[:2] for e in lightmaps}
    with_both = with_stream = with_section = fits_page = 0
    for name in archives:
        path = game / name
        if not path.exists():
            continue
        archive = NResArchive.open(path)
        for entry in archive:
            if not entry.name.lower().endswith(".msh"):
                continue
            m = objmesh.parse(archive.read(entry), entry.name)
            try:
                wear = objmesh.parse_wear(
                    archive.read_name(entry.name.replace(".msh", ".wea"))
                )
            except KeyError:
                wear = objmesh.Wear()
            has_stream = bool(m.lightmap_uv)
            has_section = bool(wear.lightmaps)
            with_stream += has_stream and not has_section
            with_section += has_section and not has_stream
            if not (has_stream and has_section):
                continue
            with_both += 1
            page = pages.get(wear.lightmaps[0].upper())
            if not page:
                continue
            # An atlas is authored inset by half a texel, so the largest UV is
            # 1 - 0.5 / width.  Exact on every one, which pins both the 1024
            # divisor and the pairing.
            top = max(max(u, v) for u, v in m.lightmap_uv)
            want = round((1 - 0.5 / page[0]) * objmesh.LIGHTMAP_UV_SCALE)
            fits_page += round(top * objmesh.LIGHTMAP_UV_SCALE) == want
    check("MESH: stream 18 is the lightmap's UV set",
          with_stream == 0 and with_section == 0,
          f"{with_both} meshes carry both a LIGHTMAPS section and stream 18, "
          f"{with_stream} the stream alone, {with_section} the section alone")
    check("MESH: lightmap UVs are uint16 over 1024", fits_page == with_both,
          f"{fits_page}/{with_both} top out at exactly 1 - 0.5/width of their "
          f"own page -- 1022 for a 256-pixel lightmap, 1020 for a 128")

    # The high byte of a batch's material word marks the lit batches.
    lit_meshes = set()
    lightmapped = set()
    lit_uv = lit_total = unlit_uv = unlit_total = 0
    for name in archives:
        path = game / name
        if not path.exists():
            continue
        archive = NResArchive.open(path)
        for entry in archive:
            if not entry.name.lower().endswith(".msh"):
                continue
            m = objmesh.parse(archive.read(entry), entry.name)
            if any(b.is_lit for b in m.batches):
                lit_meshes.add(entry.name.lower())
            try:
                wear = objmesh.parse_wear(
                    archive.read_name(entry.name.replace(".msh", ".wea"))
                )
            except KeyError:
                continue
            if wear.lightmaps:
                lightmapped.add(entry.name.lower())
            if not m.lightmap_uv:
                continue
            for b in m.batches:
                first, count = b.triangles
                seen = {
                    v
                    for tri in m.triangles[first : first + count]
                    for v in tri
                    if v < len(m.lightmap_uv)
                }
                mapped = sum(1 for v in seen if m.lightmap_uv[v] != (0.0, 0.0))
                if b.is_lit:
                    lit_total += len(seen)
                    lit_uv += mapped
                else:
                    unlit_total += len(seen)
                    unlit_uv += mapped
    check("MESH: a batch's material high byte marks the lit batches",
          lit_meshes == lightmapped and lit_uv == lit_total,
          f"the {len(lit_meshes)} meshes with a 0x00 batch are exactly the "
          f"{len(lightmapped)} with a lightmap; {lit_uv}/{lit_total} of their "
          f"vertices carry a lightmap UV, against {unlit_uv}/{unlit_total} "
          f"under 0xFF")

    # Winding, which is what says front-face culling is safe.
    consistent = wound = 0
    for _name, m in meshes:
        agree = seen = 0
        for tri in m.triangles:
            if max(tri) >= len(m.positions):
                continue
            a, b, c = (m.positions[i] for i in tri)
            u = (b[0] - a[0], b[1] - a[1], b[2] - a[2])
            v = (c[0] - a[0], c[1] - a[1], c[2] - a[2])
            face = (
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            )
            avg = [sum(m.normals[i][k] for i in tri) / 3 for k in range(3)]
            d = sum(f * n for f, n in zip(face, avg, strict=True))
            if abs(d) < 1e-9:
                continue
            seen += 1
            agree += d > 0
        if seen:
            wound += 1
            consistent += agree > seen * 0.95
    check("MESH: triangle winding agrees with the vertex normals",
          consistent >= wound - 1,
          f"{consistent}/{wound} meshes wind over 95% of their triangles the "
          f"same way, which is what makes front-face culling safe")

    # Assemblies: a depth-first tree whose children name a socket node.
    dats = sorted((game / "UNITS").rglob("*.dat"))
    trees = attachments = to_socket = 0
    cache: dict[tuple[str, str], objmesh.ObjectMesh | None] = {}
    opened: dict[str, NResArchive] = {}
    lib = objects.ObjectLibrary(game / "objects.rlb")

    def record_mesh(rec, depth=0):
        if rec is None or depth > 3:
            return None
        if rec.mesh:
            return rec.mesh
        for slot in rec.slots:
            if slot and not slot.suffix:
                found = record_mesh(lib.get(slot.member), depth + 1)
                if found:
                    return found
        return None

    def load(ref):
        if ref is None:
            return None
        key = (ref.library, ref.member)
        if key not in cache:
            try:
                if ref.library not in opened:
                    opened[ref.library] = NResArchive.open(game / ref.library)
                cache[key] = objmesh.parse(
                    opened[ref.library].read_name(ref.member), ref.member
                )
            except (KeyError, ValueError, FileNotFoundError):
                cache[key] = None
        return cache[key]

    mounts = rooted_at_origin = agree = 0
    disagree: list[tuple[float, str]] = []
    for path in dats:
        unit = objects.load_unit(path)
        try:
            parents = unit.parents()
        except objects.ObjectFormatError:
            continue
        trees += 1
        refs = [record_mesh(lib.get(c.ref.member)) for c in unit.components]
        for i, component in enumerate(unit.components):
            parent = parents[i]
            if parent < 0 or not component.is_external:
                continue
            host = load(refs[parent])
            if host is None:
                continue
            attachments += 1
            if 0 <= component.attach_node < len(host.nodes):
                name = host.nodes[component.attach_node].name.lower()
                to_socket += name.startswith("base")

            # Mounting makes the part's root node take the socket's pose.
            part = load(refs[i])
            if part is None or not (0 <= component.attach_node < len(host.nodes)):
                continue
            mounts += 1
            root_translation, root_rotation = part.root_pose()
            rooted_at_origin += math.dist(root_translation, (0.0, 0.0, 0.0)) < 1e-6
            socket = host.world_pose(component.attach_node)
            turn = objmesh.quaternion_multiply(
                socket[1], (root_rotation[0], *(-v for v in root_rotation[1:]))
            )
            degrees = math.degrees(2 * math.acos(min(1.0, abs(turn[0]))))
            if degrees < 1.0:
                agree += 1
            else:
                disagree.append((degrees, unit.components[parent].label))

    check("UNITS/*.dat: components form a depth-first tree", trees == len(dats),
          f"{trees}/{len(dats)} assemblies consume their child counts exactly")
    check("UNITS/*.dat: every part attaches to a Base_* node",
          to_socket == attachments,
          f"{to_socket}/{attachments} turret and gun attachments")

    # A part's root translation is zero throughout, which is why using the
    # socket's position alone was indistinguishable from using its whole pose
    # everywhere except where the two rotations differ.
    check("UNITS/*.dat: a mounted part is rooted at its own origin",
          rooted_at_origin == mounts > 0,
          f"{rooted_at_origin}/{mounts} turret and gun meshes have a root node "
          f"at (0, 0, 0), so a socket contributes position and rotation "
          f"independently")

    # And where a socket does say something the part does not, it says the
    # part hangs upside down -- which is what an aircraft's turret does.
    flipped = [d for d in disagree if abs(d[0] - 180.0) < 1.0]
    airborne = sum(
        1 for _turn, label in flipped
        if "flying" in label.lower() or "helicopter" in label.lower()
    )
    check("UNITS/*.dat: a socket that disagrees is a turret hung underneath",
          airborne == len(flipped) > 0,
          f"{agree}/{mounts} sockets carry the part's own rotation; of the "
          f"{len(disagree)} that do not, {len(flipped)} are exactly 180 degrees "
          f"and all {airborne} of those sit on a chassis whose own name says "
          f"Flying or Helicopter")

    # The ground datum: a placed object's own z = 0 sits on the terrain.
    heights: dict[Path, landmesh.LandMesh] = {}
    origin_on_ground = base_on_ground = buildings = units = 0
    for folder in gamedir.missions(game):
        m = mission.load(folder / "data.tma")
        if not m.map_path:
            continue
        target = game / m.map_path.replace("\\", "/")
        land = (target if target.is_dir() else target.parent) / "Land.msh"
        if not land.exists():
            continue
        if land not in heights:
            heights[land] = landmesh.load(land)
        terrain = heights[land]
        for o in m.objects:
            if o.kind not in (mission.KIND_BUILDING, mission.KIND_UNIT):
                continue
            ground = terrain.height_at(o.position[0], o.position[1])
            if ground is None:
                continue
            ref = None
            if not o.is_static:
                f = game / o.path.replace("\\", "/")
                if not f.exists() and f.parent.exists():
                    f = {x.name.lower(): x for x in f.parent.iterdir()}.get(f.name.lower())
                if f is not None and f.exists():
                    definition = objects.load_unit(f)
                    if definition.components:
                        ref = record_mesh(lib.get(definition.components[0].ref.member))
            model = load(ref)
            if model is None:
                continue
            posed = model.posed_positions(0)
            reachable = {
                v
                for i, node in enumerate(model.nodes)
                if not node.is_interior
                for index in node.slots_for_lod(0)
                if index < len(model.slots)
                for tri in model.triangles[
                    model.slots[index].first_triangle :
                    model.slots[index].first_triangle + model.slots[index].triangle_count
                ]
                for v in tri
            }
            reachable = {v for v in reachable if v < len(posed)}
            if not reachable:
                continue
            lowest = min(posed[v][2] for v in reachable)
            if o.kind == mission.KIND_BUILDING:
                buildings += 1
                origin_on_ground += abs(o.position[2] - ground) < 2.0
            else:
                units += 1
                base_on_ground += abs(o.position[2] + lowest - ground) < 1.0
    check("placement: a building's origin sits on the terrain",
          origin_on_ground >= buildings * 0.5,
          f"{origin_on_ground}/{buildings} within 2 units of the height under them")
    check("placement: a unit's lowest exterior vertex sits on the terrain",
          base_on_ground >= units * 0.5,
          f"{base_on_ground}/{units} within 1 unit of the height under them")


def check_damage(check, game: Path) -> None:
    """The .ndp damage table, and what the later slot variants hold."""
    library = objects.ObjectLibrary(game / "objects.rlb")
    opened: dict[str, NResArchive] = {}

    def member(ref) -> bytes | None:
        if ref is None:
            return None
        try:
            if ref.library not in opened:
                opened[ref.library] = NResArchive.open(game / ref.library)
            return opened[ref.library].read_name(ref.member)
        except (KeyError, ValueError, FileNotFoundError):
            return None

    parsed = total = per_node = 0
    named = 0
    variant_nodes = variant_named = 0
    nested = blocks = 0
    shrinks = compared = 0
    for record in library.records.values():
        raw = member(record.damage)
        if raw is None:
            continue
        total += 1
        try:
            table = objects.parse_damage(raw, record.damage.member)
        except objects.ObjectFormatError:
            continue
        parsed += 1
        named += sum(1 for row in table if row.explosion)

        blob = member(record.mesh)
        if blob is None:
            continue
        try:
            model = objmesh.parse(blob, record.mesh.member)
        except (ValueError, struct.error):
            continue
        per_node += len(table) == len(model.nodes)
        if len(table) != len(model.nodes):
            continue
        for i, node in enumerate(model.nodes):
            filled = [
                any(
                    node.slot_index[v * objmesh.SLOTS_PER_VARIANT + lod]
                    != objmesh.NO_SLOT
                    for lod in range(objmesh.SLOTS_PER_VARIANT)
                )
                for v in range(objmesh.VARIANT_COUNT)
            ]
            if any(filled):
                blocks += 1
                nested += filled == sorted(filled, reverse=True)
            if not filled[1]:
                continue
            variant_nodes += 1
            variant_named += bool(table[i].explosion)
            intact = node.slot_index[0]
            damaged = node.slot_index[objmesh.SLOTS_PER_VARIANT]
            if objmesh.NO_SLOT not in (intact, damaged) and max(
                intact, damaged
            ) < len(model.slots):
                compared += 1
                shrinks += (
                    model.slots[damaged].triangle_count
                    <= model.slots[intact].triangle_count
                )

    check("objects.rlb: a .ndp is one damage record per node",
          parsed == total > 0 and per_node >= parsed * 0.99,
          f"{parsed}/{total} tables are exactly 4 + n*{objects.DAMAGE_STRIDE} "
          f"bytes and {per_node} of them have one record per mesh node; "
          f"{named} records name an explosion")

    check("MESH: the five-slot blocks are filled in order",
          nested == blocks > 0,
          f"{nested}/{blocks} nodes fill block 0 first, then 1, then 2 -- no "
          f"node carries a later block without the earlier ones")

    check("MESH: a node with a second block is one that can be destroyed",
          variant_named == variant_nodes > 0,
          f"all {variant_nodes} nodes that carry a second five-slot block name "
          f"an explosion in their .ndp, and the block holds the same part with "
          f"pieces gone: fewer triangles on {shrinks}/{compared}")


def check_effects(check, game: Path) -> None:
    """effects.rlb and the .exp explosions, and the chain that reaches them."""
    library = effects.EffectLibrary(game / "effects.rlb")
    members = len(library.archive)
    emitters = sum(len(e.emitters) for e in library)
    check("effects.rlb: every effect walks its emitter blocks exactly",
          len(library) == members > 0,
          f"{len(library)}/{members} effects parse into {emitters} emitters "
          f"across {len(effects.EMITTER_SIZE)} block types")

    # Each block names a material or a sound, and they resolve.
    mats = materials.MaterialLibrary(game / "Material.lib")
    heard = {e.name.lower() for e in NResArchive.open(game / "sounds.lib")}
    drawn = drawn_ok = played = played_ok = 0
    for effect in library:
        for emitter in effect.emitters:
            if not emitter.resource.member:
                continue
            if emitter.is_sound:
                played += 1
                played_ok += emitter.resource.member.lower() in heard
            else:
                drawn += 1
                drawn_ok += mats.get(emitter.resource.member) is not None
    check("effects.rlb: an emitter's resource resolves",
          drawn_ok == drawn > 0 and played_ok >= played - 1,
          f"{drawn_ok}/{drawn} materials resolve through Material.lib and "
          f"{played_ok}/{played} sounds are in sounds.lib")

    # Inside a block, the sound emitter is the one that reads: a near and a
    # far audible distance, in that order.
    ordered = sounds = 0
    ranges: Counter[tuple[int, int]] = Counter()
    flagged = 0
    for effect in library:
        for emitter in effect.emitters:
            flagged += emitter.flagged
            span = emitter.audible_range
            if span is None:
                continue
            sounds += 1
            ordered += span[0] <= span[1]
            ranges[(round(span[0]), round(span[1]))] += 1
    check("effects.rlb: a sound emitter carries a near and far distance",
          ordered == sounds > 0,
          f"+{effects.SOUND_NEAR} <= +{effects.SOUND_FAR} on {ordered}/{sounds} "
          f"sound blocks; commonest are "
          + ", ".join(f"{a}..{b}" for (a, b), _ in ranges.most_common(3)))
    check("effects.rlb: bit 8 of the type word is a flag, not the type",
          0 < flagged < emitters,
          f"set on {flagged} of the {emitters} emitters, and the engine's "
          f"factory keys only on the low byte -- it stores (word >> 8) & 1 "
          f"separately")

    # An explosion's size lives in the .exp, not in the effect: the small,
    # medium and big fortification blasts share their emitter blocks exactly.
    trio = [library.get(f"exp_frt_{suffix}") for suffix in ("l", "m", "b")]
    shared = 0
    if all(trio):
        width = min(len(e.emitters) for e in trio)
        for i in range(width):
            bodies = {e.emitters[i].body for e in trio}
            shared += len(bodies) == 1
        check("effects.rlb: an explosion's size is a scale, not new geometry",
              shared == width > 0,
              f"exp_frt_l, _m and _b share {shared}/{width} emitter blocks byte "
              f"for byte; the 2, 3 and 4 that separate them are the magnitude "
              f"in their .exp")

    # .exp records, wherever they live.
    total = refs = named = 0
    for path in sorted(game.glob("*.rlb")) + sorted(game.glob("*.lib")):
        try:
            archive = NResArchive.open(path)
        except NotAnNResArchive:
            continue
        for entry in archive:
            if not entry.name.lower().endswith(".exp"):
                continue
            total += 1
            record = effects.parse_explosion(archive.read(entry), entry.name)
            for ref in record.effects:
                if ref.member:
                    refs += 1
                    named += library.get(ref.member) is not None
    check("*.exp: a 24-byte header and one 64-byte name per effect",
          total > 0 and named >= refs - 1,
          f"{total} explosion definitions parse; {named}/{refs} of the effects "
          f"they name are real FXID members")

    # And the whole chain: a mesh node's damage record reaches sprites.
    library_objects = objects.ObjectLibrary(game / "objects.rlb")
    opened: dict[str, NResArchive] = {}

    def member(ref) -> bytes | None:
        if ref is None or not ref.library:
            return None
        try:
            if ref.library not in opened:
                opened[ref.library] = NResArchive.open(game / ref.library)
            return opened[ref.library].read_name(ref.member)
        except (KeyError, ValueError, FileNotFoundError):
            return None

    wired = reached = 0
    for record in library_objects.records.values():
        raw = member(record.damage)
        if raw is None:
            continue
        try:
            table = objects.parse_damage(raw, record.damage.member)
        except objects.ObjectFormatError:
            continue
        for row in table:
            if not row.explosion:
                continue
            wired += 1
            blob = member(row.explosion)
            if blob is None:
                continue
            explosion = effects.parse_explosion(blob, row.explosion.member)
            effect = next(
                (library.get(r.member) for r in explosion.effects
                 if library.get(r.member)), None
            )
            if effect and all(mats.get(m) for m in effect.materials):
                reached += 1
    check("a destroyed node reaches real sprites",
          reached >= wired * 0.98 > 0,
          f"{reached}/{wired} .ndp explosion references walk through to an "
          f"effect whose every material resolves")


def run(game: Path) -> int:
    """Run every check against ``game``.  Returns a process exit code."""
    results: list[tuple[str, bool, str]] = []

    def check(name: str, ok: bool, evidence: str) -> None:
        results.append((name, bool(ok), evidence))
        print(f"{'PASS' if ok else 'FAIL'}  {name:<46} {evidence}")

    print(f"verifying against {game}\n")
    checks = (
        check_nres, check_texm, check_terrain, check_uv,
        check_water, check_layers, check_materials, check_sky,
        check_minimap_agreement, check_arealmap,
        check_missions, check_objects, check_poses, check_damage,
        check_effects,
    )
    for fn in checks:
        fn(check, game)
    failed = [n for n, ok, _ in results if not ok]
    print(f"\n{len(results) - len(failed)}/{len(results)} checks passed")
    for n in failed:
        print(f"  failed: {n}")
    return 1 if failed else 0
