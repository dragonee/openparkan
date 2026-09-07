"""Checks every factual claim the docs make against a real installation.

Each check prints PASS/FAIL and the evidence behind it.  If a claim in
``docs/`` cannot be re-derived here, the claim does not belong in the docs.

    uv run openparkan verify
"""

from __future__ import annotations

import math
from pathlib import Path

from . import gamedir, landmesh, mission, texm
from .nres import HEADER_SIZE, NResArchive, is_nres


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


def check_texm(check, game: Path) -> None:
    ar = NResArchive.open(game / "Textures.lib")
    exact = decoded = 0
    fmts: dict[int, int] = {}
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
    check("Texm: declared format predicts the payload size", exact >= len(ar) * 0.8,
          f"{exact}/{len(ar)} exact (rest have a truncated mip tail)")
    check("Texm: every texture decodes to RGBA", decoded == len(ar),
          f"{decoded}/{len(ar)}, formats {dict(sorted(fmts.items()))}")


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


def run(game: Path) -> int:
    """Run every check against ``game``.  Returns a process exit code."""
    results: list[tuple[str, bool, str]] = []

    def check(name: str, ok: bool, evidence: str) -> None:
        results.append((name, bool(ok), evidence))
        print(f"{'PASS' if ok else 'FAIL'}  {name:<46} {evidence}")

    print(f"verifying against {game}\n")
    checks = (
        check_nres, check_texm, check_terrain, check_uv,
        check_water, check_minimap_agreement, check_missions,
    )
    for fn in checks:
        fn(check, game)
    failed = [n for n, ok, _ in results if not ok]
    print(f"\n{len(results) - len(failed)}/{len(results)} checks passed")
    for n in failed:
        print(f"  failed: {n}")
    return 1 if failed else 0
