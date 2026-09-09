"""Checks every factual claim the docs make against a real installation.

Each check prints PASS/FAIL and the evidence behind it.  If a claim in
``docs/`` cannot be re-derived here, the claim does not belong in the docs.

    uv run openparkan verify
"""

from __future__ import annotations

import math
import random
import struct
from collections import Counter, defaultdict
from pathlib import Path

from . import (
    arealmap,
    effects,
    font,
    gamedir,
    landmesh,
    materials,
    mission,
    objects,
    rsli,
    sky,
    texm,
)
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


def check_grid(check, game: Path) -> None:
    """Stream 2 is the map's own spatial index -- the engine shipped one."""
    maps = gamedir.maps(game)
    parsed = chained = summed = 0
    inside = checked = 0
    shapes: Counter[tuple[int, int, int]] = Counter()
    coarser = cell_pairs = 0
    for folder in maps:
        mesh = landmesh.load(folder / "Land.msh")
        cells = mesh.cells
        if not cells:
            continue
        parsed += 1
        chained += all(cells[i].first + cells[i].count == cells[i + 1].first
                       for i in range(len(cells) - 1))
        summed += sum(c.count for c in cells) == mesh.face_count
        wide, deep = mesh.grid_size
        shapes[(wide, deep, len(cells) // max(1, wide * deep))] += 1
        slack = (cells[0].maximum[0] - cells[0].minimum[0]) * 0.001
        for cell in cells:
            for face in cell.faces:
                if face >= mesh.face_count:
                    continue
                checked += 1
                points = [mesh.positions[v] for v in mesh.faces[face]]
                inside += all(
                    cell.minimum[a] - slack <= p[a] <= cell.maximum[a] + slack
                    for p in points for a in range(3)
                )
        per = len(cells) // landmesh.LOD_COUNT
        for i in range(per):
            cell_pairs += 1
            coarser += cells[i + per].count <= cells[i].count

    check("Land.msh: stream 2 is a grid of face runs", parsed == len(maps) > 0,
          f"{parsed}/{len(maps)} maps parse a cell table with nothing left "
          f"over; shapes (across, down, records per cell) {dict(shapes)}")
    check("Land.msh: the runs chain and cover every face",
          chained == summed == parsed,
          f"{chained}/{parsed} maps have first + count equal to the next "
          f"first on every record, and on {summed}/{parsed} the counts sum "
          f"exactly to the face count")
    check("Land.msh: every face lies inside its own cell", inside == checked > 0,
          f"{inside}/{checked} faces across the {parsed} maps have all three "
          f"vertices inside the box of the cell whose run holds them -- so the "
          f"faces are stored in cell order and a run indexes them directly")
    # Stream 1 is the square table over those cells.
    squared = headers = named = boxed = 0
    used: Counter[int] = Counter()
    for folder in maps:
        mesh = landmesh.load(folder / "Land.msh")
        if not mesh.cells or not mesh.squares:
            continue
        wide, deep = mesh.grid_size
        squares = wide * deep
        squared += len(mesh.squares) == squares
        named += all(sq == (i, squares + i)
                     for i, sq in enumerate(mesh.squares))
        boxed += all(
            mesh.cells[sq[0]].minimum == mesh.cells[sq[1]].minimum
            and mesh.cells[sq[0]].maximum == mesh.cells[sq[1]].maximum
            for sq in mesh.squares if len(sq) == 2
        )
        raw = NResArchive.open(folder / "Land.msh").one_of_type(
            landmesh.STREAM_SQUARES)
        words = struct.unpack(f"<{len(raw) // 2}H", raw)
        stride = landmesh.SQUARE_WORDS
        headers += all(
            words[i * stride : i * stride + 4] == (0, 0xFFFF, 0, 0)
            for i in range(len(raw) // 2 // stride)
        )
        for sq in mesh.squares:
            used[len(sq)] += 1
    check("Land.msh: stream 1 is one record per grid square",
          squared == headers == len(maps) > 0,
          f"{squared}/{len(maps)} maps hold exactly one 19-word record per "
          f"square, and on {headers} of them the four header words are "
          f"0, 0xFFFF, 0, 0 throughout -- so nothing in the data tells those "
          f"four apart")
    check("Land.msh: a square names its own two cells",
          named == boxed == len(maps) and set(used) == {2},
          f"{named}/{len(maps)} maps have square i naming cells i and "
          f"squares + i, and on {boxed} the pair always shares a bounding box; "
          f"all {sum(used.values())} squares use {sorted(used)} of the 15 "
          f"slots a record has room for")

    check("Land.msh: a square's two cells are two levels of detail",
          coarser == cell_pairs > 0,
          f"the second record of a cell holds no more faces than the first on "
          f"all {coarser}/{cell_pairs} pairs -- the same ground simplified, "
          f"not a second patch of it")


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
    # The map is stored twice, at two levels of detail -- one cell per level
    # per square -- and drawing both is what made the flat ground flicker.
    rng = random.Random(11)
    stored = drawn = 0
    sliced = covered = subset = coarser = pairs = 0
    identical = fine_total = 0
    grazing = 0
    for folder in gamedir.maps(game):
        m = landmesh.load(folder / "Land.msh")
        cells = m.cells
        per = len(cells) // landmesh.LOD_COUNT
        stored += m.face_count
        fine = m.lod_faces(0)
        coarse = m.lod_faces(1)
        drawn += len(fine)
        # Each level is one contiguous slice of the face array.
        sliced += (
            [f for c in cells[:per] for f in c.faces] == fine
            and [f for c in cells[per:] for f in c.faces] == coarse
        )
        for i in range(per):
            pairs += 1
            coarser += cells[i + per].count <= cells[i].count
        subset += (
            {v for f in coarse for v in m.faces[f]}
            <= {v for f in fine for v in m.faces[f]}
        )
        # Each level covers the map on its own: a random point lands on
        # exactly one face of each, wherever the ground is not folded.
        (minx, miny, _), (maxx, maxy, _) = m.bounds()
        hits = [0, 0]
        for _ in range(150):
            x = rng.uniform(minx, maxx)
            y = rng.uniform(miny, maxy)
            for level, faces in ((0, fine), (1, coarse)):
                if _covers(m, faces, x, y):
                    hits[level] += 1
        covered += hits[0] > 140 and hits[1] > 140
        # Where the simplifier left a triangle alone the two copies are
        # bit-identical; where it did not they graze each other.
        keys = {tuple(sorted(m.positions[v] for v in m.faces[f])) for f in fine}
        fine_total += len(fine)
        identical += sum(
            1 for f in coarse
            if tuple(sorted(m.positions[v] for v in m.faces[f])) in keys
        )
        grazing += len(coarse) - sum(
            1 for f in coarse
            if tuple(sorted(m.positions[v] for v in m.faces[f])) in keys
        )
    check("Land.msh: the map is stored twice, at two levels of detail",
          sliced == covered == subset == len(gamedir.maps(game)),
          f"each level is one contiguous slice of the face array on "
          f"{sliced} maps, each covers the whole map on its own on {covered}, "
          f"and level 1 uses no vertex level 0 does not on {subset}; level 1 "
          f"is no finer in {coarser}/{pairs} cell pairs")
    check("Land.msh: drawing one level is what stops the ground flickering",
          drawn < stored * 0.75,
          f"level 0 is {drawn} of the {stored} stored faces; {identical} of "
          f"level 1's repeat a level-0 triangle exactly -- those are the ones "
          f"the simplifier left alone -- and the other {grazing} sit a "
          f"fraction of a unit from the surface they replace, which is what "
          f"z-fights")

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


def _covers(mesh, faces, x: float, y: float) -> bool:
    """Whether any of ``faces`` spans the world point ``(x, y)``."""
    for f in faces:
        a, b, c = (mesh.positions[v] for v in mesh.faces[f])
        den = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1])
        if abs(den) < 1e-12:
            continue
        l1 = ((b[1] - c[1]) * (x - c[0]) + (c[0] - b[0]) * (y - c[1])) / den
        l2 = ((c[1] - a[1]) * (x - c[0]) + (a[0] - c[0]) * (y - c[1])) / den
        if l1 >= -1e-9 and l2 >= -1e-9 and 1.0 - l1 - l2 >= -1e-9:
            return True
    return False


def _is_name(field: bytes) -> bool:
    """A texture-name field: NUL-terminated, printable ASCII, non-empty run."""
    if b"\0" not in field:
        return False
    head = field.split(b"\0")[0]
    return all(33 <= b < 127 for b in head)


def check_materials(check, game: Path) -> None:
    """Material.lib: the record layout, the tracks, and the ground's M twin."""
    lib = materials.MaterialLibrary(game / "Material.lib")
    textures = NResArchive.open(game / "Textures.lib")
    known = {e.name.split(".")[0].upper() for e in textures}
    raw = lib.archive
    records = [e for e in raw if e.tag == materials.MATERIAL_TAG]
    total = len(lib)

    # The whole record, end to end.  A 14-byte header, 34 bytes an entry, then
    # a track table -- and nothing left over anywhere.
    exact = versioned = 0
    for entry in records:
        blob = raw.read(entry)
        count, tracks = struct.unpack_from("<2H", blob, 0)
        at = materials.HEADER_SIZE + count * materials.ENTRY_STRIDE
        exact += materials.parse_tracks(blob, at, tracks)[1] == len(blob)
        versioned += entry.link_count == materials.RECORD_VERSION
    check("Material.lib: a record parses to the byte", exact == total,
          f"{exact}/{total} records are a {materials.HEADER_SIZE}-byte header, "
          f"{materials.ENTRY_STRIDE} bytes an entry and then the track table, "
          f"with nothing left over")
    check("Material.lib: the record version lives in the archive directory",
          versioned == total,
          f"the directory entry's second count is "
          f"{materials.RECORD_VERSION} on all {versioned}/{total} records, and "
          f"the parser gates the header's last four fields on it -- which is "
          f"what makes the header 14 bytes rather than 6")

    # An entry is a D3DMATERIAL7 written as bytes: four colours, each three
    # bytes and a per-cent alpha, then the power, the cell and the name.  A
    # wrong stride or a wrong base puts arbitrary bytes in the alpha slots.
    def entry_fits(base: int, stride: int) -> int:
        """Records whose every entry reads as a D3DMATERIAL7 at this layout."""
        good = 0
        for entry in records:
            blob = raw.read(entry)
            count = struct.unpack_from("<H", blob, 0)[0]
            good += all(
                base + (i + 1) * stride <= len(blob)
                and all(
                    blob[base + i * stride + off + materials.ALPHA_STEP]
                    <= materials.ALPHA_FULL
                    for off in (0, 4, 8, 12)
                )
                and _is_name(blob[base + i * stride + materials.NAME_OFFSET:
                                  base + (i + 1) * stride])
                for i in range(count)
            )
        return good
    here = entry_fits(materials.HEADER_SIZE, materials.ENTRY_STRIDE)
    rival = max(
        entry_fits(base, stride)
        for base, stride in ((6, 34), (10, 34), (12, 34), (13, 34), (15, 34),
                             (16, 34), (14, 30), (14, 32), (14, 33),
                             (14, 35), (14, 36), (14, 40))
    )
    slots = sum(m.entry_count for m in lib.materials.values()) * 4
    check("Material.lib: an entry is a D3DMATERIAL7 and 34 bytes long",
          here == total > rival,
          f"at a 14-byte header and a stride of 34 all four alpha bytes are a "
          f"percentage -- 100 or below, over {slots} slots -- and the 16-byte "
          f"name field is terminated ASCII, in every entry of {here}/{total} "
          f"records; the best of twelve other (base, stride) pairs -- the "
          f"shorter headers the version gate would have produced among them "
          f"-- manages {rival}")

    counted = names = 0
    for m in lib.materials.values():
        if all(t.upper().split(".")[0] in known for t in m.textures):
            counted += 1
        names += len(m.textures)
    check("Material.lib: every material names a texture that exists",
          counted == total,
          f"{counted}/{total} materials name only textures that are in "
          f"Textures.lib, {names} names in all; reading them by pattern "
          f"instead of by offset left 14 that did not, qqds.7 and 0FAIR.0 "
          f"among them")

    # The ambient alpha is the only one that varies, and it is a fade.
    ramps = {}
    opacity = Counter()
    for name, m in lib.materials.items():
        values = [e.ambient_alpha for e in m.entries]
        opacity.update(round(v, 2) for v in values)
        if any(v != 1.0 for v in values):
            ramps[name] = values
    faded = sum(n for v, n in opacity.items() if v != 1.0)
    check("Material.lib: the ambient alpha is an opacity, not a marker",
          len(ramps) == 1 and faded > 0,
          f"{sum(opacity.values()) - faded} of {sum(opacity.values())} entries "
          f"are fully opaque and the exception is {', '.join(ramps)} at "
          f"{[round(v * 100) for v in next(iter(ramps.values()))][:6]} per cent "
          f"across its frames -- a fade-in, which is what a constant marker "
          f"could not be")

    tinted = sum(1 for m in lib.materials.values() if m.colour != (255, 255, 255))
    check("Material.lib: the diffuse colour is the fifth byte group",
          tinted > total // 2,
          f"{tinted}/{total} materials carry a diffuse other than white, "
          f"WATER #4d6aff and ENV_NLAVA #b41e00 among them -- the texture "
          f"under both is neutral grey")

    # 0xFF in the two version-gated bytes is the engine's own "not set": it is
    # exactly what the parser substitutes when the record is too old to carry
    # them.
    groups = Counter()
    unset_five = 0
    for entry in records:
        blob = raw.read(entry)
        if len(blob) > 5:
            groups[blob[4]] += 1
            unset_five += blob[5] == materials.UNSET
    named_groups = {g: n for g, n in groups.items() if g != materials.UNSET}
    check("Material.lib: 0xFF in the class byte means unset, not a class",
          unset_five == total and groups[materials.UNSET] > 0,
          f"byte 5 is 0xFF on all {unset_five} records and byte 4 on "
          f"{groups[materials.UNSET]}; the parser writes exactly 0xFF "
          f"into both when the record's version is below 2, so it is the "
          f"engine's own default.  The other {len(named_groups)} values sort "
          f"the library by role: {dict(sorted(named_groups.items()))}")

    # The archive directory's first count field is a flags byte the loader
    # reads -- bit 1 into one material field, bits 2..5 into another -- and it
    # sorts the library by transparency far more sharply than the class byte
    # in the record does.
    fmt = {}
    for entry in textures:
        try:
            fmt[entry.name.upper()] = texm.decode(textures.read(entry)).fmt
        except Exception:  # noqa: BLE001 - a few members are not decodable
            continue
    alpha_formats = (texm.FMT_ARGB4444, texm.FMT_ARGB8888)
    by_flag = defaultdict(Counter)
    by_class = defaultdict(Counter)
    for entry in records:
        material = lib.get(entry.name)
        texture = material.texture
        carries = fmt.get(texture.upper()) in alpha_formats if texture else False
        by_flag[entry.element_count][carries] += 1
        by_class[raw.read(entry)[4]][carries] += 1
    def purity(table) -> int:
        return sum(max(c.values()) for c in table.values())
    opaque = by_flag[materials.BLEND_OPAQUE]
    bit_one = by_flag[materials.BLEND_LIT]
    check("Material.lib: the directory's flags byte says whether a material "
          "is transparent",
          opaque[True] == 0 and purity(by_flag) > purity(by_class),
          f"not one of the {sum(opaque.values())} materials whose flags byte is "
          f"0 names a texture that carries alpha, and {bit_one[True]} of the "
          f"{sum(bit_one.values())} whose byte is 2 do; over the library the "
          f"flags byte puts {purity(by_flag)}/{total} materials in a "
          f"transparency-pure group against {purity(by_class)} for the record's "
          f"class byte.  The loader reads bit 1 of it into one material field "
          f"and bits 2..5 into another")

    # What each value is, from four independent directions: the artists'
    # naming, the lighting slots, the specular, and who names the material.
    add = [m for m in lib.materials.values() if m.blend == materials.BLEND_ADD]
    named_add = [m for m in lib.materials.values() if m.name.lower().endswith("_add")]
    in_add = sum(1 for m in named_add if m.blend == materials.BLEND_ADD)
    unlit = sum(1 for m in add if m.entries and m.entries[0].colour == (0, 0, 0))
    shiny = sum(1 for m in add
                if any(e.specular != (0, 0, 0) for e in m.entries))
    lit = [m for m in lib.materials.values() if m.blend == materials.BLEND_LIT]
    lit_shiny = sum(1 for m in lit
                    if any(e.specular != (0, 0, 0) for e in m.entries))
    check("Material.lib: flags 8 is additive",
          in_add >= len(named_add) - 2 and unlit >= len(add) - 6 and shiny == 0,
          f"{in_add} of the {len(named_add)} materials the artists named "
          f"*_add carry it, along with every JET*, SHOOT*, LASER_* and "
          f"SPLASH*; {unlit} of the {len(add)} carry a black diffuse, so they "
          f"are drawn unlit, and not one of them carries a specular colour "
          f"against {lit_shiny}/{len(lit)} of the flags-2 skins")

    # What a see-through material's alpha actually is, which is what a
    # renderer has to branch on: a silhouette takes an alpha test, a graded
    # one takes blending.
    stems = {e.name.split(".")[0].upper(): e for e in textures}
    def alpha_kind(material) -> str:
        texture = material.texture
        entry = stems.get(texture.upper().split(".")[0]) if texture else None
        if entry is None:
            return "none"
        pixels = texm.decode(textures.read(entry))
        channel = pixels.rgba[3::4]
        if not any(v < 16 for v in channel):
            return "opaque"
        return "silhouette" if texm.is_cutout(pixels) else "graded"
    see = Counter(alpha_kind(m) for m in lib.materials.values()
                  if m.blend == materials.BLEND_ALPHA)
    lit = Counter(alpha_kind(m) for m in lib.materials.values()
                  if m.blend == materials.BLEND_LIT)
    check("Material.lib: a see-through material's alpha is real",
          see["graded"] + see["silhouette"] > sum(see.values()) * 0.85
          and lit["silhouette"] < sum(lit.values()) * 0.02,
          f"the flags-4 materials name {dict(sorted(see.items()))} textures "
          f"and the flags-2 skins {dict(sorted(lit.items()))} -- so a "
          f"see-through material really does carry transparency, and the "
          f"silhouettes among it are the foliage (FTREE1, HTREE1, GRASS, "
          f"ELKA), which want an alpha test rather than blending")

    # Who names which group.  Each consumer lands in one, sharply.
    def group_of(names) -> Counter:
        out: Counter[int] = Counter()
        for name in names:
            m = lib.get(name.split(".")[0])
            if m is not None:
                out[m.blend] += 1
        return out
    ground = group_of(
        n for folder in gamedir.maps(game)
        for table in (landmesh.load(folder / "Land.msh").layer1_names,
                      landmesh.load(folder / "Land.msh").layer2_names)
        for n in table if n
    )
    emitters = group_of(
        n for e in effects.EffectLibrary(game / "effects.rlb").effects.values()
        for n in e.materials
    )
    check("Material.lib: the flags groups line up with who names them",
          ground[materials.BLEND_OPAQUE] > sum(ground.values()) * 0.6
          and emitters[materials.BLEND_ADD] > emitters[materials.BLEND_LIT] * 50,
          f"the terrain's layer tables name "
          f"{ground[materials.BLEND_OPAQUE]}/{sum(ground.values())} at flags 0, "
          f"and the materials an effect's emitters name are "
          f"{dict(sorted(emitters.items()))} -- {emitters[materials.BLEND_ADD]} "
          f"additive glows and {emitters[materials.BLEND_ALPHA]} smokes against "
          f"{emitters[materials.BLEND_LIT]} ordinary skins")

    # The second uint16 counts animation tracks, not texture layers.  The
    # engine caps it at 20 and every shipped record is far below.
    by_tracks = Counter(m.track_count for m in lib.materials.values())
    keys = [k for m in lib.materials.values() for t in m.tracks for k in t.keys]
    in_range = sum(1 for m in lib.materials.values()
                   for t in m.tracks for k in t.keys if k.entry < m.entry_count)
    animated = sum(1 for m in lib.materials.values() if m.frame_count > 1)
    check("Material.lib: the second count is animation tracks, not layers",
          max(by_tracks) <= materials.MAX_TRACKS and in_range == len(keys),
          f"the engine refuses more than {materials.MAX_TRACKS} of them "
          f'("Too many animations for material."); the library holds '
          f"{dict(sorted(by_tracks.items()))} and all {in_range} keys across "
          f"them name an entry that exists.  {animated} materials animate")

    multi = [m for m in lib.materials.values() if m.track_count > 1]
    singles = sum(1 for m in multi
                  for t in m.tracks if len(t.keys) == 1)
    own = sum(1 for m in multi
              for i, t in enumerate(m.tracks)
              if len(t.keys) == 1 and t.keys[0].entry == i)
    check("Material.lib: a multi-track material is variants, not frames",
          len(multi) == 45 and own == singles == sum(m.track_count for m in multi),
          f"every one of the {singles} tracks across the {len(multi)} "
          f"materials that have more than one holds a single key, and track i "
          f"names entry i on all {own} -- so the extra tracks are alternative "
          f"renderings of the same surface, not later frames of one")

    # The ground's M twin is the second track, and what separates it from the
    # first is the lighting rather than the texture.
    twins = [m for m in lib.materials.values() if m.track_count == 2]
    suffixed = sum(1 for m in twins if len(m.textures) == 2
                   and m.textures[1].upper() == m.textures[0].upper().replace(".0", "M.0"))
    unlit = sum(1 for m in twins
                if (m.variant(0) and m.variant(1)
                    and m.variant(0).lit and not m.variant(1).lit
                    and m.variant(1).ambient == (255, 255, 255)))
    check("Material.lib: the ground's M twin is its second track, drawn unlit",
          suffixed == len(twins) and unlit >= len(twins) - 6,
          f"on all {suffixed} of the {len(twins)} two-track materials the "
          f"second track's entry names the first's texture with an M inserted, "
          f"and on {unlit} of them that entry carries a black diffuse over a "
          f"white ambient where the first carries the reverse -- so the scene "
          f"light reaches one and not the other")

    # The two eight-track materials are eight cells of one sheet.
    eights = [m for m in lib.materials.values() if m.track_count == 8]
    one_sheet = all(len({t.upper() for t in m.textures}) == 1 for m in eights)
    cells_ok = all(sorted(e.cell for e in m.entries) == list(range(8))
                   for m in eights)
    check("Material.lib: an eight-track material is eight cells of one sheet",
          len(eights) == 2 and one_sheet and cells_ok,
          f"{[m.name for m in eights]} each name "
          f"{eights[0].textures[0] if eights else '?'} eight times and ask for "
          f"cells 0-7 of it -- team variants of one insignia sheet, not eight "
          f"images")

    # What the twin's texture is, measured against the base.
    by_stem = {e.name.split(".")[0].upper(): e for e in textures}
    pairs = []
    for m in twins:
        if len(m.textures) != 2:
            continue
        a, b = (by_stem.get(t.upper().split(".")[0]) for t in m.textures)
        if a is None or b is None:
            continue
        pairs.append((m, texm.decode(textures.read(a)), texm.decode(textures.read(b))))
    def from_grey(pix) -> float:
        """Mean per-pixel distance from neutral grey, over a sample."""
        n = pix.width * pix.height
        return sum(
            abs(pix.rgba[i * 4] - 128) + abs(pix.rgba[i * 4 + 1] - 128)
            + abs(pix.rgba[i * 4 + 2] - 128)
            for i in range(0, n, 7)
        ) / (3 * len(range(0, n, 7)))

    sized = fmt = neutral = 0
    widest = ("", 0.0, 0.0)
    for _m, a, b in pairs:
        sized += (a.width, a.height) == (b.width, b.height)
        fmt += a.fmt == texm.FMT_RGB565 and b.fmt == texm.FMT_XRGB8888
        if (a.width, a.height) != (b.width, b.height):
            continue
        near, far = from_grey(a), from_grey(b)
        neutral += far < near
        if near - far > widest[1] - widest[2]:
            widest = (_m.name, near, far)
    check("Material.lib: a two-track material is a texture and its M twin",
          len(pairs) >= 40 and sized == len(pairs) == fmt,
          f"{len(pairs)} materials name a pair; all {sized} hold the same "
          f"dimensions, and all {fmt} are RGB565 paired with XRGB8888")
    check("Material.lib: the M half is the flattened one",
          neutral >= len(pairs) - 3,
          f"on {neutral}/{len(pairs)} pairs the M texture sits closer to "
          f"neutral grey than the base does -- {widest[0]} goes "
          f"{widest[1]:.0f} to {widest[2]:.0f} -- so it is the base flattened "
          f"towards 128, which is what an unlit copy of a lit surface looks "
          f"like; the three exceptions differ by about a point")

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
          f"{flat}/{with_water} maps with water -- so no water surface is ever "
          f"behind another, which is what lets the viewer draw it see-through "
          f"without writing depth")


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

    # The sub-image cell indexes the texture's own Page table -- on every
    # entry of every material, now that the entries can be walked.
    indexed = inside = biggest = 0
    pages_seen = set()
    page_cache: dict[str, int] = {}
    for material in lib.materials.values():
        for item in material.entries:
            if item.cell == materials.WHOLE_TEXTURE or not item.texture:
                continue
            entry = textures_by_stem.get(item.texture.upper().split(".")[0])
            if entry is None:
                continue
            indexed += 1
            biggest = max(biggest, item.cell)
            if entry.name not in page_cache:
                page_cache[entry.name] = len(texm.parse_pages(textures.read(entry)))
            pages_seen.add(entry.name)
            inside += item.cell < page_cache[entry.name]
    check("Texm: a material's cell indexes the texture's own Page table",
          inside == indexed > 0,
          f"{inside}/{indexed} cells fall inside the Page table of the texture "
          f"they name, over {len(pages_seen)} textures and cells up to {biggest} "
          f"-- every entry of every material, not just the first")

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

    # Where the sun stands is not in the file -- CSun takes two whole-degree
    # angles from a block Terrain.dll fills with constants, picked by whether
    # the keyframe's name is exactly "sun".  Nothing in the data can confirm a
    # constant directly, but three things it implies are checkable.

    # One: the engine's test is name == "sun", so the only two names that can
    # reach it must be the two it distinguishes.
    named: dict[str, int] = {}
    sections = 0
    paired = 0
    overlaps = 0
    odd = []
    for path in files:
        try:
            atmosphere = sky.load(path)
        except sky.SkyFormatError:
            continue
        for k in atmosphere.keyframes:
            if k.name:
                named[k.name] = named.get(k.name, 0) + 1
        for section in sorted({k.section for k in atmosphere.keyframes}):
            sections += 1
            window = {}
            for name in sky.BODY_ANGLES:
                marks = sorted(
                    k.minutes for k in atmosphere.keyframes
                    if k.name == name and k.section == section
                )
                if len(marks) == 2:
                    window[name] = marks
            if len(window) == 2:
                paired += 1
                (rise, set_), (moonrise, moonset) = window["sun"], window["moon"]
                overlaps += not (set_ <= moonrise or moonset <= rise)
            else:
                odd.append(f"{path.parent.name}/{section}")
    bodies = {n: named.get(n, 0) for n in sky.BODY_ANGLES}
    check("sky.ske: the only bodies a keyframe names are the sun and the moon",
          set(named) - {sky.RAIN_MARKER, sky.LIGHTNING_MARKER}
          == set(sky.BODY_ANGLES),
          f"{bodies} against the engine's single test, name == 'sun'")
    check("sky.ske: the sun and the moon come in start/stop pairs",
          paired >= sections - 3,
          f"{paired}/{sections} sections hold exactly one pair of each; the "
          f"other {len(odd)} are two five-keyframe skies that name only the "
          f"sun and one that names each body once, so the reader toggles on "
          f"each mark rather than assuming a pair")
    check("sky.ske: the sun and the moon are never up together", overlaps == 0,
          f"{overlaps} of {paired} paired sections overlap -- the sun runs "
          f"about 01:30 to 15:00 and the moon 16:20 to midnight, which is what "
          f"makes two fixed positions a quarter turn apart coherent")

    # Two: the block's fourth field is 3 for the sun and 4 for the moon, and
    # SLOT_ROLES -- read out of sky.wea, quite separately -- says the same.
    check("CSun: the engine's slot for each body is the sky.wea role",
          all(sky.SLOT_ROLES[slot] == name
              for name, slot in sky.BODY_SLOT.items()),
          ", ".join(f"{name} -> slot {slot} = {sky.SLOT_ROLES[slot]}"
                    for name, slot in sky.BODY_SLOT.items()))

    # Three: the flare's second gate ramps between cos 60 and cos 30, and the
    # sun's own tilt is 30 degrees off the zenith -- so its height is cos 30
    # exactly and it sits on the top edge of the ramp.  That coincidence is
    # what identifies the gate as the body's height.
    heights = {n: sky.body_direction(n)[2] for n in sky.BODY_ANGLES}
    check("CSun: the sun stands exactly at the top of the flare's second gate",
          abs(heights["sun"] - sky.FLARE_HEIGHT_FULL) < 1e-6
          and sky.flare_height_gate(heights["sun"]) == 1.0,
          f"the sun's height is {heights['sun']:.6f} against a ramp that tops "
          f"out at {sky.FLARE_HEIGHT_FULL:.6f}; the moon's "
          f"{heights['moon']:.6f} gives "
          f"{sky.flare_height_gate(heights['moon']):.3f}")
    check("CSun: both bodies stand above the horizon and a quarter turn apart",
          all(sky.body_elevation(n) > 0 for n in sky.BODY_ANGLES)
          and abs(sky.BODY_ANGLES["sun"][0] - sky.BODY_ANGLES["moon"][0]) == 90,
          f"sun {sky.body_elevation('sun'):.0f} degrees up, moon "
          f"{sky.body_elevation('moon'):.0f}, azimuths "
          f"{sky.BODY_ANGLES['sun'][0]:.0f} and "
          f"{sky.BODY_ANGLES['moon'][0]:.0f}")


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
            posed = model.posed_positions()
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
    # Bridges come in halves placed back to back, their rotations exactly pi
    # apart, so their ends must meet.  That pins the sense of the placement
    # angle, which nothing else in the data does.
    joined = flipped = pairs = 0
    for folder in gamedir.missions(game):
        m = mission.load(folder / "data.tma")
        spans = [o for o in m.objects if "bridge" in (o.path or "").lower()]
        for i in range(0, len(spans) - 1, 2):
            first, second = spans[i], spans[i + 1]
            if abs(abs(second.rotation - first.rotation) - math.pi) > 0.01:
                continue
            definition = objects.load_unit(
                game / first.path.replace("\\", "/")
            )
            model = load(record_mesh(lib.get(definition.components[0].ref.member)))
            if model is None or model.volume is None:
                continue
            pairs += 1
            lo, hi = model.volume.minimum, model.volume.maximum
            # The half's long horizontal axis, and its two ends in model space.
            axis = 0 if hi[0] - lo[0] > hi[1] - lo[1] else 1
            ends = []
            for angle in (1, -1):
                here = []
                for o in (first, second):
                    t = angle * o.rotation
                    for reach in (lo[axis], hi[axis]):
                        p = [0.0, 0.0]
                        p[axis] = reach
                        here.append((
                            o.position[0] + p[0] * math.cos(t) - p[1] * math.sin(t),
                            o.position[1] + p[0] * math.sin(t) + p[1] * math.cos(t),
                        ))
                ends.append(min(
                    math.dist(a, b) for a in here[:2] for b in here[2:]
                ))
            joined += ends[0] < 1.0
            flipped += ends[1] < 1.0
    check("placement: a bridge's two halves meet",
          joined == pairs > 0 and flipped < pairs,
          f"{joined}/{pairs} pairs join to within a unit when the placement "
          f"angle is used as it stands, against {flipped}/{pairs} when it is "
          f"negated -- which is what fixes the sense of the rotation")

    check("placement: a unit's lowest exterior vertex sits on the terrain",
          base_on_ground >= units * 0.5,
          f"{base_on_ground}/{units} within 1 unit of the height under them")


def check_lod(check, game: Path) -> None:
    """The five slots of a variant: only the first is in the model's own frame."""
    filled = Counter()
    triangles = Counter()
    fits = Counter()
    seen = Counter()
    for library in sorted(game.glob("*.rlb")):
        try:
            archive = NResArchive.open(library)
        except NotAnNResArchive:
            continue
        for entry in archive:
            if not entry.name.lower().endswith(".msh"):
                continue
            try:
                m = objmesh.parse(archive.read(entry), entry.name)
            except (ValueError, struct.error):
                continue
            if not m.volume:
                continue
            low, high = m.volume.minimum, m.volume.maximum
            span = max(high[a] - low[a] for a in range(3)) or 1.0
            posed = m.posed_positions()
            for k in range(objmesh.SLOTS_PER_VARIANT):
                vertices: set[int] = set()
                count = 0
                for node in m.nodes:
                    if node.is_collision:
                        continue
                    index = (node.slot_index[k] if k < len(node.slot_index)
                             else objmesh.NO_SLOT)
                    if index == objmesh.NO_SLOT or index >= len(m.slots):
                        continue
                    slot = m.slots[index]
                    for b in m.batches[slot.first_batch:
                                       slot.first_batch + slot.batch_count]:
                        first, n = b.triangles
                        count += n
                        for t in range(first, min(first + n, len(m.triangles))):
                            vertices.update(m.triangles[t])
                if not vertices:
                    continue
                filled[k] += 1
                triangles[k] += count
                seen[k] += 1
                points = [posed[v] for v in vertices]
                over = max(
                    max(low[a] - min(q[a] for q in points),
                        max(q[a] for q in points) - high[a])
                    for a in range(3)
                ) / span
                fits[k] += over <= 0.02

    falls = all(triangles[k] > triangles[k + 1] for k in range(3))
    check("MESH: slots 0 to 3 fall like a level-of-detail chain", falls,
          "triangles per slot position: "
          + ", ".join(f"{k}: {triangles[k]}" for k in range(4))
          + f" -- but slot 4 goes back up to {triangles[4]} over only "
          f"{filled[4]} meshes, so it is not a fifth level")
    check("MESH: every level is a simplification in place",
          fits[0] == seen[0] > 0
          and all(fits[k] >= seen[k] * 0.65 for k in range(1, 5)),
          ", ".join(f"slot {k}: {fits[k]}/{seen[k]}"
                    for k in range(objmesh.SLOTS_PER_VARIANT))
          + " fit the model's authored box -- once every slot is posed, not "
            "just the level being read.  The coarse levels sit on level 0's "
            "centre to within a median 0.000 to 0.011 of the model's size and "
            "match its extent to within 0.001 to 0.037, so they are the same "
            "object with detail removed.  The stragglers are trees and stones "
            "whose silhouette a simplification legitimately changes")

    # Posing every slot at once is only safe because the slots never disagree.
    clashes = 0
    checked = 0
    for library in sorted(game.glob("*.rlb")):
        try:
            archive = NResArchive.open(library)
        except NotAnNResArchive:
            continue
        for entry in archive:
            if not entry.name.lower().endswith(".msh"):
                continue
            try:
                m = objmesh.parse(archive.read(entry), entry.name)
            except (ValueError, struct.error):
                continue
            checked += 1
            owner: dict[int, tuple] = {}
            for ni, node in enumerate(m.nodes):
                pose = m.world_pose(ni)
                for si in node.slot_index:
                    if si == objmesh.NO_SLOT or si >= len(m.slots):
                        continue
                    slot = m.slots[si]
                    stop = slot.first_triangle + slot.triangle_count
                    for tri in m.triangles[slot.first_triangle:stop]:
                        for v in tri:
                            if owner.get(v, pose) != pose:
                                clashes += 1
                                break
                            owner[v] = pose
    check("MESH: no vertex is shared by slots under different poses",
          clashes == 0 and checked > 0,
          f"{clashes} of {checked} meshes have a vertex reached by two slots "
          f"whose nodes pose it differently, which is what lets "
          f"posed_positions pose every level at once")

    # Buildings you can walk into: 21 of the 435 meshes carry nodes the file
    # marks internal, and the inside is most of the model.
    hollow = solid = 0
    inside_tris = outside_tris = 0
    graphed = 0
    graph_only: list[str] = []
    biggest = ("", 0, 0)
    for library in sorted(game.glob("*.rlb")):
        try:
            archive = NResArchive.open(library)
        except NotAnNResArchive:
            continue
        for entry in archive:
            if not entry.name.lower().endswith(".msh"):
                continue
            blob = archive.read(entry)
            try:
                m = objmesh.parse(blob, entry.name)
            except (ValueError, struct.error):
                continue
            try:
                graph = objmesh.read_path_graph(NResArchive(blob))
            except (ValueError, struct.error, NotAnNResArchive):
                graph = None
            if not m.has_interior:
                solid += 1
                if graph:
                    graph_only.append(entry.name)
                continue
            hollow += 1
            graphed += graph is not None
            inner = len(m.select(0, interior=True))
            outer = len(m.select(0, interior=False))
            inside_tris += inner
            outside_tris += outer
            if inner > biggest[1]:
                biggest = (entry.name, inner, outer)
    check("MESH: a building's interior is most of its model",
          hollow > 15 and inside_tris > outside_tris * 2,
          f"{hollow} of the {hollow + solid} meshes carry nodes the file marks "
          f"internal, {inside_tris} triangles of inside against "
          f"{outside_tris} of shell -- {biggest[0]} alone is {biggest[1]} "
          f"against {biggest[2]}")
    check("MESH: every interior comes with a graph to walk it",
          graphed == hollow > 0,
          f"{graphed}/{hollow} of the meshes with an interior also carry a "
          f"path graph, and the {len(graph_only)} that carry a graph without "
          f"one are bridges and ruins -- things you cross rather than enter: "
          f"{sorted(graph_only)[:4]}")

    # The animation: 157 of the 435 meshes carry one, and what makes it
    # playable as a bone per node is that it is rigid.
    animated = placed = unplaced = 0
    timed = mistimed = 0
    rested = restless = 0
    still = 0
    for library in sorted(game.glob("*.rlb")):
        try:
            archive = NResArchive.open(library)
        except NotAnNResArchive:
            continue
        for entry in archive:
            if not entry.name.lower().endswith(".msh"):
                continue
            try:
                m = objmesh.parse(archive.read(entry), entry.name)
            except (ValueError, struct.error):
                continue
            if not m.animated:
                continue
            animated += 1
            owner = m.node_of_vertex()
            placed += sum(1 for v in owner if v != objmesh.NO_NODE)
            unplaced += sum(1 for v in owner if v == objmesh.NO_NODE)
            for ni in range(len(m.nodes)):
                run = m.track(ni)
                if not run:
                    still += 1
                    continue
                if m.rest_key(ni) == run[0]:
                    rested += 1
                else:
                    restless += 1
                last = None
                for at, key in enumerate(run):
                    if key == last or key >= len(m.keys):
                        continue
                    last = key
                    if abs(m.keys[key].time - at) < 1e-6:
                        timed += 1
                    else:
                        mistimed += 1
    check("MESH: an animation is rigid, one node to a vertex",
          animated > 100 and unplaced == 0,
          f"{animated} of the 435 meshes carry an animation, and every one of "
          f"the {placed} vertices they hold is reached by exactly one node -- "
          f"so a bone per node at a single weight plays it, with nothing to "
          f"blend")
    check("MESH: a pose key's time is the frame it starts at",
          mistimed == 0 and timed > 30000,
          f"{timed}/{timed + mistimed} keys across the {animated} animated "
          f"meshes; the frame map repeats a key to hold it, so the times are "
          f"what a player interpolates between -- a turret's nine frames are "
          f"six keys, and stepping them would jump 90 degrees at a time")
    check("MESH: an animated node's rest pose is its first frame",
          restless == 0 and rested > 0,
          f"{rested}/{rested + restless} animated nodes, over {still} that "
          f"never move -- which is what lets a bind pose be taken from the "
          f"rest pose and the animation start on it")


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


def check_rsli(check, game: Path) -> None:
    """gamefont.rlb and sprites.lib -- the two archives that are not NRes."""
    archives = []
    for name in ("gamefont.rlb", "sprites.lib"):
        path = game / name
        if path.exists() and rsli.is_rsli(path):
            archives.append((name, rsli.RsLiArchive.open(path)))
    members = sum(len(a) for _n, a in archives)
    named = sum(
        1 for _n, a in archives for e in a
        if e.name and e.name.isascii() and e.name == e.name.upper() and "." in e.name
    )
    check("RsLi: the entry table decrypts to real names", named == members > 0,
          f"{named}/{members} entries across {len(archives)} archives read as "
          f"uppercase ASCII with an extension -- {', '.join(e.name for e in archives[0][1])}"
          if archives else "no RsLi archives found")

    totals = all(sum(e.size for e in a) == a.total for _n, a in archives)
    check("RsLi: the unpacked sizes sum to the header's total", totals,
          ", ".join(f"{n} {sum(e.size for e in a)}/{a.total}" for n, a in archives))

    unpacked = 0
    kinds: Counter[str] = Counter()
    refused: Counter[str] = Counter()
    for _name, archive in archives:
        for entry in archive:
            try:
                blob = archive.read(entry)
            except rsli.RsLiFormatError:
                refused[entry.storage] += 1
                continue
            if len(blob) == entry.size:
                unpacked += 1
                kinds[entry.storage] += 1
    check("RsLi: every member unpacks to the size it declares",
          unpacked == members > 0 and not refused,
          f"{unpacked}/{members} members across both archives, "
          f"{dict(kinds)} -- nothing is refused any more")

    # And the sprites are ordinary textures once they are out.
    sprites = next((a for n, a in archives if n == "sprites.lib"), None)
    decoded = 0
    if sprites is not None:
        for entry in sprites:
            try:
                texm.decode(sprites.read(entry))
                decoded += 1
            except (texm.UnsupportedTexture, rsli.RsLiFormatError, ValueError):
                pass
    # The font, which the LZSS kept shut until the decoder was right.
    fonts = rsli.RsLiArchive.open(game / "gamefont.rlb")
    blobs = {}
    for entry in fonts:
        try:
            blobs[entry.name.upper()] = fonts.read(entry)
        except rsli.RsLiFormatError:
            pass
    check("RsLi: the LZSS members unpack to the size they declare",
          len(blobs) == len(fonts) == 2,
          f"{len(blobs)}/{len(fonts)} members of gamefont.rlb, both 0x040 -- "
          f"the offset is an absolute ring index, the ring starts at "
          f"{rsli.LZSS_START:#x} and is filled with spaces")

    glyphs = font.parse_font(blobs["ARIALTEX.TFT"])
    palette = font.parse_palette(blobs["PAL.PAL"])
    atlas = texm.decode(glyphs.atlas, palette.raw)
    fits = sum(1 for g in glyphs.glyphs
               if g.drawn and g.width(atlas.width) == g.advance + 1)
    ordered = sum(1 for g in glyphs.glyphs if g.u0 <= g.u1)
    check("Tfnt: a glyph's advance matches the span it occupies",
          fits == len(glyphs.drawn) > 100 and ordered == font.GLYPH_COUNT,
          f"on all {fits} of the {len(glyphs.drawn)} drawn glyphs the span is "
          f"exactly advance + 1; the other "
          f"{font.GLYPH_COUNT - len(glyphs.drawn)} are the placeholder, and "
          f"u0 <= u1 on {ordered}/{font.GLYPH_COUNT}")
    check("Tfnt: the glyphs sit in rows of one height",
          len(glyphs.rows) > 3
          and len({round((b - a) * atlas.height)
                   for a, b in zip(glyphs.rows, glyphs.rows[1:], strict=False)}) == 1,
          f"{len(glyphs.rows)} rows at v = "
          + ", ".join(str(round(v * atlas.height)) for v in glyphs.rows))

    lit = {atlas.rgba[i * 4 : i * 4 + 3]
           for i in range(atlas.width * atlas.height)
           if any(atlas.rgba[i * 4 : i * 4 + 3])}
    check("Texm: pixel format 2 indexes the external palette",
          atlas.fmt == texm.FMT_INDEX8 and lit == {bytes((255, 255, 255))},
          f"the {atlas.width}x{atlas.height} atlas is format {atlas.fmt}, one "
          f"byte per pixel, and every lit pixel comes out {sorted(lit)[0].hex()} "
          f"-- white, through PAL.PAL")

    table = palette.blend
    side = font.TABLE_SIDE
    symmetric = sum(1 for a in range(side) for b in range(side)
                    if table[a * side + b] == table[b * side + a])
    diagonal = sum(1 for i in range(side) if table[i * side + i] == i)
    check("PAL.PAL: the Ipol table is a symmetric colour mixer",
          symmetric == side * side and diagonal > side * 0.9,
          f"table[a][b] == table[b][a] on all {symmetric} cells and "
          f"table[i][i] == i on {diagonal}/{side} -- mixing a colour with "
          f"itself returns it, and the {side - diagonal} that do not are "
          f"indices the palette never uses")

    check("RsLi: sprites.lib holds Texm textures", sprites and decoded == len(sprites),
          f"{decoded}/{len(sprites) if sprites else 0} members decode as Texm, "
          f"which is the whole 2D interface")


def check_footprints(check, game: Path) -> None:
    """.bas ground plans, and whether a placed building sits on its own."""
    archive = NResArchive.open(game / "fortif.rlb")
    plans: dict[str, list[objects.Footprint]] = {}
    rings = anticlockwise = 0
    for entry in archive:
        if not entry.name.lower().endswith(".bas"):
            continue
        try:
            plan = objects.parse_base(archive.read(entry), entry.name)
        except objects.ObjectFormatError:
            continue
        plans[entry.name.lower()[:-4]] = plan
        rings += len(plan)
        anticlockwise += sum(1 for r in plan if r.area > 0)
    total = sum(1 for e in archive if e.name.lower().endswith(".bas"))
    two = sum(1 for p in plans.values() if len(p) == 2)
    check("fortif.rlb: a .bas is two closed rings",
          len(plans) == total > 0 and two == total,
          f"{len(plans)}/{total} records parse and {two} hold exactly two "
          f"rings; all {anticlockwise}/{rings} wind anticlockwise")

    # The inner ring traces the model, the outer one stands off from it.
    boxed = matched = 0
    ratios: list[float] = []
    for stem, plan in plans.items():
        inner, outer = plan[0], plan[1]
        if inner.area:
            ratios.append(abs(outer.area / inner.area))
        try:
            model = objmesh.parse(archive.read_name(stem + ".msh"), stem)
        except (KeyError, ValueError, struct.error):
            continue
        if model.volume is None:
            continue
        boxed += 1
        lo, hi = model.volume.minimum, model.volume.maximum
        xs = [p[0] for p in inner.points]
        ys = [p[1] for p in inner.points]
        matched += (
            abs(min(xs) - lo[0]) < 0.1 and abs(max(xs) - hi[0]) < 0.1
            and abs(min(ys) - lo[1]) < 0.1 and abs(max(ys) - hi[1]) < 0.1
        )
    ratios.sort()
    check("fortif.rlb: the inner ring is the model, the outer a clearance",
          matched >= boxed * 0.6 and ratios[0] > 1.0,
          f"the inner ring's XY extent is the model's own bounding box on "
          f"{matched}/{boxed}; the outer ring is {ratios[0]:.2f} to "
          f"{ratios[-1]:.2f} times its area, {ratios[len(ratios)//2]:.2f} median")

    # And placed on a map, the outline lands on the ground.
    library = objects.ObjectLibrary(game / "objects.rlb")

    def plan_of(path: str) -> list[objects.Footprint] | None:
        target = game / path.replace("\\", "/")
        if not target.exists():
            return None
        try:
            unit = objects.load_unit(target)
        except (objects.ObjectFormatError, OSError, struct.error):
            return None
        for component in unit.components:
            record = library.get(component.ref.member)
            slot = record.footprint if record else None
            if slot:
                return plans.get(slot.member.lower()[:-4])
        return None

    heights: dict[Path, landmesh.LandMesh] = {}
    spreads: list[float] = []
    drops: list[float] = []
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
        ground = heights[land]
        for o in m.objects:
            if o.kind != mission.KIND_BUILDING or o.is_static:
                continue
            plan = plan_of(o.path)
            if not plan:
                continue
            angle = o.rotation
            under = []
            for x, y, _z in plan[0].points:
                wx = o.position[0] + x * math.cos(angle) - y * math.sin(angle)
                wy = o.position[1] + x * math.sin(angle) + y * math.cos(angle)
                h = ground.height_at(wx, wy)
                if h is not None:
                    under.append(h)
            if len(under) < 3:
                continue
            spreads.append(max(under) - min(under))
            drops.append(o.position[2] - sum(under) / len(under))
    spreads.sort()
    drops.sort()
    middle = abs(drops[len(drops) // 2]) if drops else 99.0
    check("placement: a building's footprint lands on the ground",
          len(drops) > 100 and middle < 0.5
          and spreads[len(spreads) // 2] < 5.0,
          f"over {len(drops)} placed buildings the terrain under the outline "
          f"spans a median {spreads[len(spreads)//2]:.2f} units, and the "
          f"placement height sits a median {middle:.2f} above its mean")


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

    # Which floats of a block are live.  Each class keeps a pointer to its own
    # block and the live fields are whatever its virtual methods read through
    # it; the set came out of Effect.dll.  Two things check it.
    reads = effects.READ_OFFSETS
    recorded = sum(len(v) for v in reads.values())
    stray = [
        (t, at) for t, offs in reads.items() for at in offs
        if at + 4 > effects.EMITTER_SIZE[t]
        or (effects.RESOURCE_AT.get(t) is not None
            and effects.RESOURCE_AT[t] <= at < effects.RESOURCE_AT[t] + 64)
    ]
    check("Effect.dll: every float a class reads lies in its own block",
          not stray and recorded > 100,
          f"{recorded}/{recorded} recorded offsets fall inside the block and "
          f"none of them lands in the (archive, member) pair -- the read map "
          f"comes from the code and RESOURCE_AT from the data, and they do "
          f"not collide anywhere")

    # The control: the sound emitter's two distances were read off the data
    # long before this map existed, so the map has to contain them.
    check("Effect.dll: the read map contains the distances already known",
          effects.SOUND_NEAR in reads[effects.EMITTER_SOUND]
          and effects.SOUND_FAR in reads[effects.EMITTER_SOUND],
          f"type {effects.EMITTER_SOUND} reads "
          f"{len(reads[effects.EMITTER_SOUND])} offsets and "
          f"+{effects.SOUND_NEAR} and +{effects.SOUND_FAR} are among them")

    slots = sum(effects.EMITTER_SIZE[t] // 4 for t in effects.EMITTER_SIZE)
    check("Effect.dll: most of an emitter block is never read",
          recorded < slots / 2,
          f"{recorded} of the {slots} four-byte slots across the ten block "
          f"types are loaded as a float; the rest the editor writes and the "
          f"engine never looks at")

    # A third check on the read map, and the sharpest: a slot the engine loads
    # as a float should hold one.  The map came out of the vtables and knows
    # nothing about the data.
    def sane(v: float) -> bool:
        return math.isfinite(v) and (v == 0.0 or 1e-6 <= abs(v) <= 1e6)
    bodies: dict[int, list[bytes]] = {}
    for effect in library.effects.values():
        for em in effect.emitters:
            bodies.setdefault(em.kind, []).append(em.body)
    live_ok = live_n = dead_ok = dead_n = 0
    live_slots = dead_slots = 0
    shape: Counter[str] = Counter()
    for kind, blocks in bodies.items():
        read = set(effects.READ_OFFSETS.get(kind, ()))
        resource = effects.RESOURCE_AT.get(kind)
        for off in range(4, len(blocks[0]) - 3, 4):
            if resource is not None and resource <= off < resource + 64:
                continue
            values = [struct.unpack_from("<f", b, off)[0]
                      for b in blocks if off + 4 <= len(b)]
            good = sum(1 for v in values if sane(v))
            if off in read:
                live_slots += 1
                live_ok += good
                live_n += len(values)
                lo, hi = min(values), max(values)
                if lo == hi == 0.0:
                    shape["zero"] += 1
                elif all(v == int(v) for v in values) and hi < 1e6:
                    shape["integral"] += 1
                elif lo >= 0 and hi <= 1.0:
                    shape["0..1"] += 1
                elif lo >= 0:
                    shape["positive"] += 1
                else:
                    shape["signed"] += 1
            else:
                dead_slots += 1
                dead_ok += good
                dead_n += len(values)
    check("effects.rlb: every slot the engine reads holds a real float",
          live_ok == live_n and dead_ok < dead_n * 0.98,
          f"all {live_ok} reads of the {live_slots} live slots are finite and "
          f"either zero or between 1e-6 and 1e6, against "
          f"{100 * dead_ok / max(dead_n, 1):.1f}% of the {dead_slots} dead "
          f"ones -- {dead_n - dead_ok} of those are NaN, denormal or absurd.  "
          f"The map came from Effect.dll's vtables and knows nothing about "
          f"the data, so this is a third witness to it")
    check("effects.rlb: the live floats sort by shape",
          sum(shape.values()) == live_slots and shape["signed"] > 0,
          f"{dict(sorted(shape.items()))} -- and the recurring motif is a "
          f"component-wise (low, high) triple, which type 3's +40..+48 and "
          f"+52..+60 were the first of")

    # Types 9 and 3 read the same fields because 9's constructor installs 3's
    # vtable before overriding it; 7 and 10 are siblings on one 0x48 object.
    check("Effect.dll: the emitter classes fall into families",
          reads[9] == reads[3] and reads[7] == reads[10]
          and set(reads[4]) < set(reads[3]),
          f"types 3 and 9 read the same {len(reads[3])} offsets, 7 and 10 the "
          f"same {len(reads[7])}, and 4 reads a strict subset of 3's, short "
          f"by {sorted(set(reads[3]) - set(reads[4]))}")

    # And one field is identified by its shape rather than by the code: types
    # 1 and 2 keep a unit vector where the drawing types keep something else.
    unit = defaultdict(lambda: [0, 0])
    for effect in library:
        for emitter in effect.emitters:
            vector = emitter.direction
            if vector is None:
                continue
            unit[emitter.kind][1] += 1
            length = math.sqrt(sum(v * v for v in vector))
            unit[emitter.kind][0] += abs(length - 1.0) < 0.01
    ok = all(good >= total * 0.95 for good, total in unit.values())
    check("effects.rlb: types 1 and 2 keep a unit vector at +52", ok and unit,
          ", ".join(f"type {t}: {g}/{n} unit length"
                    for t, (g, n) in sorted(unit.items()))
          + " -- a direction, defaulting to (1, 0, 0)")

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
        check_grid, check_missions, check_objects, check_poses, check_lod, check_damage,
        check_effects, check_footprints, check_rsli,
    )
    for fn in checks:
        fn(check, game)
    failed = [n for n, ok, _ in results if not ok]
    print(f"\n{len(results) - len(failed)}/{len(results)} checks passed")
    for n in failed:
        print(f"  failed: {n}")
    return 1 if failed else 0
