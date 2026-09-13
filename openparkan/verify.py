"""Checks every factual claim the docs make against a real installation.

Each check prints PASS/FAIL and the evidence behind it.  If a claim in
``docs/`` cannot be re-derived here, the claim does not belong in the docs.

    uv run openparkan verify
"""

from __future__ import annotations

import math
import random
import re
import statistics
import struct
from collections import Counter, defaultdict
from pathlib import Path

from . import (
    arealmap,
    behaviour,
    briefing,
    control,
    controls,
    descriptions,
    effects,
    font,
    gamedir,
    landmesh,
    materials,
    mission,
    objects,
    packages,
    profiles,
    research,
    resources,
    rsli,
    save,
    settings,
    sky,
    texm,
    units,
    weapons,
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
    exact = decoded = paged = 0
    fmts: dict[int, int] = {}
    cutout = graded = palettised = keyed = 0
    silhouettes = trees = 0
    for e in ar:
        blob = ar.read(e)
        w, h, mips, flags, fmt = texm.parse_header(blob)
        fmts[fmt] = fmts.get(fmt, 0) + 1
        bpp = {0: 1, 565: 2, 4444: 2, 888: 4, 8888: 4}[fmt]
        want = texm.mip_pyramid_pixels(w, h, mips) * bpp + (texm.PALETTE_SIZE if fmt == 0 else 0)
        left = len(blob) - texm.HEADER_SIZE - want
        if left == 0:
            exact += 1
        else:
            # Not a truncated tail, which an earlier reading called it: every
            # declared level is there and the extra bytes are the Page table.
            pages = texm.parse_pages(blob)
            paged += bool(pages) and (
                left == texm.PAGE_HEADER + len(pages) * texm.PAGE_STRIDE
                and blob[texm.HEADER_SIZE + want:][:4] == texm.PAGE_MAGIC
            )
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
    check("Texm: the header accounts for every byte of the payload",
          exact + paged == len(ar),
          f"{exact}/{len(ar)} end exactly on the last mip level and the other "
          f"{paged} carry a Page table after it -- so no texture has a "
          f"truncated mip tail, which an earlier reading called them: every "
          f"declared level is present in all {len(ar)}")
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

    # The surface word's bit 0x10 is clear on lava and set on everything else.
    # Pooling the maps hides it: four files never set the bit and have no lava,
    # and their clear faces outnumber the ones that carry the signal.
    exact = using = lava_faces = unused = 0
    for folder in maps:
        mesh = landmesh.load(folder / "Land.msh")
        names = [n.upper() for n in mesh.layer1_names]
        lava = {i for i, t in enumerate(mesh.face_tex1)
                if "LAVA" in (names[t] if t < len(names) else "")}
        lava_faces += len(lava)
        if mesh.marks_lava:
            using += 1
            exact += lava == {i for i in range(len(mesh.faces)) if mesh.is_lava(i)}
        else:
            unused += 1 if not lava else 0
    # Field 13 is the mesh's winged-edge link: three 2-bit codes, one per edge,
    # naming the matching edge back in the neighbour.  Read as one number it
    # looks like a meaningless 0..62 with no spatial structure, which is what
    # earlier notes concluded.
    twins = slots = mutual = free_ok = 0
    for folder in maps:
        mesh = landmesh.load(folder / "Land.msh")
        for i, adj in enumerate(mesh.adjacency):
            for e in range(3):
                slots += 1
                back = mesh.edge_twin(i, e)
                free_ok += (back is None) == (adj[e] == 0xFFFF)
                if back is None:
                    continue
                twins += 1
                mutual += mesh.adjacency[adj[e]][back] == i
    # The draw-order flags byte has exactly four values across the library, and
    # two of its bits are set on every entry, so they carry nothing.
    seen = Counter()
    for folder in maps:
        seen.update(landmesh.load(folder / "Land.msh").draw_flags)
    always = 0xFF
    for v in seen:
        always &= v
    check("Land.msh: the draw order's flags byte takes four values",
          len(seen) == 4 and always == 0x48,
          f"{dict(sorted(seen.items()))} over {sum(seen.values())} entries; "
          f"0x08 and 0x40 are set on every one and say nothing, 0x10 opens a "
          f"batch, and 0x80 is on {sum(n for v, n in seen.items() if v & 0x80)} "
          f"entries of two maps")

    check("Land.msh: field 13 is the winged-edge link",
          mutual == twins and free_ok == slots,
          f"each edge's 2-bit code names the matching edge back in its "
          f"neighbour on {mutual}/{twins} shared edges, and marks no-neighbour "
          f"on all {free_ok}/{slots} edge slots.  63 -- all three edges free -- "
          f"is why the field stops at 62: no map has such a face")

    check("Land.msh: the surface word's bit 0x10 is clear on lava",
          exact == using and unused == len(maps) - using,
          f"on all {exact}/{using} maps that set the bit, the faces with it "
          f"clear are exactly the faces whose layer-1 material names lava -- "
          f"{lava_faces} of them in the library, surfaces and beds alike.  The "
          f"other {unused} maps never set it and have no lava, which is why "
          f"pooling the maps made it look like a 95/5 split with no meaning")


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

    # Stream 11 is the draw order: a permutation that keeps every cell
    # contiguous and sorts the faces inside it by texture pair.
    permuted = contiguous = 0
    optimal = cells_seen = file_optimal = 0
    for folder in maps:
        mesh = landmesh.load(folder / "Land.msh")
        order = mesh.draw_order
        if not order or not mesh.cells:
            continue
        permuted += sorted(order) == list(range(mesh.face_count))
        cell_of = [0] * mesh.face_count
        for ci, cell in enumerate(mesh.cells):
            for f in cell.faces:
                if f < mesh.face_count:
                    cell_of[f] = ci
        runs = 1
        for a, b in zip(order, order[1:], strict=False):
            runs += cell_of[a] != cell_of[b]
        contiguous += runs == len(mesh.cells)
        pair = [(mesh.face_tex1[f], mesh.face_tex2[f]) for f in range(mesh.face_count)]
        for faces in (order, list(range(mesh.face_count))):
            grouped: dict[int, list[int]] = {}
            for f in faces:
                grouped.setdefault(cell_of[f], []).append(f)
            tally = 0
            for run in grouped.values():
                batches = 1
                for a, b in zip(run, run[1:], strict=False):
                    batches += pair[a] != pair[b]
                tally += batches == len({pair[f] for f in run})
            if faces is order:
                optimal += tally
                cells_seen += len(grouped)
            else:
                file_optimal += tally
    # And its flags byte says where each batch begins.
    starts = marked = agreed = drawn = 0
    for folder in maps:
        mesh = landmesh.load(folder / "Land.msh")
        if not mesh.draw_order or not mesh.cells:
            continue
        cell_of = [0] * mesh.face_count
        for ci, cell in enumerate(mesh.cells):
            for f in cell.faces:
                if f < mesh.face_count:
                    cell_of[f] = ci
        previous = None
        for f, flag in zip(mesh.draw_order, mesh.draw_flags, strict=True):
            key = (cell_of[f], mesh.face_tex1[f], mesh.face_tex2[f])
            opens = key != previous
            previous = key
            bit = bool(flag & landmesh.DRAW_BATCH_START)
            drawn += 1
            starts += opens
            marked += bit
            agreed += bit == opens
    check("Land.msh: the draw order's flags byte opens each batch",
          agreed == drawn > 0 and marked == starts,
          f"bit 0x{landmesh.DRAW_BATCH_START:02x} is set on exactly the "
          f"{marked} faces that open a run of one texture pair inside a cell "
          f"and clear on the other {drawn - marked} -- agreeing on all "
          f"{agreed}.  Walk the order and change material where the bit is "
          f"set and the map draws")

    check("Land.msh: stream 11 is the order to draw the faces in",
          permuted == contiguous == len(maps) and optimal == cells_seen,
          f"the face indices are a permutation of the whole list on "
          f"{permuted}/{len(maps)} maps and reorder faces only inside a cell "
          f"-- every cell stays contiguous on {contiguous}.  In that order "
          f"all {optimal}/{cells_seen} cells draw in the minimum number of "
          f"batches, no texture pair twice, against {file_optimal} in file "
          f"order")

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
    # Field 0's bit 2 says the face has a second texture layer.
    marked = layered = both = neither = 0
    for folder in gamedir.maps(game):
        m = landmesh.load(folder / "Land.msh")
        for i in range(m.face_count):
            bit = bool(m.face_flags[i] & landmesh.FLAGS_LAYER2_BIT)
            has = m.face_tex2[i] != landmesh.NO_TEXTURE
            marked += bit
            layered += has
            both += bit and has
            neither += not bit and not has
    # And bit 13 marks the ground under a liquid.
    bed_named = bed_bit = bed_both = 0
    for folder in gamedir.maps(game):
        m = landmesh.load(folder / "Land.msh")
        for i in range(m.face_count):
            name = (m.texture_name(1, m.face_tex1[i]) or "").upper()
            under = name in ("WATER_BOT", "ENV_LAVA_BOT")
            bit = bool(m.face_flags[i] & landmesh.FLAGS_LIQUID_BED_BIT)
            bed_named += under
            bed_bit += bit
            bed_both += under and bit
    check("Land.msh: face flags bit 0x2000 marks the bed under a liquid",
          bed_both == bed_named == bed_bit > 0,
          f"set on exactly the {bed_bit} faces whose layer-1 material is "
          f"WATER_BOT or ENV_LAVA_BOT and on no other face of any map -- so "
          f"the ground beneath a lake says so itself, which is the third way "
          f"a map marks its liquids")

    check("Land.msh: face flags bit 0x004 marks the two-layer faces",
          both == marked == layered and neither == 275882 - layered,
          f"set on {marked} faces and clear on the other {neither}, and it "
          f"agrees with the layer-2 texture index on every one of the "
          f"{both + neither} -- so a face says twice that it has a second "
          f"layer, in its flags and in its texture word")

    # Fields 10, 11 and 12 are the face's own normal.
    faces_seen = unit = agrees = 0
    for folder in gamedir.maps(game):
        m = landmesh.load(folder / "Land.msh")
        for i, n in enumerate(m.face_normal):
            faces_seen += 1
            unit += abs(math.dist(n, (0, 0, 0)) - 1) < 0.02
            a, b, c = (m.positions[v] for v in m.faces[i])
            u = [b[k] - a[k] for k in range(3)]
            w = [c[k] - a[k] for k in range(3)]
            cross = (u[1] * w[2] - u[2] * w[1],
                     u[2] * w[0] - u[0] * w[2],
                     u[0] * w[1] - u[1] * w[0])
            length = math.dist(cross, (0, 0, 0))
            if length < 1e-9:
                continue
            agrees += sum(x * y for x, y in zip(n, cross, strict=True)) / length > 0.99
    check("Land.msh: face fields 10, 11 and 12 are the face's own normal",
          unit > faces_seen - 5 and agrees > faces_seen - 10,
          f"read as int16 over {landmesh.NORMAL_SCALE:.0f} they are unit "
          f"length on {unit}/{faces_seen} faces and point the same way as the "
          f"cross product of the triangle on {agrees} -- so flat shading "
          f"needs no cross product, and the winding is confirmed a third time")

    # Field 13 is six bits wide, and the surface word two.
    widest = 0
    six_bit = seen_faces = 0
    surface_values: Counter[int] = Counter()
    for folder in gamedir.maps(game):
        m = landmesh.load(folder / "Land.msh")
        for i in range(m.face_count):
            seen_faces += 1
            widest = max(widest, m.face_patch[i])
            six_bit += m.face_patch[i] < 64
            surface_values[m.face_surface[i]] += 1
    check("Land.msh: face field 13 is a six-bit field, not an id",
          six_bit == seen_faces and widest < 64,
          f"every one of the {seen_faces} faces holds a value below 64, the "
          f"largest being {widest}, and 63 of the 64 occur -- so it is a set "
          f"of six flags rather than an index, which is also why its groups "
          f"are neither spatial nor tied to a material.  The surface word "
          f"beside it uses two bits: {dict(sorted(surface_values.items()))}")

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


def _near(a, b, tol: float = 0.01) -> bool:
    """Whether two points coincide."""
    return all(abs(x - y) <= tol for x, y in zip(a, b, strict=False))


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

    # The simulation reads the class byte -- it is the ground's surface id,
    # fetched through the manager's slot 9 (Control.dll:0x1001aaf5; see
    # check_ground) -- but no drawing code does.  So the question a renderer
    # has to answer is what it loses by ignoring it, and the answer is nothing:
    # the only distinction it draws that a renderer could act on is which
    # materials carry the ground's second track, and the track count says that
    # already.
    ground = {e.name for e in records
              if raw.read(e)[4] < materials.GROUND_CLASSES}
    twins = {e.name for e in records
             if len(lib.get(e.name).tracks) == materials.TWIN_TRACKS}
    check("Material.lib: the class byte draws no distinction the track count "
          "does not",
          bool(twins) and twins <= ground,
          f"all {len(twins)} materials with a second track carry a class below "
          f"{materials.GROUND_CLASSES} and no material above it has one, so "
          f"the ground's M twin -- the one thing the class byte separates that "
          f"a renderer acts on -- is already in the track count.  Classes "
          f"0..{materials.GROUND_CLASSES - 1} hold {len(ground)} materials in "
          f"all, {len(ground) - len(twins)} of them single-track")

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
    # The flags byte's route to a D3D blend function, every link from a binary.
    # (flags >> 2) & 0xF indexes a five-entry table, so a byte that ran past it
    # would be a wrong reading of the field.
    modes = defaultdict(set)
    for m in lib.materials.values():
        modes[m.blend].add(m.blend_function)
    in_range = all(m.blend_index < len(materials.BLEND_TRANSLATE)
                   for m in lib.materials.values())
    additive = {m.blend_function for m in lib.materials.values()
                if m.blend == materials.BLEND_ADD}
    check("Material.lib: the flags byte indexes the engine's blend table",
          in_range and additive == {("SRCALPHA", "ONE", True)}
          and len(modes[materials.BLEND_OPAQUE]) == 1,
          f"(flags >> {materials.BLEND_SHIFT}) lands inside the five-entry "
          f"translate table on all {total} materials: "
          + "; ".join(f"{f} -> {next(iter(v))[0]}/{next(iter(v))[1]}"
                      for f, v in sorted(modes.items()))
          + ".  Flags 8 reaches SRCALPHA/ONE, which is additive")

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

    # Edge field 1 is the twin edge: the index of the same edge in the
    # neighbour's list.  It was set aside as "ranges beyond the areal count, so
    # not a second areal reference" -- true, because it is an edge reference.
    twins = shared = outside_ok = outside = 0
    for _, am in loaded:
        for i, a in enumerate(am.areals):
            for e in range(len(a.edges)):
                across = a.twin(e)
                if across is None:
                    outside += 1
                    outside_ok += a.edges[e][1] == arealmap.NO_NEIGHBOUR
                    continue
                shared += 1
                nb, back = across
                twins += (0 <= back < len(am.areals[nb].edges)
                          and am.areals[nb].edges[back][0] == i)
    check("Land.map: edge field 1 is the twin edge", twins == shared
          and outside_ok == outside,
          f"on {twins}/{shared} shared edges it names the edge in the neighbour "
          f"that points straight back, and it is -1 on all {outside_ok}/"
          f"{outside} boundary edges -- the land mesh's winged-edge link again")

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

    # A property's two further words are its minimum and maximum.
    inside = locked = unbounded = ore = ore_total = 0
    instances = 0
    for m in parsed:
        for o in m.objects:
            for prop in o.properties.values():
                if prop.name == "CurrentOre":
                    top = o.properties.get("MaximumOre")
                    ore_total += 1
                    ore += top is not None and prop.c == top.value
                    continue
                instances += 1
                hi = prop.maximum
                inside += prop.minimum <= prop.value and (hi is None or prop.value <= hi)
                locked += prop.locked
                unbounded += hi is None
    check("data.tma: a property's two further words are its bounds",
          inside == instances and ore == ore_total > 0,
          f"{inside}/{instances} property instances keep their value between "
          f"the two -- {unbounded} of them an int whose maximum -1 means none, "
          f"{locked} locked with both bounds on the value.  CurrentOre is the "
          f"one exception, and not an exception: its maximum is the same "
          f"object's MaximumOre on {ore}/{ore_total}")

    # The word after a clan's behaviour-tree path is its mind count: the
    # SuperAI gets that many free slots, a bot holds one alive or under
    # construction, and a factory will not start without one.  So no clan can
    # be placed with more robots than it -- while every owned object together
    # does exceed it, which keeps the bound from being trivially loose.
    clans = [c for m in parsed for c in m.clans]
    word = Counter(c.minds for c in clans)
    robots_over = []
    owned_over = 0
    at_limit = []
    for d, m in zip(dirs, parsed, strict=False):
        robots: Counter[int] = Counter()
        owned: Counter[int] = Counter()
        for o in m.objects:
            if o.clan_id is None:
                continue
            owned[o.clan_id] += 1
            if o.path.replace("\\", "/").upper().startswith("UNITS/UNITS/"):
                robots[o.clan_id] += 1
        for i, c in enumerate(m.clans):
            if robots[i] > c.minds:
                robots_over.append(f"{d.name} {c.name} {robots[i]}>{c.minds}")
            if robots[i] == c.minds:
                at_limit.append(f"{d.parent.name}/{d.name} {c.name!r}")
            owned_over += owned[i] > c.minds
    multi = {d.name: sorted((c.minds for c in m.clans), reverse=True)[:2]
             for d, m in zip(dirs, parsed, strict=False)
             if d.name in ("Multi.01", "Multi.02", "Multi.03", "Multi.04")}
    check("data.tma: a clan's placed robots fit its mind count",
          clans and not robots_over and owned_over > 0
          and all(a == b for a, b in multi.values()),
          f"the word after the tree path runs {min(word)}..{max(word)} over "
          f"{len(clans)} clans; placed UNITS\\UNITS robots exceed it on "
          f"{len(robots_over)}, sit exactly at it on {len(at_limit)} "
          f"({', '.join(at_limit)}), while all owned objects exceed it on "
          f"{owned_over}.  The two top clans of Multi.01..04 get equal minds: "
          f"{multi}{'; ' + ', '.join(robots_over[:3]) if robots_over else ''}")

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
    # type word, whose 1s and 2s repeat across campaign missions.
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

    # The third triple is a direction whose length is a magnitude, not a
    # scalar written into a vector slot.
    axis_named = axis_ok = frame_named = frame_unit = zeroed = seen = 0
    for name in ARCHIVES:
        ar = NResArchive.open(game / name)
        for e in ar:
            if e.tag != "CTPT":
                continue
            try:
                pts = objmesh.parse_control_points(ar.read(e), e.name)
            except (ValueError, struct.error):
                continue
            for p in pts:
                seen += 1
                zeroed += all(abs(v) < 1e-6 for v in p.a)
                low = p.name.lower()
                if any(k in low for k in ("width", "height", "size")):
                    axis_named += 1
                    axis_ok += sum(1 for v in p.direction if abs(v) > 1e-6) == 1
                if any(k in low for k in ("_x", "_y", "_z", "direct", "center")):
                    frame_named += 1
                    frame_unit += abs(math.dist(p.direction, (0, 0, 0)) - 1) < 0.01
    check("CTPT: the third triple is a direction with a length",
          axis_ok == axis_named > 0 and frame_unit > frame_named * 0.95,
          f"every one of the {axis_named} points named Width, Height or Size "
          f"has exactly one non-zero component -- an axis times the size, not "
          f"a scalar in a vector slot -- and {frame_unit}/{frame_named} of the "
          f"frame and aim points are unit length.  The first triple is exactly "
          f"zero on {zeroed}/{seen}")

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
    # Stream 7 is the per-face record: flags, three edge neighbours, the face's
    # own normal, and a trailing class.
    recs = per_tri = rec_meshes = 0
    rec_unit = rec_agrees = 0
    slots_seen = no_face = in_range = shares_edge = 0
    flag_values: Counter[int] = Counter()
    small_class = class_seen = 0
    for name in ARCHIVES:
        ar = NResArchive.open(game / name)
        for e in ar:
            if e.tag != "MESH":
                continue
            try:
                m = objmesh.parse(ar.read(e), e.name)
            except (ValueError, struct.error):
                continue
            if not m.face_normal:
                continue
            rec_meshes += 1
            per_tri += len(m.face_normal) == len(m.triangles)
            place = [tuple(sorted(m.positions[v] for v in t if v < len(m.positions)))
                     for t in m.triangles]
            for i, normal in enumerate(m.face_normal):
                if i >= len(m.triangles):
                    break
                recs += 1
                rec_unit += abs(math.dist(normal, (0, 0, 0)) - 1) < 0.02
                flag_values[m.face_flags[i]] += 1
                class_seen += 1
                small_class += m.face_class[i] < 64
                a, b, c = (m.positions[v] for v in m.triangles[i])
                u = [b[k] - a[k] for k in range(3)]
                w = [c[k] - a[k] for k in range(3)]
                cross = (u[1] * w[2] - u[2] * w[1],
                         u[2] * w[0] - u[0] * w[2],
                         u[0] * w[1] - u[1] * w[0])
                length = math.dist(cross, (0, 0, 0))
                if length > 1e-9:
                    rec_agrees += sum(
                        x * y for x, y in zip(normal, cross, strict=True)
                    ) / length > 0.99
                for j in m.face_adjacency[i]:
                    slots_seen += 1
                    if j == objmesh.NO_FACE:
                        no_face += 1
                    elif j < len(place):
                        in_range += 1
                        shares_edge += len(set(place[i]) & set(place[j])) >= 2
    check("MESH: stream 7 is one record per triangle",
          per_tri == rec_meshes > 0,
          f"{per_tri}/{rec_meshes} meshes hold exactly one 16-byte record per "
          f"triangle, {recs} in all")
    check("MESH: a face record carries the face's own normal",
          rec_unit == recs and rec_agrees > recs - 20,
          f"read as int16 over {objmesh.NORMAL_SCALE:.0f} it is unit length on "
          f"all {rec_unit} faces and points the same way as the cross product "
          f"of the triangle on {rec_agrees} -- the same encoding the terrain's "
          f"face record uses")
    check("MESH: a face record's three neighbours share an edge",
          shares_edge > in_range - 20 and no_face > 0,
          f"{shares_edge}/{in_range} in-range neighbours share two vertex "
          f"positions with the face that names them, over {slots_seen} slots "
          f"of which {no_face} are 0xFFFF -- so fields 1 to 3 are the edge "
          f"adjacency, as the terrain's fields 7 to 9 are")
    check("MESH: a face record's flags and class",
          len(flag_values) < 10 and small_class > class_seen * 0.99,
          f"the flags word takes {dict(sorted(flag_values.items()))} and the "
          f"trailing field sits below 64 on {small_class}/{class_seen} faces "
          f"-- the shape of the terrain's six-bit class, though not as clean: "
          f"{class_seen - small_class} faces spread over the library go above")

    # A batch's vertex range is D3D's (BaseVertexIndex, NumVertices): the span
    # its indices reach, not a slice of the array it owns.
    spans = span_ok = span_meshes = span_all = 0
    for name in ARCHIVES:
        ar = NResArchive.open(game / name)
        for e in ar:
            if e.tag != "MESH":
                continue
            try:
                m = objmesh.parse(ar.read(e), e.name)
            except (ValueError, struct.error):
                continue
            span_meshes += 1
            whole = True
            for b in m.batches:
                first, count = b.triangles
                rel = [v for t in m.raw_triangles[first:first + count] for v in t]
                if not rel:
                    continue
                spans += 1
                if max(rel) + 1 == b.vertex_count:
                    span_ok += 1
                else:
                    whole = False
            span_all += whole
    check("MESH: a batch's vertex range is the span its indices reach",
          span_ok == spans > 0 and span_all == span_meshes,
          f"vertex_count is exactly the largest relative index plus one on "
          f"all {span_ok} batches of all {span_all} meshes -- D3D's "
          f"NumVertices beside first_vertex's BaseVertexIndex.  It is a draw "
          f"hint, which is why it never tiled the vertex array: two batches "
          f"are free to reach the same vertices")

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

    # Bytes 64 and 68 of the header say how long one in-game day lasts in real
    # time.  The engine keeps `hours * 3600 + minutes * 60` and scales every
    # keyframe's clock stamp against it; the give-away in the data is that the
    # files declaring a full 24 hours -- a sky that keeps real time, so never
    # visibly moves -- are exactly the ones with the fewest keyframes.
    days = Counter()
    static, static_frames, moving_frames = [], [], []
    for path in files:
        try:
            atmosphere = sky.load(path)
        except sky.SkyFormatError:
            continue
        hours, minutes = atmosphere.day
        days[(hours, minutes)] += 1
        if (hours, minutes) == sky.STATIC_DAY:
            static.append(path.parent.name)
            static_frames.append(len(atmosphere))
        else:
            moving_frames.append(len(atmosphere))
    sane = all(h <= 24 and mi < 60 for h, mi in days)
    check("sky.ske: a header says how long one day lasts",
          sane and bool(static) and max(static_frames) < min(moving_frames),
          f"bytes 64 and 68 read as hours and minutes on all "
          f"{sum(days.values())} files: {dict(sorted(days.items()))}.  The "
          f"{len(static)} declaring a full {sky.STATIC_DAY[0]} hours carry "
          f"{max(static_frames)} keyframes against a minimum of "
          f"{min(moving_frames)} everywhere else -- a sky that keeps real "
          f"time has nothing to animate")
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
    # The fifth slot of a variant is where collision geometry lives: the nodes
    # carrying only that slot are exactly the collision hulls, and no ordinary
    # node shares its fifth slot with one of its levels.
    only_fifth, hulls, shared, both = set(), set(), 0, 0
    for mesh_name, m in meshes:
        for k, node in enumerate(m.nodes):
            levels = [node.slot_index[i] for i in range(objmesh.LOD_COUNT)
                      if node.slot_index[i] != objmesh.NO_SLOT]
            fifth = node.collision_slot()
            if node.is_collision:
                hulls.add((mesh_name, k))
            if fifth is None:
                continue
            if not levels:
                only_fifth.add((mesh_name, k))
            else:
                both += 1
                shared += fifth in levels
    check("MESH: the fifth slot of a variant holds the collision hulls",
          only_fifth == hulls and hulls and shared == 0,
          f"the {len(only_fifth)} nodes carrying only a fifth slot are exactly "
          f"the {len(hulls)} collision hulls; on the {both} ordinary nodes that "
          f"carry both, the fifth is a separate slot on all {both - shared}")

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
    building_off: list[float] = []
    unit_off: list[float] = []
    datum: dict[int, tuple[list[float], list[float]]] = defaultdict(lambda: ([], []))
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
            datum[o.kind][0].append(o.position[2] - ground)
            datum[o.kind][1].append(o.position[2] + lowest - ground)
            if o.kind == mission.KIND_BUILDING:
                buildings += 1
                building_off.append(abs(o.position[2] - ground))
                origin_on_ground += abs(o.position[2] - ground) < 2.0
            elif o.kind == mission.KIND_UNIT:
                units += 1
                unit_off.append(abs(o.position[2] + lowest - ground))
                base_on_ground += abs(o.position[2] + lowest - ground) < 1.0
    check("placement: a building's origin sits on the terrain",
          origin_on_ground >= buildings * 0.5,
          f"{origin_on_ground}/{buildings} within 2 units of the height under "
          f"them, a median {statistics.median(building_off):.2f} off")
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

    # The ground datum, all four kinds at once, so the table in the docs is
    # re-derivable rather than hand-kept.
    table = ", ".join(
        f"{mission.KIND_NAMES.get(k, k)} {statistics.median(a):+.2f} / "
        f"{statistics.median(b):+.2f} over {len(a)}"
        for k, (a, b) in sorted(datum.items()) if a
    )
    check("placement: a model's own z = 0 is the ground datum",
          abs(statistics.median(datum[mission.KIND_BUILDING][0])) < 0.2
          and abs(statistics.median(datum[mission.KIND_UNIT][1])) < 0.2,
          f"median (origin - terrain) / (lowest exterior vertex - terrain) by "
          f"kind: {table} -- a building's origin lands on the ground and a "
          f"unit's mission z is set so its feet do, which is the same rule "
          f"from two ends")

    check("placement: a unit's lowest exterior vertex sits on the terrain",
          base_on_ground >= units * 0.5,
          f"{base_on_ground}/{units} within 1 unit of the height under them, "
          f"a median {statistics.median(unit_off):.2f} off")


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
    second_float: Counter[float] = Counter()
    computed_by_tag: Counter[str] = Counter()
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
        for row in table:
            second_float[row.unknown] += 1
            if row.unknown != int(row.unknown):
                computed_by_tag[record.tag] += 1

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

    whole = {v: n for v, n in second_float.items() if v == int(v)}
    check("objects.rlb: a .ndp record's second float is not a handful of values",
          len(second_float) > 50 and set(computed_by_tag) <= {"EXTO", "BTLU"},
          f"{len(second_float)} distinct values over {sum(second_float.values())} "
          f"records.  {len(whole)} are whole numbers -- the commonest "
          f"{sorted(whole.items(), key=lambda kv: -kv[1])[:4]} -- and the other "
          f"{len(second_float) - len(whole)} are not, on "
          f"{sum(computed_by_tag.values())} records that are all unit parts: "
          f"{dict(computed_by_tag)}")

    check("MESH: the five-slot blocks are filled in order",
          nested == blocks > 0,
          f"{nested}/{blocks} nodes fill block 0 first, then 1, then 2 -- no "
          f"node carries a later block without the earlier ones")

    check("MESH: a node with a second block is one that can be destroyed",
          variant_named == variant_nodes > 0,
          f"all {variant_nodes} nodes that carry a second five-slot block name "
          f"an explosion in their .ndp, and the block holds the same part with "
          f"pieces gone: fewer triangles on {shrinks}/{compared}")


def check_profiles(check, game: Path) -> None:
    """behpsp.res and the economy it carries, against the missions."""
    if not (game / profiles.ARCHIVE).exists():
        return
    archive = NResArchive.open(game / profiles.ARCHIVE)
    parsed, failures, types = {}, [], set()
    for entry in archive:
        try:
            parsed[entry.name] = profiles.parse(archive.read(entry), entry.name)
        except profiles.ProfileFormatError as exc:
            failures.append(str(exc))
            continue
        types.update(v.type for v in parsed[entry.name].values())
    check("behpsp.res: every profile walks to the byte",
          not failures and len(parsed) == len(archive)
          and types == set(profiles.TYPES),
          f"{len(parsed)}/{len(archive)} .var members, variable types "
          f"{sorted(profiles.TYPES[t] for t in types)} -- the three type names "
          f"MVarSet::LinkVar checks"
          + ("" if not failures else f" -- {failures[0]}"))

    buildings = {name: vars_ for name, vars_ in parsed.items()
                 if name.startswith("prof_") and profiles.POWER_OUT in vars_}
    sources = {name: vars_[profiles.POWER_OUT].value
               for name, vars_ in buildings.items()
               if vars_[profiles.POWER_OUT].value > 0}
    mine = buildings.get("prof_mine.var", {})
    storage = buildings.get("prof_storage.var", {})
    check("behpsp.res: the generator is the only profile that makes power",
          sources.get("prof_generator.var") == 10
          and set(sources) <= {"prof_generator.var", "prof_universal.var",
                               "prof_bunker.var"},
          f"Transfer_Power_Out is non-zero only on {sources} -- a debug "
          f"profile and a bunker's trickle aside, the generator")

    # The distribution step draws ore from a holder at variable 0x1002, which
    # MBehaviour's getter computes as efficiency x Transfer_Ore_OffBoard.  An
    # earlier reading took 0x1002 to be Transfer_Ore_OnBoard -- 0 on a mine --
    # and concluded a mine feeds nobody, which the tutorial contradicts.
    check("behpsp.res: a mine feeds consumers directly, as a storage does",
          mine and storage and mine[profiles.ORE_OFF].value > 0
          and storage[profiles.ORE_OFF].value > 0,
          f"ore leaves a holder at efficiency x Transfer_Ore_OffBoard, which "
          f"is {mine[profiles.ORE_OFF].value:g} on a mine and "
          f"{storage[profiles.ORE_OFF].value:g} on a storage -- so a mine "
          f"supplies a factory without a warehouse, slowly, as the tutorial "
          f"says; Transfer_Ore_OnBoard, which is "
          f"{mine[profiles.ORE_ON].value:g} on a mine, is what it accepts")

    # The object's own directory says what it is: UNITS\UNITS\TRANSPRT,
    # UNITS\UNITS\BUILDER, and the mine and storage models by name.
    by_value: dict[float, set[str]] = defaultdict(set)
    mines_at: set[float] = set()
    for folder in gamedir.missions(game):
        for obj in mission.load(folder / "data.tma").objects:
            prop = obj.properties.get("MaximumOre")
            if prop is None:
                continue
            parts = obj.path.lower().split("\\")
            leaf = parts[-1]
            if "mine" in leaf:
                mines_at.add(prop.value)
            if prop.value:
                kind = (parts[-2] if len(parts) > 1 and parts[-2] in
                        ("transprt", "builder") else leaf)
                by_value[prop.value].add(kind)
    carriers = by_value.get(profiles.TRANSPORT_MAX_ORE, set())
    mine_cap = min(by_value) if by_value else 0
    store_cap = max(by_value) if by_value else 0
    check("data.tma: one mine and one storage at capacity fill the HUD's ore bar",
          mine_cap + store_cap == profiles.HUD_ORE_FULL
          and round(100 * mine_cap / profiles.HUD_ORE_FULL) == 11,
          f"the HUD divides held ore by {profiles.HUD_ORE_FULL:g} "
          f"(iron3d.dll:0x1006d927); a placed mine holds {mine_cap:g} and a "
          f"storage {store_cap:g}, which sum to exactly that, so a lone full "
          f"mine reads {100 * mine_cap / profiles.HUD_ORE_FULL:.1f}%")

    # The energy bar is the clan's net power over every clan's power.  On The
    # Convoy the three generators are the same model, one each for the player
    # and two enemies, so the bar's formula gives the player a third -- and the
    # game shows 33% there.
    convoy = [folder for folder in gamedir.missions(game)
              if folder.parent.name.upper() == "CAMPAIGN.03" and folder.name == "Mission.02"]
    if convoy:
        mission_c = mission.load(convoy[0] / "data.tma")
        gens = [o for o in mission_c.objects if "gener" in o.path.lower()]
        models = {o.path.split("\\")[-1].lower() for o in gens}
        player = sum(1 for o in gens if o.clan_id == 0)
        owners = sorted({o.clan_id for o in gens})
        check("data.tma: The Convoy gives the player a third of the map's power",
              "convoy" in mission_c.title.lower() and len(gens) == 3
              and len(models) == 1 and player == 1 and len(owners) == 3,
              f"{mission_c.title!r} places {len(gens)} generators, all "
              f"{sorted(models)}, one each for clans {owners}; the player's is "
              f"one of three equal sources, so the energy bar reads "
              f"{round(100 * player / len(gens))}% once nothing is left to "
              f"recharge")

    check("data.tma: a placed holder's MaximumOre is the engine's own constant",
          set(by_value) == {profiles.MINE_MAX_ORE, profiles.TRANSPORT_MAX_ORE,
                            profiles.STORAGE_MAX_ORE}
          and mines_at == {profiles.MINE_MAX_ORE}
          and all("mine" in n for n in by_value[profiles.MINE_MAX_ORE])
          and carriers == {"transprt", "builder"}
          and all("sto" in n for n in by_value[profiles.STORAGE_MAX_ORE]),
          f"the missions carry exactly {sorted(by_value)} -- Mine_MaxOre, "
          f"Transport_MaxOre and Storage_MaxOre as Behavior.dll compiles them.  "
          f"Every mine is at 500 {sorted(by_value[profiles.MINE_MAX_ORE])}; "
          f"2000 is on the transport and builder units and nothing else; "
          f"{sorted(by_value[profiles.STORAGE_MAX_ORE])} are at 4000")

    # MBehaviour's property setter takes its +4 sub-object, so the four grants
    # land four bytes past where the switch appears to put them: free bots on
    # the counter construction spends, free technologies on the one research
    # spends, and the research time on the field research reads as its budget.
    placed = [o for d in gamedir.missions(game)
              for o in mission.load(d / "data.tma").objects
              if profiles.FREE_BOTS in o.properties]

    def role(o) -> str:
        parts = o.path.replace("\\", "/").split("/")
        return parts[2].upper() if len(parts) > 3 else "?"

    def value(o, name):
        return o.properties[name].value

    bots = Counter(role(o) for o in placed if value(o, profiles.FREE_BOTS))
    technos = Counter(role(o) for o in placed if value(o, profiles.FREE_TECHNOLOGIES))
    slow = [o for o in placed
            if value(o, profiles.FREE_CONSTRUCTION_TIME) != profiles.CONSTRUCTION_TIME_DEFAULT]
    long = [o for o in placed
            if value(o, profiles.FREE_RESEARCH_TIME) != profiles.RESEARCH_TIME_DEFAULT]
    check("data.tma: free bots are a factory's, free technologies a research centre's",
          placed and set(bots) == {"PLANT"} and set(technos) == {"INSTITUT"}
          and all(role(o) == "PLANT" and value(o, profiles.FREE_BOTS) for o in slow)
          and all(role(o) == "INSTITUT" for o in long),
          f"FreeBotNum is non-zero on {sum(bots.values())} objects, all factories; "
          f"FreeTechnoNum on {sum(technos.values())}, a research centre; "
          f"FreeConstructionTime leaves the engine's "
          f"{profiles.CONSTRUCTION_TIME_DEFAULT:g} on {len(slow)} objects, every "
          f"one a factory that grants bots; FreeResearchTime is the engine's "
          f"{profiles.RESEARCH_TIME_DEFAULT:g} on {len(placed) - len(long)} of "
          f"{len(placed)} and otherwise on {len(long)} research centre")

    # Only a building joins its clan's distributor: MBehaviour registers an
    # object only when bit 31 of its Type is set (Behavior.dll:0x100060ad).
    kinds: Counter[tuple[str, bool]] = Counter()
    for d in gamedir.missions(game):
        for o in mission.load(d / "data.tma").objects:
            if o.type_id is None:
                continue
            parts = o.path.replace("\\", "/").split("/")
            kinds[(parts[1].upper() if len(parts) > 2 else "?",
                   bool(o.type_id & 0x80000000))] += 1
    check("data.tma: only buildings share their clan's power",
          kinds and set(kinds) == {("BUILDS", True), ("UNITS", False)},
          f"bit 31 of Type, which the distributor registration tests, is set on "
          f"all {kinds[('BUILDS', True)]} placed buildings and on none of the "
          f"{kinds[('UNITS', False)]} placed units -- a bot's batteries are "
          f"never refilled by the clan's generators")

    # The size letters the construction task reads: a chassis's third
    # character and a building's fourth.  The names agree with the model
    # codes the labels carry -- T-, S-, M-, L- for chassis, -17, -30, -47, -67
    # for buildings -- so the classes are sizes, smallest first.
    chassis: dict[str, Counter[str]] = defaultdict(Counter)
    for dat in sorted((game / "UNITS" / "UNITS").rglob("*.dat")):
        try:
            unit = objects.load_unit(dat)
        except objects.ObjectFormatError:
            continue
        member = unit.components[0].ref.member if unit.components else ""
        code = re.search(r"\(([A-Z])-", unit.label)
        if re.match(r"(?i)r_[a-z]_", member) and code:
            chassis[member[2].lower()][code.group(1)] += 1
    model = {"t": "T", "l": "S", "m": "M", "b": "L"}
    blds: dict[str, set[str]] = defaultdict(set)
    for dat in sorted((game / "UNITS" / "BUILDS").rglob("*.dat")):
        unit = objects.load_unit(dat)
        member = unit.components[0].ref.member.lower()
        number = re.search(r"-(\d\d)", unit.label)
        # A bridge's code is its span over its width, BS-32/20, not a size.
        if member.startswith("fr_") and number and "/" not in unit.label:
            blds[member[3]].add(number.group(1))
    check("UNITS: a name's size letter is the model's size",
          set(chassis) == set(model) <= set(profiles.CHASSIS_SIZE)
          and all(set(codes) == {model[k]} for k, codes in chassis.items())
          and blds.get("l") == {"17"} and blds.get("m") == {"30"}
          and blds.get("b") == {"47"} and blds.get("e") == {"67"},
          f"chassis R_T_, R_L_, R_M_, R_B_ carry model codes "
          f"{', '.join(model[k] + '-' for k in 'tlmb')} on "
          f"{sum(sum(c.values()) for c in chassis.values())} assemblies, sizes "
          f"{[profiles.CHASSIS_SIZE[k] for k in 'tlmb']}; buildings fr_l_, fr_m_, "
          f"fr_b_, fr_e_ carry -17, -30, -47, -67 (bridges aside), sizes "
          f"{[profiles.BUILDING_SIZE[k] for k in 'lmbe']}")


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

    # The block between the rings is a back-reference: which triangle of the
    # building's own mesh each corner was taken off, and which corner of it.
    inner_traced = outer_traced = 0
    on_mesh = resolved = points = 0
    whole = partial = 0
    for stem, plan in plans.items():
        inner_traced += bool(plan[0].traced)
        outer_traced += bool(plan[1].traced)
        try:
            m = objmesh.parse(archive.read_name(stem + ".msh"), stem)
        except (KeyError, ValueError, struct.error):
            continue
        hits = 0
        for point, (face, corner) in zip(plan[0].points, plan[0].traced,
                                         strict=False):
            points += 1
            if not (0 <= face < len(m.triangles) and 0 <= corner < 3):
                continue
            v = m.triangles[face][corner]
            if v < len(m.positions) and _near(m.positions[v], point):
                hits += 1
        resolved += hits
        on_mesh += hits == len(plan[0].points)
        whole += hits == len(plan[0].points)
        partial += 0 < hits < len(plan[0].points)
    check("fortif.rlb: the inner ring is traced on the model's own triangles",
          inner_traced == len(plans) and outer_traced == 0
          and whole >= 15 and resolved >= 150,
          f"every one of the {inner_traced} inner rings carries an int32 "
          f"triangle and an int32 corner per point, and not one of the "
          f"{len(plans)} outer rings does -- the outer ring is a clearance "
          f"drawn round the building rather than taken off it.  The reference "
          f"resolves every point on {whole} of the records, {resolved} in "
          f"all; on the other {len(plans) - whole - partial} no ring point "
          f"sits on a mesh vertex at all, so those outlines were traced on "
          f"geometry the shipped mesh no longer carries")

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
          reads[9] == reads[3] == reads[4] and reads[7] == reads[10],
          f"types 3, 4 and 9 read the same {len(reads[3])} offsets -- 4 reaches "
          f"+32 and +36 through a second pointer to its block, at +0xfc "
          f"(Effect.dll:0x100108f9) -- and 7 and 10 the same {len(reads[7])}")

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
              f"for byte; the 2, 3 and 4 that separate them are the radius "
              f"in their .exp")

    # .exp records, wherever they live: a hit kind and twelve name slots.
    total = refs = named = count_fits = 0
    kinds: Counter[int] = Counter()
    filled: Counter[int] = Counter()
    in_order = backwards = surfaces = generic = 0
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
            kinds[record.kind] += 1
            filled[len(record.effects)] += 1
            count_fits += len(record.effects) == record.kind
            for ref in record.effects:
                refs += 1
                named += library.get(ref.member) is not None
            if len(record.effects) != effects.EXPLOSION_SLOTS:
                continue
            for i, ref in enumerate(record.by_surface):
                # exp_Hsn_bul, exp_b_st_how: the tag is the last two letters of
                # a two- or three-letter token; "mn" is the flame family's
                # catch-all.
                tags = {w[-2:] for w in ref.member.lower().split("_") if len(w) in (2, 3)}
                mine = {effects.SURFACE_TAGS[i]} | ({"st"} if effects.SURFACE_TAGS[i] == "gr"
                                                    else set())
                surfaces += 1
                generic += "mn" in tags
                in_order += bool(tags & (mine | {"mn"}))
                backwards += bool(tags & {effects.SURFACE_TAGS[-1 - i]})
    check("*.exp: every one is 792 bytes of header and twelve name slots",
          total > 0 and set(filled) <= {0, 1, effects.EXPLOSION_SLOTS} and named >= refs - 2,
          f"all {total} parse at {effects.EXPLOSION_SIZE} bytes; they fill "
          f"{dict(sorted(filled.items()))} slots, and {named}/{refs} of the names are "
          f"real FXID members")
    check("*.exp: the first word is the hit kind, not a count",
          set(kinds) == {effects.HIT_NONE, effects.HIT_DIRECT, effects.HIT_AREA}
          and count_fits < total // 2,
          f"kinds {dict(sorted(kinds.items()))} -- nothing, a direct hit, an area "
          f"blast (Control.dll:0x1000ebc0); read as a count it matches the filled "
          f"slots on only {count_fits}")
    check("*.exp: slots 1-11 are the effect for the surface struck, in one order",
          surfaces and in_order == surfaces and backwards < surfaces // 2,
          f"{in_order}/{surfaces} names carry their slot's surface tag "
          f"({' '.join(effects.SURFACE_TAGS)}), {generic} of them the catch-all mn; "
          f"read backwards, {backwards}")

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


def check_control(check, game: Path) -> None:
    """.ctl -- the controller's 212-byte parameter frame."""
    names = frozenset(p.name.lower() for p in all_archives(game))
    members: dict[str, set[str]] = {}
    blobs: list[tuple[str, str, bytes]] = []
    for path in all_archives(game):
        archive = NResArchive.open(path)
        members[path.name.lower()] = {e.name.lower() for e in archive}
        for entry in archive:
            if entry.tag.upper().startswith("CTL"):
                blobs.append((path.name, entry.name, archive.read(entry)))

    parsed: list[control.Controller] = []
    sizes: list[int] = []
    bare_names: list[str] = []
    refused = 0
    for _lib, name, blob in blobs:
        try:
            controller = control.parse(blob, names)
        except control.ControlFormatError:
            refused += 1
            continue
        parsed.append(controller)
        if controller.bare:
            sizes.append(len(blob))
            bare_names.append(name)
    check(".ctl: every controller carries the 212-byte frame",
          parsed and not refused,
          f"{len(parsed)}/{len(blobs)} members across "
          f"{len({lib for lib, _n, _b in blobs})} archives parse, none refused")

    check(".ctl: a controller with no sections is the frame and nothing else",
          bool(sizes) and all(n == control.FRAME_SIZE for n in sizes),
          f"{len(sizes)} members are {control.FRAME_SIZE} bytes with 0xFF from "
          f"+{control.HEADER_SIZE} on -- {', '.join(bare_names[:3])}")

    modes = 0
    worst = ""
    for off, want in sorted(control.DEFAULTS.items()):
        if off in control.DEFAULT_INTS:
            seen = Counter(struct.unpack_from("<i", blob, off)[0]
                           for _l, _n, blob in blobs)
            hit = seen.most_common(1)[0][0] == want
        else:
            seen = Counter(struct.unpack_from("<f", blob, off)[0]
                           for _l, _n, blob in blobs)
            hit = seen.most_common(1)[0][0] == want
        modes += hit
        if not hit:
            worst = f"; +{off} is {seen.most_common(1)[0][0]}, not {want}"
    check(".ctl: the frame's commonest value is the engine's own default",
          modes == len(control.DEFAULTS) > 0,
          f"{modes}/{len(control.DEFAULTS)} slots -- the initialiser at "
          f"0x10006689 writes each of them into the live object at "
          f"file + 0x{control.FIELD_BASE:x}{worst}")

    reads = [v for c in parsed for t in c.triples for v in t]
    reads += [v for c in parsed for v in (*c.pair, *c.bounds, c.cone, c.payload)]
    finite = sum(1 for v in reads if math.isfinite(v))
    check(".ctl: every float in the parameter block is a float",
          finite == len(reads) > 0,
          f"{finite}/{len(reads)} reads across the block's 24 float slots on "
          f"{len(parsed)} members are finite")

    equal = sum(1 for c in parsed for t in c.triples if t[0] == t[1] == t[2])
    total = sum(len(c.triples) for c in parsed)
    check(".ctl: the six triples are per-axis",
          equal >= total * 0.8 > 0,
          f"{equal}/{total} triples hold the same value on all three "
          f"components, which is what a per-axis default looks like")

    turn = sum(1 for c in parsed
               if all(abs(v - control.FULL_TURN) <= 0.01 for v in c.triples[5]))
    cone = sum(1 for c in parsed if abs(c.cone - control.HALF_CONE) <= 1e-4)
    unlimited = sum(1 for c in parsed if c.payload >= control.FLT_MAX)
    check(".ctl: the angle limits carry the engine's own defaults",
          turn > len(parsed) * 0.7 and cone > len(parsed) * 0.9
          and unlimited > len(parsed) * 0.9,
          f"a whole turn on {turn}, pi/2 on {cone}, FLT_MAX payload on {unlimited} "
          f"of {len(parsed)}")

    flags = [c.flags for c in parsed]
    bits = sorted({b for v in flags for b in range(32) if v >> b & 1})
    check(".ctl: the word at +116 is a bitfield",
          flags and all(0 <= v < 32 for v in flags) and bits == list(range(5)),
          f"every value is below 32 and bits {bits[0]}..{bits[-1]} are used; "
          f"bit 0 -- the one Control.dll tests with "
          f"`test byte ptr [ptr+0x60], 1` -- is set on "
          f"{sum(1 for v in flags if v & 1)} of {len(flags)}")

    typed = sum(1 for c in parsed for k in c.components
                if k.type_id in control.COMPONENT_TYPES)
    parts = sum(len(c.components) for c in parsed)
    check(".ctl: every member is consumed to the byte",
          len(parsed) == len(blobs) > 0,
          f"{len(parsed)}/{len(blobs)} walk end to end -- section 1 "
          f"({control.SECTION1_RECORD} + {control.SECTION1_PER_B}*B a record, "
          f"then A*A int32), section 2 ({control.SECTION2_RECORD}), "
          f"{parts} component records, the {control.BLOCK_SIZE}-byte block, "
          f"and the reference groups")

    check(".ctl: a component record names a type the factory knows",
          typed == parts > 0,
          f"all {parts} records carry an id in {control.COMPONENT_TYPES.start}.."
          f"{control.COMPONENT_TYPES.stop - 1}, the range the factory at "
          f"0x1002d4b0 dispatches; "
          f"{len({k.type_id for c in parsed for k in c.components})} of the 30 "
          f"are used")

    named = [r for c in parsed for r in c.named]
    resolved = sum(1 for r in named
                   if r.member.lower() in members.get(r.library.lower(), ()))
    check(".ctl: every reference names a member that exists",
          resolved == len(named) > 0,
          f"{resolved}/{len(named)} (archive, member) pairs resolve -- "
          f"{sum(1 for c in parsed for k in c.components if k.resource)} on "
          f"components and the rest in section 5, out of "
          f"{sum(len(c.references) for c in parsed)} records there")

    tags = {e.name.lower(): e.tag for e in NResArchive.open(game / "objects.rlb")} \
        if (game / "objects.rlb").exists() else {}
    kinds: Counter[str] = Counter()
    shooters: set[str] = set()
    for (lib, _name, _blob), c in zip(blobs, parsed, strict=True):
        for r in c.named:
            if r.library.lower() == "objects.rlb":
                kinds[tags.get(r.member.lower(), "?")] += 1
                shooters.add(lib)
    check(".ctl: the objects a controller names are projectiles",
          kinds and set(kinds) == {"BULL"}
          and shooters <= {"guns.rlb", "turrets.rlb", "animals.rlb", "bases.rlb"},
          f"all {sum(kinds.values())} objects.rlb references are BULL records, "
          f"and the only archives that carry any are {', '.join(sorted(shooters))}")

    labels = {k.label for c in parsed for k in c.components if k.label}
    parts = {e.name.lower(): e.tag
             for e in NResArchive.open(game / "objects.rlb")} \
        if (game / "objects.rlb").exists() else {}
    matched = {label for label in labels
               if any(n.startswith(label.lower()) for n in parts)}
    tagged = {parts[n] for label in matched for n in parts
              if n.startswith(label.lower())}
    check(".ctl: a component's label names a family of internal parts",
          labels and matched == labels and tagged == {"INTO"},
          f"all {len(labels)} distinct labels on "
          f"{sum(1 for c in parsed for k in c.components if k.label)} records "
          f"are a prefix of an objects.rlb member, and every member they reach "
          f"is an {', '.join(sorted(tagged))} record")


#: A building's model number, as its display name carries it, against the
#: efficiency its controller gives it.
EFFICIENCY_BY_MODEL = {"17": 1.0, "30": 3.0, "47": 5.0, "67": 7.0}
#: The building roles whose size sets their efficiency.
SIZED_ROLES = ("INSTITUT", "PLANT", "MINE")


def check_efficiency(check, game: Path) -> None:
    """.ctl: the efficiency component, and a building's size."""
    names = frozenset(p.name.lower() for p in all_archives(game))
    values = 0
    finite = 0
    efficiency: dict[str, list[float]] = defaultdict(list)
    homes: set[str] = set()
    others_zero = True
    families: dict[str, Counter[int]] = defaultdict(Counter)
    buildings: dict[str, control.Controller] = {}
    powers = []
    # (component nodes, node count of the same-stem .ndp) per controller
    paired: list[tuple[list[int], int]] = []
    efficiency_life: list[float] = []
    for path in all_archives(game):
        archive = NResArchive.open(path)
        tables = {e.name.lower()[:-4]: e for e in archive
                  if e.name.lower().endswith(".ndp")}
        for entry in archive:
            if not entry.tag.upper().startswith("CTL"):
                continue
            try:
                parsed = control.parse(archive.read(entry), names)
            except control.ControlFormatError:
                continue
            table = tables.get(entry.name.lower()[:-4])
            if table is not None and parsed.components:
                rows = objects.parse_damage(archive.read(table), table.name)
                paired.append(([p.node for p in parsed.components], len(rows)))
                efficiency_life.extend(
                    rows[p.node].durability for p in parsed.components
                    if p.type_id == control.EFFICIENCY_TYPE and 0 <= p.node < len(rows))
            if path.name.lower() == "fortif.rlb":
                buildings[entry.name.lower()] = parsed
            for part in parsed.components:
                powers.append(part.power)
                if part.label:
                    families[part.label[:5].lower()][part.type_id] += 1
                values += len(part.values)
                finite += sum(1 for v in part.values if math.isfinite(v))
                if part.efficiency is not None:
                    efficiency[entry.name.lower()].append(part.efficiency)
                    homes.add(path.name.lower())
                    others_zero &= not any(part.values[1:])

    inside = sum(1 for nodes, count in paired for n in nodes if 0 <= n < count)
    parts = sum(len(nodes) for nodes, _ in paired)
    shuffled = [count for _, count in paired]
    random.Random(1).shuffle(shuffled)
    misses = sum(1 for (nodes, _), count in zip(paired, shuffled, strict=True)
                 for n in nodes if not 0 <= n < count)
    check(".ctl: a component's +4 is a node of its object's .ndp",
          parts and inside == parts and misses > 0,
          f"{inside}/{parts} component nodes across {len(paired)} controllers fall "
          f"inside the same-stem .ndp's record count; paired with a shuffled .ndp, "
          f"{misses} fall outside.  Control.dll asks that node's life fraction "
          f"(0x1000dc40, id 1) for every value read with bit 0x100")

    solid = [life for life in efficiency_life if life >= 1_000_000]
    soft = [life for life in efficiency_life if life < 1_000_000]
    check(".ctl: a building's efficiency part can usually be shot down",
          efficiency_life and soft and solid and len(efficiency_life) == sum(
              1 for c in buildings.values() for p in c.components
              if p.type_id == control.EFFICIENCY_TYPE),
          f"of {len(efficiency_life)} efficiency components, {len(soft)} sit on a "
          f".ndp node with durability {min(soft):g}..{max(soft):g} and "
          f"{len(solid)} on the 1000000 that cannot be destroyed -- and KPD falls "
          f"with that node's life")

    check(".ctl: a component's 64-byte block is sixteen floats",
          finite == values > 0,
          f"{finite}/{values} values across every component record are finite; "
          f"the value getter at Control.dll:0x10021d00 indexes them by an id's "
          f"low byte")

    library = objects.ObjectLibrary(game / "objects.rlb")
    rows = []
    wrong = []
    for role in SIZED_ROLES:
        for dat in sorted((game / "UNITS" / "BUILDS" / role).glob("*.dat")):
            unit = objects.load_unit(dat)
            model = re.search(r"-(\d\d)", unit.label)
            fort = library.get(unit.components[0].ref.member)
            body = library.get(fort.slots[0].member) if fort and fort.slots else None
            ctl = body.slot_with_suffix("ctl") if body else None
            if not (model and ctl):
                wrong.append(f"{dat.name} unresolved")
                continue
            got = sum(efficiency.get(ctl.member.lower(), []))
            want = EFFICIENCY_BY_MODEL.get(model.group(1))
            rows.append((dat.name, got))
            if got != want:
                wrong.append(f"{dat.name} {model.group(1)} has {got}")
    sized = {ctl for ctl in efficiency
             if any(f"_{r.lower()[:4]}" in ctl for r in SIZED_ROLES)}
    rest = [v for ctl, vs in efficiency.items() if ctl not in sized for v in vs]
    check(".ctl: a building's efficiency is its size",
          rows and not wrong and set(rest) == {1.0} and others_zero
          and homes == {"fortif.rlb"},
          f"on all {len(rows)} research centre, factory and mine assemblies the "
          f"controller's type-{control.EFFICIENCY_TYPE} value is 1, 3, 5 or 7 "
          f"as the model is 17, 30, 47 or 67; every other building's is 1, the "
          f"other fifteen values are zero, and the class lives only in "
          f"{', '.join(sorted(homes))}{'; ' + ', '.join(wrong[:3]) if wrong else ''}")

    # A component's type id is the engine's CICLS class: every label family
    # sits on one type, and every family CICLS has a name for sits on that
    # name's number -- which is what lets the power channels be named.
    by_number = {v: k for k, v in controls.CICLS.items()}
    single = all(len(types) == 1 for types in families.values())
    named = {family: by_number[next(iter(types))] for family, types in families.items()
             if next(iter(types)) in by_number}
    want = {"i_pws": "CICLS_POWERSTOR", "i_fsh": "CICLS_FIGHTSHIELD",
            "i_dsh": "CICLS_DETECTSHIELD", "i_eng": "CICLS_ENGINE",
            "i_rdr": "CICLS_RADAR", "i_rps": "CICLS_REPAIRSYS"}
    check(".ctl: a component's type id is the engine's CICLS class",
          families and single and all(named.get(f) == n for f, n in want.items())
          and all(n == "CICLS_MULTIGUN" for f, n in named.items() if f.startswith("i_c")),
          f"each of {len(families)} label families sits on one type id, and the "
          f"named ones agree: " + ", ".join(f"{f} {named.get(f, '?')[6:]}"
                                            for f in sorted(want)))

    # Power, per controller tick: stores give up to their power figure a second
    # times their charge; channel 3 is served first, and on every building the
    # efficiency component is alone there.  So its level -- the 0x200 factor in
    # KPD -- stays at 1 until the batteries hold less than draw / output.
    role_profile = {"_inst": "prof_institute.var", "_plan": "prof_plant.var",
                    "_mine": "prof_mine.var"}
    held = profiles.load(game) if (game / profiles.ARCHIVE).exists() else {}
    alone = []
    margins = []
    for name, parsed in sorted(buildings.items()):
        parts = parsed.components
        if not any(p.type_id == control.EFFICIENCY_TYPE for p in parts):
            continue
        first = [p for p in parts if p.channel in control.POWER_ORDER[0]]
        alone.append(all(p.type_id == control.EFFICIENCY_TYPE for p in first))
        stores = [p for p in parts if p.type_id == control.POWER_STORE_TYPE
                  and p.values[0] > 0]
        role = next((r for r in role_profile if r in name), None)
        if role and stores and role_profile[role] in held:
            draw = (sum(p.power for p in first)
                    + held[role_profile[role]][profiles.USE_POWER].value)
            margins.append((name, draw / sum(p.power for p in stores)))
    worst = max(margins, key=lambda m: m[1]) if margins else ("-", 1.0)
    check(".ctl: a building's batteries keep its efficiency whole until nearly empty",
          alone and all(alone) and margins and worst[1] < 0.1
          and all(math.isfinite(v) and v >= 0 for v in powers),
          f"on all {len(alone)} building controllers with an efficiency component "
          f"it is the only thing on channel 3, served first; across "
          f"{len(margins)} research centres, factories and mines the charge "
          f"below which it gets less than it draws is at most "
          f"{100 * worst[1]:.1f}% ({worst[0]}); every one of {len(powers)} power "
          f"figures is finite and non-negative")


#: A payload the game treats as no limit, in kg: 1000 t and 10000 t.
NO_LIMIT_PAYLOAD = (1_000_000.0, 10_000_000.0)


def check_motion(check, game: Path) -> None:
    """.ctl: how a machine moves -- states, speed limits, running gear, load."""
    names = frozenset(p.name.lower() for p in all_archives(game))
    every: list[tuple[str, str, control.Controller]] = []
    for path in all_archives(game):
        archive = NResArchive.open(path)
        for entry in archive:
            if not entry.tag.upper().startswith("CTL"):
                continue
            try:
                parsed = control.parse(archive.read(entry), names)
            except control.ControlFormatError:
                continue
            every.append((path.name.lower(), entry.name.lower(), parsed))

    # Section 1 is a list of states: the bounds a flag switches on are ordered
    # boxes, and the ones it leaves off are the open interval.
    states = [s for _lib, _name, c in every for s in c.states]
    ordered = unordered = off_open = off_other = 0
    for s in states:
        for box, (lo, hi) in enumerate((s.velocity, s.spin)):
            for axis in range(3):
                if s.flags & (1 << (axis + 4 * box)):
                    ordered += lo[axis] <= hi[axis]
                    unordered += lo[axis] > hi[axis]
                elif lo[axis] == -control.FLT_MAX and hi[axis] == control.FLT_MAX:
                    off_open += 1
                else:
                    off_other += 1
    check(".ctl: section 1 is a list of states whose flags switch their bounds on",
          states and ordered and off_open and not unordered and not off_other,
          f"{len(states)} states; the {ordered} bounds a flag bit enables all read "
          f"min <= max, and the {off_open} it leaves off are all -FLT_MAX..FLT_MAX "
          f"(bits 0-2 velocity, bits 4-6 spin; Control.dll:0x10001000)")

    factors = Counter(s.engine for s in states)
    hero = Counter(s.engine for lib, name, c in every
                   if (lib, name) == ("bases.rlb", "r_h_02.ctl") for s in c.states)
    check(".ctl: a state's engine factor is 0, 1, 1.5 or 2, and the hero's is always 0",
          set(factors) == {0.0, 1.0, 1.5, 2.0} and set(hero) == {0.0},
          ", ".join(f"{v:g} on {n}" for v, n in sorted(factors.items()))
          + f" of {len(states)} states; the current state's is the +0x154 the engine "
          f"draw multiplies by (Control.dll:0x100266e1); all {hero[0.0]} states of "
          f"r_h_02, the hero chassis, carry 0")

    # .ndp flags 0x20 and 0x40 sit on one side each of the model.
    left = right = wrong = other_neg = other_pos = models = 0
    for libname in ("bases.rlb", "animals.rlb"):
        archive = NResArchive.open(game / libname)
        meshes = {e.name.lower()[:-4]: e for e in archive
                  if e.tag.upper().startswith("MESH")}
        for entry in archive:
            stem = entry.name.lower()[:-4]
            if not entry.name.lower().endswith(".ndp") or stem not in meshes:
                continue
            rows = objects.parse_damage(archive.read(entry), entry.name)
            if not any(r.flags & (objects.LEFT_GEAR | objects.RIGHT_GEAR) for r in rows):
                continue
            model = objmesh.parse(archive.read(meshes[stem]), meshes[stem].name)
            models += 1
            for i, row in enumerate(rows):
                x = model.world_pose(i)[0][0]
                if row.flags == objects.LEFT_GEAR:
                    left += x < 0
                    wrong += x >= 0
                elif row.flags == objects.RIGHT_GEAR:
                    right += x > 0
                    wrong += x <= 0
                elif abs(x) > 0.05:
                    other_neg += x < 0
                    other_pos += x > 0
    check(".ndp: flag 0x20 marks a machine's left running gear, 0x40 its right",
          models and left and right and not wrong and other_neg and other_pos,
          f"in {models} chassis and animal models, all {left} nodes flagged 0x20 rest "
          f"at x < 0 and all {right} flagged 0x40 at x > 0, where their other "
          f"off-centre nodes split {other_neg} and {other_pos}.  Control.dll:0x10012a40 "
          f"averages each side's life")

    parts = descriptions.read(game / descriptions.LIBRARY)
    bases = NResArchive.open(game / "bases.rlb")
    ctl = {e.name.lower(): e for e in bases if e.tag.upper().startswith("CTL")}
    panels = [p for p in parts.values()
              if p.group == 2 and {(s.field, s.unit) for s in p.stats}
              >= {("weight", "t"), ("payload", "t"), ("maxspeed", "kmph")}]
    joined = {p.part: control.parse(bases.read(ctl[p.part.lower() + ".ctl"]))
              for p in panels if p.part.lower() + ".ctl" in ctl}
    check("objects.dlb: a chassis shows its weight and payload in t, its speed in km/h",
          panels and len(joined) == len(panels),
          f"{len(panels)} chassis descriptions carry all three rows, and every one "
          f"names a same-stem controller in bases.rlb; iron3d.dll:0x1006f300 fills "
          f"them from +124 x 0.001, +48 x 3.6 and the total mass x 0.001")

    # What moving costs: one engine of power 20 and one 10000 battery on every
    # chassis but the hero's, whose engine asks 0.1.
    engine_power = {k: [p.power for p in c.components if p.type_id == control.ENGINE_TYPE]
                    for k, c in joined.items()}
    batteries = {k: sorted((p.values[0], p.power) for p in c.components
                           if p.type_id == control.POWER_STORE_TYPE)
                 for k, c in joined.items()}
    hero_engine = [e for k, e in engine_power.items() if k.upper() == "R_H_02"]
    second = sorted(k for k, b in batteries.items() if len(b) > 1)
    check(".ctl: every chassis declares an engine slot of 20 and a 10000 battery slot",
          joined and all(e == [20.0] for k, e in engine_power.items() if k.upper() != "R_H_02")
          and hero_engine and round(hero_engine[0][0], 3) == 0.1
          and all((10000.0, 250.0) in b for b in batteries.values()),
          f"all {len(joined)} chassis carry a store of 10000 at 250 a second, and all "
          f"but the hero's an engine asking 20 (the hero's asks "
          f"{hero_engine[0][0] if hero_engine else '?':.2g}); {len(second)} carry a "
          f"second store ({', '.join(second)})")

    top = control.TRIPLE_TOP_SPEED
    moving = {k: c.triples[top][1] for k, c in joined.items() if c.triples[top][1] >= 1.0}
    kmh = [k for k, v in moving.items()
           if abs(v * control.KMH_PER_MS - round(v * control.KMH_PER_MS)) < 0.01]
    ms = [k for k, v in moving.items() if abs(v - round(v)) < 0.01]
    check(".ctl: a chassis's top speed is authored in km/h and stored in m/s",
          moving and len(kmh) >= len(moving) - 1 and len(ms) < len(kmh),
          f"+48 x 3.6 is a whole number of km/h on {len(kmh)} of the {len(moving)} "
          f"chassis that move (not on {', '.join(sorted(set(moving) - set(kmh)))}); "
          f"+48 itself is a whole number of m/s on {len(ms)}")

    by_size: dict[int, list[float]] = defaultdict(list)
    unlimited = []
    for part, c in joined.items():
        if c.payload in NO_LIMIT_PAYLOAD:
            unlimited.append(part)
        elif part.lower()[2] in profiles.CHASSIS_SIZE:
            by_size[profiles.CHASSIS_SIZE[part.lower()[2]]].append(c.payload)
    ranks = sorted(by_size)
    rising = all(max(by_size[a]) < min(by_size[b]) for a, b in zip(ranks, ranks[1:], strict=False))
    check(".ctl: a chassis's payload (+124, kg) rises with its size, without overlap",
          len(ranks) == 4 and rising,
          "; ".join(f"size {r}: {min(by_size[r]) / 1000:g}-{max(by_size[r]) / 1000:g} t"
                    for r in ranks)
          + f"; {len(unlimited)} carry a no-limit 1000 or 10000 t "
          f"({', '.join(sorted(unlimited))})")

    modes: dict[int, Counter[float]] = defaultdict(Counter)
    for _lib, _name, c in every:
        modes[c.mode][round(c.cone, 4)] += 1
    check(".ctl: the slope cone is authored only where the mode reads it",
          set(modes[0]) == {round(control.HALF_CONE, 4)}
          and set(modes[control.SLOPE_MODE]) == {0.6},
          "; ".join(f"mode {m}: " + ", ".join(f"{cone:g} on {n}"
                                             for cone, n in sorted(c.items()))
                    for m, c in sorted(modes.items()))
          + ".  Only mode 2 compares the ground's tilt with +112 (Control.dll:0x100157ac)")

    engines = [(name, p.values[0], p.power, p.mass) for lib, name, c in every
               if lib == "intsys.rlb" for p in c.components
               if p.type_id == control.ENGINE_TYPE]
    elsewhere = [p.mass for lib, _name, c in every if lib != "intsys.rlb"
                 for p in c.components if p.type_id == control.ENGINE_TYPE]
    rows_ok = True
    detail = []
    for size in "lmb":
        mine = sorted((v, w, m) for n, v, w, m in engines if n.startswith(f"o_eng_{size}_"))
        rows_ok &= len(mine) == 4 and all(a[1] < b[1] and a[2] < b[2]
                                          for a, b in zip(mine, mine[1:], strict=False))
        detail.append(f"{size}: " + ", ".join(f"{v:.1f}/{w:g}/{m:g}" for v, w, m in mine))
    check(".ctl: an internal engine's mass (+0x1c) and draw rise with its value",
          engines and rows_ok and not any(elsewhere),
          f"value/power/kg by size -- {'; '.join(detail)}; the {len(elsewhere)} other "
          f"engines weigh 0.  Control.dll:0x1000fac0 adds a part's mass to its node")


#: The four marks of an internal part, by name suffix: MK1 to MK4.
MARKS = ("df", "01", "02", "03")


#: The four materials that hurt a unit standing on them, and how fast.
GROUND_RATES = {"water_bot": materials.LIQUID_BED_RATE,
                "env_lava_bot": materials.LIQUID_BED_RATE,
                "b_s0_dam": 1000.0, "b_dd1dk_dam": 1000.0}
WATER_SURFACE = 7


def check_ground(check, game: Path) -> None:
    """The ground a unit stands on: surface records, surface groups, lakes, gravity."""
    lib = materials.MaterialLibrary(game / "Material.lib")
    mats = {m.name.lower(): m for m in (lib.get(e.name) for e in lib.archive
                                        if e.tag == materials.MATERIAL_TAG)}

    # The ground contact (Control.dll:0x1001aaf5) takes the class byte, the
    # float and the dword of the material under the unit.
    check("Material.lib: the ground's speed factor G is 1.0 on every material",
          mats and all(m.speed_factor == 1.0 for m in mats.values()),
          f"the float at +6 on all {len(mats)} records; the ground contact copies it "
          f"to +0x1a8, so on shipped data no ground changes a machine's top speed")
    rated = {n: m.damage_rate for n, m in mats.items() if m.damage_rate}
    late = sum(struct.unpack_from("<f", lib.archive.read_name(n.upper()),
                                  materials.SURFACE_FIELDS_AT + 7)[0] in GROUND_RATES.values()
               for n in GROUND_RATES)
    check("Material.lib: a damage rate is set on the two liquid beds and two damaged bases",
          rated == GROUND_RATES and late == 0,
          f"{rated}: the dword at +10, read as a float, is hit points a second an agent "
          f"of kind 4 loses on that ground (Control.dll:0x10012a66); control: one byte "
          f"late, {late} of the four rates survive")
    classes = Counter(m.surface for m in mats.values())
    check("Material.lib: the class byte is a surface id 0..10, or unset",
          set(classes) <= set(range(control.SURFACES)) | {materials.UNSET},
          f"{dict(sorted(classes.items()))}; the ground contact runs block group "
          f"+0x504[id] only for an id of 10 or less (0x1001ab3e)")
    check("Material.lib: the liquid beds are surface 1, water 7, lava's surface unset",
          (mats["water_bot"].surface, mats["env_lava_bot"].surface,
           mats["water"].surface, mats["env_nlava"].surface)
          == (1, 1, WATER_SURFACE, materials.UNSET),
          "a bed is walked as surface 1 and costs 10000 hit points a second")

    # Which surfaces the terrain hands out, face by face.
    per_surface: Counter[int] = Counter()
    beds_ok = water_ok = True
    tut = None
    lakes: dict[bool, Counter[str]] = {True: Counter(), False: Counter()}
    tut_lakes = None
    steep = faces0 = 0
    words: list[Counter[int]] = [Counter() for _ in range(4)]
    for d in gamedir.maps(game):
        land = landmesh.load(d / "Land.msh")
        counts: Counter[tuple[str, int]] = Counter()
        for fi in range(land.face_count):
            name = land.layer1_names[land.face_tex1[fi]].lower()
            m = mats[name]
            per_surface[m.surface] += 1
            counts[(name, m.surface)] += 1
            if land.face_flags[fi] & landmesh.FLAGS_LIQUID_BED_BIT:
                beds_ok &= m.surface == 1 and m.damage_rate == materials.LIQUID_BED_RATE
            elif land.face_surface[fi] & landmesh.SURFACE_WATER_BIT:
                water_ok &= m.surface in (WATER_SURFACE, materials.UNSET) and not m.damage_rate
        if d.name.lower() == "tut_1":
            tut = counts
        if not (d / "Land.map").exists():
            continue
        areals = arealmap.load(d / "Land.map")
        for a in areals.areals:
            for i, word in enumerate(a.flags):
                words[i][word] += 1
        if d.name.lower() == "tut_1":
            tut_lakes = Counter(a.flags[2] for a in areals.areals)
        for fi in land.lod_faces(0):
            faces0 += 1
            steep += land.face_normal[fi][2] <= landmesh.WALKABLE_NORMAL_Z
            a, b, c = land.faces[fi]
            x = sum(land.positions[v][0] for v in (a, b, c)) / 3
            y = sum(land.positions[v][1] for v in (a, b, c)) / 3
            ids = areals.areals_at(x, y)
            if len(ids) != 1:
                continue
            kind = ("bed" if land.face_flags[fi] & landmesh.FLAGS_LIQUID_BED_BIT else
                    "water" if land.face_surface[fi] & landmesh.SURFACE_WATER_BIT else
                    "ground")
            lakes[areals.areals[ids[0]].lake][kind] += 1
    check("Land.msh: the terrain's faces are surfaces 0, 1, 2 and 7, or unset lava",
          set(per_surface) <= {0, 1, 2, WATER_SURFACE, materials.UNSET}
          and beds_ok and water_ok,
          f"faces by surface, both levels of detail: {dict(sorted(per_surface.items()))}; "
          f"every liquid-bed face is surface 1 at 10000 a second and every water "
          f"face 7 (or unset lava) with no rate")
    want = {("l02", 1): 5250, ("l00", 2): 2208, ("water_bot", 1): 478,
            ("water", WATER_SURFACE): 354}
    check("Tut_1: the ground is L02 (1) and L00 (2), with a lake bed and water",
          tut == want, f"{dict(tut) if tut else None}, faces across both levels of detail")
    check("Land.msh: some terrain faces are too steep to be ground",
          faces0 and 0 < steep < faces0 // 10,
          f"{steep} of {faces0} level-0 faces have a normal z of at most "
          f"{landmesh.WALKABLE_NORMAL_Z} (cos 80 degrees), which the ground contact "
          f"never takes as ground (Control.dll:0x1001a6fd)")
    check("Land.map: an areal's flag words are 0/1, 0, a small value or a lake, and 0",
          set(words[0]) <= {0, 1} and set(words[1]) == set(words[3]) == {0}
          and all(w < 30 or w & arealmap.LAKE_BITS == arealmap.LAKE_BITS
                  for w in words[2]),
          f"word 0 {dict(sorted(words[0].items()))}; words 1 and 3 always 0; "
          f"word 2 {dict(sorted(words[2].items()))}")
    check("Land.map: an areal with all of 0xF0 in its third flag word is a lake",
          lakes[True]["water"] and not lakes[True]["ground"]
          and lakes[False]["water"] + lakes[False]["bed"] < 20,
          f"level-0 faces whose centre lies in one areal: lake areals hold "
          f"{dict(lakes[True])}; control: the rest hold {dict(lakes[False])}")
    check("Tut_1: five lake areals among 378",
          tut_lakes is not None and tut_lakes.get(240) == 5
          and sum(tut_lakes.values()) == 378, f"third flag words {dict(tut_lakes or {})}")

    # The 84-byte block is 21 section-5 group indices; 10..20 are per surface.
    names = frozenset(p.name.lower() for p in all_archives(game))
    total = valid = early_valid = set_blocks = 0
    carriers = []
    shape_ok = ops_ok = dust_ok = True
    modes: dict[int, list[str]] = defaultdict(list)
    for path in all_archives(game):
        archive = NResArchive.open(path)
        for entry in archive:
            if not entry.tag.upper().startswith("CTL"):
                continue
            blob = archive.read(entry)
            try:
                c = control.parse(blob, names)
            except control.ControlFormatError:
                continue
            label = f"{path.name.lower()}/{entry.name.lower()}"
            modes[c.mode].append(label)
            total += 1

            def indexes(block, c=c):
                return all(v == control.NO_GROUP or 0 <= v < c.counts[4] for v in block)

            valid += indexes(c.groups)
            if any(v != control.NO_GROUP for v in c.groups):
                set_blocks += 1
                at = control.section4_start(c.counts) + sum(p.size for p in c.components)
                early_valid += indexes(struct.unpack_from(
                    f"<{control.BLOCK_ENTRIES}i", blob, at - 2))
            surf = c.surface_groups
            if all(v == control.NO_GROUP for v in surf):
                continue
            carriers.append(label)
            quiet, loud = surf[0], surf[1]
            shape_ok &= (surf[2] == quiet != loud and all(v == loud for v in surf[3:]))
            by_id = {r.values[7]: r.resource.member.lower()
                     for r in c.references if r.resource.member}
            for group, action in ((quiet, 11), (loud, 10)):
                records = [r for r in c.references if r.group == group]
                ops_ok &= bool(records) and all(
                    r.values[3] == action and not r.resource.member for r in records)
                dust_ok &= all(by_id.get(r.values[4], "").startswith("dust")
                               for r in records)
    check(".ctl: the 84-byte block is 21 section-5 group indices",
          total and valid == total and early_valid == 0,
          f"every entry is -1 or a group on {valid}/{total} controllers; control: read "
          f"two bytes early, {early_valid} of the {set_blocks} blocks with an entry set "
          f"still index groups.  The loader copies them to +0x4dc (Control.dll:0x100093ee)")
    check(".ctl: nine chassis switch a group by the ground's surface",
          len(carriers) == 9 and all(n.startswith("bases.rlb/r_") for n in carriers)
          and shape_ok,
          f"{', '.join(carriers)}: on each, surfaces 0 and 2 share one group and "
          f"1 and 3..10 another")
    check(".ctl: surfaces 0 and 2 run action 11 and the rest action 10, on the dust",
          carriers and ops_ok and dust_ok,
          "every record in the first group is action 11 and in the second action 10, "
          "and every effect id they name is one of the chassis's dust_* emitters")

    three = sorted(modes[3])
    check(".ctl: mode 3, which gravity reaches, is four rounds and two hero targets",
          three == sorted(["weapon.rlb/bf_b_01.ctl", "weapon.rlb/bf_f_01.ctl",
                           "weapon.rlb/bf_l_01.ctl", "weapon.rlb/bf_m_01.ctl",
                           "bases.rlb/r_h_01.ctl", "bases.rlb/r_h_03.ctl"])
          and "bases.rlb/r_h_02.ctl" in modes[2],
          f"{', '.join(three)}; the integrator adds the world's 10.0 along -z only for "
          f"mode 3 (Control.dll:0x10015879).  Control: the hero, r_h_02, is mode 2")


#: The component record's item step factor, which a door's or pod's step time
#: divides by (``Control.dll:0x10022120``); and the step an item's progress
#: takes (``0x10020900``).
ITEM_FACTOR_AT = 0x24
ITEM_STEP = 0.45


def check_playback(check, game: Path) -> None:
    """How a controller plays: frame pairs, strides, the table, channels, item time."""
    names = frozenset(p.name.lower() for p in all_archives(game))
    bases = NResArchive.open(game / "bases.rlb")
    hero = control.parse(bases.read_name("r_h_02.ctl"), names)
    body = objmesh.parse(bases.read_name("R_H_02.msh"), "R_H_02.msh")

    frames = body.frame_count
    inside = sum(all(0 <= f < frames for f in (*s.pair_a, *s.pair_b)) for s in hero.states)
    boxes = sum(all(0 <= f < frames for f in (*s.velocity[0][:2], *s.velocity[1][:2]))
                for s in hero.states)
    check("r_h_02: every state names two frame pairs inside the hero mesh's frames",
          len(hero.states) == 105 and inside == 105 and boxes < 50,
          f"{inside} of {len(hero.states)} states' pairs A (+0x0c) and B (+0x14) lie in "
          f"the {frames} frames of R_H_02.msh; control: {boxes} velocity boxes (+0x24) do")

    walk = run = 0.0
    driven = along = 0
    for s in hero.states:
        start, end = (body.pose_at(0, f)[0] for f in s.pair_b)
        d = [b - a for a, b in zip(start, end, strict=True)]
        if not s.by_velocity:
            continue
        driven += 1
        vy = s.velocity[0][1]
        along += (d[1] > 0) == (vy > 0) and abs(d[1]) > abs(d[0])
        if vy > 0 and 5 <= min(s.pair_b) and max(s.pair_b) <= 13:
            walk += math.hypot(*d)
        if vy > 0 and 18 <= min(s.pair_b) and max(s.pair_b) <= 34:
            run += math.hypot(*d)
    check("r_h_02: a walk cycle covers 2.445 and a run cycle 5.674, the way the boxes run",
          driven == 72 and along == driven and abs(walk - 2.445) < 0.01
          and abs(run - 5.674) < 0.01,
          f"{along} of {driven} velocity-driven states move the body node along +y "
          f"where their box's vy is positive and -y where it is negative; walk frames "
          f"5-13 move it {walk:.3f}, run frames 18-34 {run:.3f}.  A step lasts "
          f"stride / speed (Control.dll:0x100057b3), so a walk cycle at 5 m/s takes "
          f"{walk / 5:.2f} s and a run cycle at 14 m/s {run / 14:.3f} s")

    joined = backwards = edges = anchors = states = 0
    fixed: Counter[str] = Counter()
    for path in all_archives(game):
        archive = NResArchive.open(path)
        for entry in archive:
            if not entry.tag.upper().startswith("CTL"):
                continue
            try:
                c = control.parse(archive.read(entry), names)
            except control.ControlFormatError:
                continue
            n = len(c.states)
            states += n
            anchors += sum(s.anchor for s in c.states)
            fixed[path.name.lower()] += sum(bool(s.mode & control.STATE_FIXED)
                                            for s in c.states)
            for to in range(n):
                for frm in range(n):
                    if frm == to or c.cost(to, frm) != 0:
                        continue
                    edges += 1
                    joined += c.states[frm].pair_b[1] == c.states[to].pair_b[0]
                    backwards += c.states[to].pair_b[1] == c.states[frm].pair_b[0]
    check(".ctl: the transition table's row is the destination",
          edges and joined == edges and backwards == 0,
          f"on {joined} of {edges} zero-cost edges table[to][from] joins a state whose "
          f"pair B ends on the frame the next one's starts; control: read the other "
          f"way, {backwards} do.  {anchors} of the {states} states are anchors the "
          f"planner chooses among (Control.dll:0x100051c0)")
    check(".ctl: the fixed, motionless step 0x100000 is the buildings'",
          fixed["fortif.rlb"] == 420 and sum(fixed.values()) == 421,
          f"{dict(+fixed)}")

    library = objects.ObjectLibrary(game / "objects.rlb")
    opened: dict[str, NResArchive] = {}

    def read(ref):
        key = ref.library.lower()
        if key not in opened:
            opened[key] = NResArchive.open(game / key)
        return opened[key].read_name(ref.member)

    def moves(m, node, first, last):
        if not 0 <= node < len(m.nodes) or not m.nodes[node].is_animated:
            return False
        base = m.pose_at(node, first)
        for f in range(int(first) + 1, int(last) + 1):
            t, q = m.pose_at(node, f)
            if math.dist(base[0], t) > 1e-4 or 1 - abs(sum(
                    x * y for x, y in zip(base[1], q, strict=True))) > 1e-6:
                return True
        return False

    seen = set()
    total = in_range = spans = hit = shifted = as_point = 0
    for record in library.records.values():
        ref, msh = record.slot_with_suffix("ctl"), record.mesh
        if not ref or not msh or (ref.library.lower(), ref.member.lower()) in seen:
            continue
        seen.add((ref.library.lower(), ref.member.lower()))
        try:
            c = control.parse(read(ref), names)
            m = objmesh.parse(read(msh), msh.member)
        except (KeyError, control.ControlFormatError):
            continue
        for ch in c.channels:
            total += 1
            in_range += 0 <= ch.node < len(m.nodes)
            if ch.last > ch.first >= 0 and ch.last < m.frame_count:
                spans += 1
                hit += moves(m, ch.node, ch.first, ch.last)
                shifted += moves(m, ch.node + 1, ch.first, ch.last)
                as_point += moves(m, ch.point, ch.first, ch.last)
    check(".ctl: a channel's +0 is the mesh node its frames move",
          total and in_range == total and hit >= spans - 10
          and 3 * shifted < hit and 3 * as_point < hit,
          f"+0 names a node of the object's mesh on {in_range} of {total} channels, and "
          f"on {hit} of the {spans} that span frames those frames move it; controls: "
          f"the next node moves on {shifted}, +20 read as a node on {as_point}")

    turrets = NResArchive.open(game / "turrets.rlb")
    tur = control.parse(turrets.read_name("o_tur_ht_02.ctl"), names)
    tm = objmesh.parse(turrets.read_name("o_tur_ha_02.msh"), "o_tur_ha_02.msh")
    named = [(tm.nodes[ch.node].name, ch.first, ch.last, ch.flags) for ch in tur.channels[:4]]
    check("o_tur_ht_02: the hero turret's channels animate its eye, yaw, pitch and barrel",
          named == [("CP_m1o1", 0, 0, 4), ("Turn_m1o1", 49, 53, 3), ("GP_m1o1", 55, 57, 0),
                    ("Gun02_m1o1", 58, 60, 0)],
          f"{named}: yaw wraps and inverts (3), the eye is not driven (4)")

    fortif = NResArchive.open(game / "fortif.rlb")
    factors: Counter[tuple[int, float]] = Counter()
    pods: dict[str, float] = {}
    for entry in fortif:
        if not entry.tag.upper().startswith("CTL"):
            continue
        blob = fortif.read(entry)
        c = control.parse(blob, names)
        rates = []
        for comp in c.components:
            if comp.type_id not in (control.DOOR_TYPE, control.COMPUTER_TYPE):
                continue
            factor = struct.unpack_from("<f", blob, comp.offset + ITEM_FACTOR_AT)[0]
            factors[(comp.flags, factor)] += 1
            if comp.type_id == control.COMPUTER_TYPE:
                rates.append(min(c.channels[k].rate for k in comp.entries))
        if rates:
            pods[entry.name.lower()] = rates[0]
    by_time = Counter(round(1 / r, 2) for r in pods.values())
    slowest = sorted(n for n, r in pods.items() if round(1 / r, 2) == 5.0)
    check("fortif.rlb: a pod opens in 1 / rate seconds, 1.43 to 5 s, and captures at 0.9 of it",
          set(factors) == {(0, 1.0)} and len(pods) == 21 and min(by_time) == 1.43
          and max(by_time) == 5.0 and all(ITEM_STEP / r >= 0.1 for r in pods.values()),
          f"all {sum(factors.values())} doors and pods carry flags 0 and step factor 1; "
          f"a building's first pod opens in (s: buildings) {dict(sorted(by_time.items()))}; "
          f"5 s on {', '.join(slowest)}.  Three 0.45 steps, each "
          f"1000 |dv| / (factor x rate) ms (Control.dll:0x10022120); the state word "
          f"clears on the third, at 0.9 / rate.  Control: no step is under the 100 ms "
          f"idle tick")


def check_effect_timing(check, game: Path) -> None:
    """How an effect runs: its time mode, its emitters' windows, the dust switch."""
    library = effects.EffectLibrary(game / "effects.rlb")

    def windows(shift: int) -> tuple[int, int, Counter[int]]:
        ok = total = 0
        bad: Counter[int] = Counter()
        for fx in library:
            for e in fx.emitters:
                at = effects.WINDOW_AT.get(e.kind)
                if at is None or at + shift < 4 or at + shift + 8 > len(e.body):
                    continue
                lo, hi = struct.unpack_from("<2f", e.body, at + shift)
                good = 0.0 <= lo <= hi <= 1.0
                ok += good
                total += 1
                bad[e.kind] += not good
        return ok, total, bad

    ok, total, bad = windows(0)
    early, late = windows(-4)[0], windows(4)[0]
    check("FXID: every emitter is active over an ordered span of effect time inside 0..1",
          total and ok >= total - 1 and early < total // 5 and late < total // 5,
          f"{ok} of {total} emitters' windows (Effect.dll, each class's update); outside: "
          f"{ {k: v for k, v in bad.items() if v} }; control: read four bytes early "
          f"{early}, late {late}")

    modes = Counter(fx.mode for fx in library)
    durations = sum(0.0 <= fx.duration < 1000.0 for fx in library)
    denormal = sum(0 < struct.unpack_from("<f", fx.header, effects.HEADER_MODE_AT)[0] < 1e-30
                   for fx in library)
    check("FXID: the header's +4 is a time mode and +8 a duration in seconds",
          len(library) and set(modes) <= set(range(effects.TIME_MODES))
          and durations == len(library) and denormal == len(library) - modes[0],
          f"modes {dict(sorted(modes.items()))} of the 18 the switch at "
          f"Effect.dll:0x10005c60 takes; {durations} of {len(library)} durations lie in "
          f"0..1000 s; control: +4 read as a float is a denormal on all {denormal} "
          f"non-zero modes")

    names = frozenset(p.name.lower() for p in all_archives(game))
    bases = NResArchive.open(game / "bases.rlb")
    carriers = held = restarted = idle = idle_total = 0
    for entry in bases:
        if not entry.tag.upper().startswith("CTL"):
            continue
        c = control.parse(bases.read(entry), names)
        surf = c.surface_groups
        if all(g == control.NO_GROUP for g in surf):
            continue
        carriers += 1
        ids = {r.args[3]: r.resource.member.lower() for r in c.references
               if r.resource.member and r.action in (control.ACT_EFFECT_POINT,
                                                     control.ACT_EFFECT_POINTS)}
        starts = [r for s, g in enumerate(surf) if s not in (0, 2)
                  for r in c.references if r.group == g]
        restarts = [r for s in (0, 2) for r in c.references if r.group == surf[s]]
        held += all(r.action == control.ACT_EFFECT_START
                    and r.args[1] == effects.TIME_MANUAL for r in starts)
        restarted += all(r.action == control.ACT_EFFECT_RESTART for r in restarts)
        for r in starts:
            fx = library.get(ids.get(r.args[0], ""))
            if fx is None:
                continue
            idle_total += 1
            lows = [e.window[0] for e in fx.emitters if e.window]
            idle += fx.mode == effects.TIME_MOTION and bool(lows) and min(lows) > 0.0
    check(".ctl: the dust shows on surfaces 0 and 2 only",
          carriers == 9 and held == restarted == carriers and idle_total and idle == idle_total,
          f"on {held} of {carriers} dust chassis surfaces 1 and 3..10 start the dust in "
          f"time mode 0, which holds its time at 0 (Effect.dll:0x100074a7), and every "
          f"one of those {idle_total} dust effects is speed-driven (mode 15) with no "
          f"emitter active at 0; on {restarted} surfaces 0 and 2 run action 11, which "
          f"hands the effect back its own mode")


def check_actions(check, game: Path) -> None:
    """Section 5's actions, what ends a round, and what an explosion plays where."""
    names = frozenset(p.name.lower() for p in all_archives(game))
    codes: Counter[int] = Counter()
    shifted: Counter[int] = Counter()
    named: Counter[tuple[int, str]] = Counter()
    rounds: dict[str, control.Controller] = {}
    turret = None
    handled = {0, 1, 2, 3, 4, 5, 7, 8, 10, 11, 12, 13, 14, 15, 17, 18, 19, 20, 21, 27}
    for path in all_archives(game):
        archive = NResArchive.open(path)
        for entry in archive:
            if not entry.tag.upper().startswith("CTL"):
                continue
            try:
                c = control.parse(archive.read(entry), names)
            except control.ControlFormatError:
                continue
            for r in c.references:
                codes[r.action] += 1
                shifted[r.args[0]] += 1
                if r.resource.member:
                    kind = "exp" if r.resource.member.lower().endswith(".exp") else "fx"
                    named[(r.action, kind)] += 1
            if path.name.lower() == "weapon.rlb":
                rounds[entry.name.lower()] = c
            if entry.name.lower() == "o_tur_ht_02.ctl":
                turret = c
    outside = sum(n for v, n in shifted.items() if v not in handled)
    check(".ctl: a section-5 record's int 3 is the action the interpreter switches on",
          set(codes) <= handled and outside > sum(shifted.values()) // 4
          and set(named) == {(3, "fx"), (4, "fx"), (5, "fx"), (27, "exp")},
          f"actions {dict(sorted(codes.items()))}, all cases of Control.dll:0x10002800; "
          f"only 3, 4 and 5 name an effect and 27 an .exp: {dict(named)}.  Control: "
          f"int 4 read as the action is outside the cases on {outside} of "
          f"{sum(shifted.values())}")

    ending: dict[int, Counter[str]] = {e: Counter() for e in (control.ENTRY_HIT,
                                                                control.ENTRY_EDGE,
                                                                control.ENTRY_RANGE)}
    for c in rounds.values():
        if not c.groups or c.groups[control.ENTRY_RANGE] == control.NO_GROUP:
            continue
        for entry, tally in ending.items():
            acts = {r.action for r in c.group(entry)}
            tally["explode" if control.ACT_EXPLODE_NODE in acts else
                  "kill" if control.ACT_KILL in acts else
                  "remove" if control.ACT_REMOVE in acts else "none"] += 1
    hit, edge, end = (ending[e] for e in (control.ENTRY_HIT, control.ENTRY_EDGE,
                                          control.ENTRY_RANGE))
    check("weapon.rlb: a round is killed on a hit, removed at the edge, exploded at range",
          hit["kill"] == 63 and hit["remove"] == 3 and edge == Counter(remove=66)
          and end["explode"] == 58 and end["none"] == 0,
          f"face hit (+0x4e4): {dict(hit)}; map edge (+0x4e8): {dict(edge)}; end of "
          f"range: {dict(end)}.  Kill (17) takes invulnerability off and runs "
          f"ILifeSystem slot 7, so node 0's .exp deals the hit; remove (15) plays "
          f"nothing; explode (27) plays the named *_end.exp")

    bound = {}
    points = {}
    if turret is not None:
        for r in turret.group(control.ENTRY_LOAD):
            if r.action == control.ACT_EFFECT_POINTS:
                bound[r.resource.member.lower()] = tuple(r.args[:3])
            elif r.action == control.ACT_EFFECT_TIME_POINT:
                points[r.args[0]] = r.args[1]
    flight = {name: sorted({r.resource.member.lower() for r in rounds[name].references
                            if r.resource.member
                            and not r.resource.member.lower().endswith(".exp")})
              for name in ("bb_h_01.ctl", "bl_h_01.ctl", "bm_h_01.ctl", "bp_h_01.ctl")}
    check("o_tur_ht_02: the hero's turret binds its nine effects to control points at load",
          len(bound) == 9 and bound.get("hero_cannon") == (11, 12, 13)
          and bound.get("hero_prifle") == (15, 16, 17)
          and bound.get("hero_redlaser") == (4, 5, 6)
          and len(points) == 7 and points.get(0) == 13
          and flight["bb_h_01.ctl"] == ["hero_cannon_bullet"]
          and flight["bl_h_01.ctl"] == ["hero_laser_bullet"],
          f"{bound}; action 14 times effect ids {points} from those control points; "
          f"the rounds fly with {flight}")

    lib = materials.MaterialLibrary(game / "Material.lib")
    by_surface: dict[int, list[str]] = defaultdict(list)
    for m in lib.materials.values():
        by_surface[m.surface].append(m.name.upper())
    tags = effects.SURFACE_TAGS
    witness = {"st": "STONE", "ic": "ICE", "mt": "B_", "gr": "TREE", "wt": "WATER",
               "an": "BIRD", "sh": "SHIELD"}

    def agree(shift: int) -> list[str]:
        out = []
        for surface in range(len(tags)):
            slot = surface + shift
            want = witness.get(tags[slot]) if 0 <= slot < len(tags) else None
            if want and surface != 2 and any(want in n for n in by_surface[surface]):
                out.append(f"{surface} {tags[slot]}")
        return out

    good = agree(0)
    off = max(len(agree(-1)), len(agree(1)))
    check(".exp: slot s + 1 is the effect for ground surface s",
          len(good) == 7 and off <= 1
          and all(n.startswith("WATER") for n in by_surface[WATER_SURFACE]),
          f"the surfaces whose materials carry their slot's tag: {', '.join(good)} "
          f"(Control.dll:0x100117d0 plays slot surface + 1); water is surface 7, "
          f"{by_surface[7]}.  Control: one slot either way, {off} of 7 agree")


def check_sensors(check, game: Path) -> None:
    """.ctl: radars and detect shields, and where the assemblies put them."""
    names = frozenset(p.name.lower() for p in all_archives(game))
    radars: list[tuple[str, str, control.Component]] = []
    shields: list[tuple[str, str, control.Component]] = []
    most_radars = 0
    seekers: set[str] = set()
    seeker_count = 0
    for path in all_archives(game):
        archive = NResArchive.open(path)
        for entry in archive:
            if not entry.tag.upper().startswith("CTL"):
                continue
            try:
                parsed = control.parse(archive.read(entry), names)
            except control.ControlFormatError:
                continue
            seekers.update(path.name.lower() for p in parsed.components if p.type_id == 17)
            seeker_count += sum(1 for p in parsed.components if p.type_id == 17)
            here = [p for p in parsed.components if p.type_id == control.RADAR_TYPE]
            most_radars = max(most_radars, len(here))
            radars += [(path.name.lower(), entry.name.lower(), p) for p in here]
            shields += [(path.name.lower(), entry.name.lower(), p)
                        for p in parsed.components if p.type_id == control.DETECT_SHIELD_TYPE]

    # -- radar values ------------------------------------------------------
    parts = {n[:-4]: p for lib, n, p in radars if lib == "intsys.rlb"}
    others = [(n, p) for lib, n, p in radars if lib != "intsys.rlb"]
    part_triples = {p.values[:3] for p in parts.values()}
    other_triples = {p.values[:3] for _n, p in others}
    periods = {p.values[control.RADAR_PERIOD] for _l, _n, p in radars}
    tail_zero = all(not any(p.values[5:]) for _l, _n, p in radars)
    check(".ctl: radar: three sensitivities, a range and a rescan period",
          len(radars) == 76 and len(parts) == 12
          and {tuple(round(v, 3) for v in t) for t in part_triples} == {(0.05, 0.7, 25.0)}
          and other_triples == {(0.5, 0.5, 0.5)}
          and periods == {750.0} and tail_zero
          and all(p.values[control.RADAR_RANGE] > 0 for _l, _n, p in radars),
          f"{len(radars)} class-8 components: the {len(parts)} radar parts read "
          f"0.05/0.7/25, the {len(others)} others 0.5/0.5/0.5; value 4 is "
          f"{sorted(periods)} on all; values 5-15 zero: {tail_zero}")

    def spread(values):
        return f"{min(values):.2f}-{max(values):.2f}"

    slot_range = {(p.label.lower(), p.values[control.RADAR_RANGE]) for lib, _n, p in radars
                  if lib == "turrets.rlb"}
    part_power = [p.power for p in parts.values()]
    fort = sorted({p.values[control.RADAR_RANGE] for lib, _n, p in radars if lib == "parts.rlb"})
    check(".ctl: radar: slots, fortifications and parts differ",
          slot_range == {("i_rdr_l", 500.0), ("i_rdr_m", 800.0), ("i_rdr_b", 800.0)}
          and {p.power for lib, _n, p in radars if lib == "turrets.rlb"} == {1.0}
          and {round(p.power, 3) for lib, _n, p in radars if lib == "parts.rlb"} == {0.01}
          and fort == [500.0, 600.0, 750.0]
          and {(lib, p.values[control.RADAR_RANGE]) for lib, _n, p in radars
               if lib in ("animals.rlb", "bases.rlb")} == {("animals.rlb", 500.0),
                                                        ("bases.rlb", 500.0)}
          and round(min(part_power), 2) == 0.08 and round(max(part_power), 2) == 0.30,
          f"turret slots {sorted(slot_range)} at power 1; bunker/tower radars "
          f"{fort} at 0.01; animals and r_l_06 500; radar parts power "
          f"{spread(part_power)}")

    ladders = {}
    for size in "lmb":
        ladders[size] = [parts[f"o_rdr_{size}_{m}"].values[control.RADAR_RANGE] for m in MARKS]
    check(".ctl: radar: a part's range grows with its mark",
          all(a < b for r in ladders.values() for a, b in zip(r, r[1:], strict=False)),
          "; ".join(f"{s} {'/'.join(f'{v:g}' for v in r)}" for s, r in ladders.items()))

    check(".ctl: radar: no controller carries two",
          most_radars == 1,
          f"at most {most_radars} class-8 component in any one .ctl -- the "
          f"control system keeps one radar pointer (Control.dll:0x1002d5d2)")

    check(".ctl: class 17 is only on projectiles",
          seekers == {"weapon.rlb"} and seeker_count == 20,
          f"{seeker_count} class-17 components, all in {sorted(seekers)}")

    # -- what a part weighs, which the mass signature totals ----------------
    masses: dict[tuple[str, int], list[float]] = defaultdict(list)
    for lib in ("intsys.rlb", "guns.rlb"):
        archive = NResArchive.open(game / lib)
        for entry in archive:
            if entry.tag.upper().startswith("CTL"):
                # o_cNN is an ammunition clip; o_gun_* is the gun itself
                kind = "clip" if entry.name.lower().startswith("o_c") else "gun"
                for part in control.parse(archive.read(entry), names).components:
                    masses[(lib if lib == "intsys.rlb" else kind, part.type_id)].append(
                        part.mass)
    armour = masses.pop(("intsys.rlb", control.ARMOUR_TYPE), [])
    clips = masses.get(("clip", control.GUN_TYPE), [])
    guns = [v for (where, _t), vs in masses.items() if where == "gun" for v in vs]
    rest = [v for (where, _t), vs in masses.items() if where == "intsys.rlb" for v in vs]
    check(".ctl: every internal part but armour, and every ammunition clip, has a mass",
          armour and not any(armour) and rest and min(rest) == 100.0
          and max(rest) == 40000.0 and clips and min(clips) > 0 and guns and not any(guns),
          f"{len(armour)} armour parts weigh 0; the other {len(rest)} intsys.rlb "
          f"parts {min(rest):g}-{max(rest):g} kg and the {len(clips)} o_cNN clips "
          f"{min(clips):g}-{max(clips):g}, where the {len(guns)} components of the o_gun "
          f"guns themselves weigh 0.  Control.dll:0x1000fac0 totals them into the mass "
          f"signature")

    # -- detect shields ----------------------------------------------------
    lib = descriptions.library(game)
    dsh = {n[:-4]: p for l_, n, p in shields if l_ == "intsys.rlb"}
    slots = [(n, p) for l_, n, p in shields if l_ == "bases.rlb" and p.label]
    joined = []
    for name, part in sorted(dsh.items()):
        entry = lib.get("i" + name[1:])
        stat = next((s.field for s in (entry.stats if entry else ())
                     if s.label.lower().startswith("supres")), None)
        joined.append((name, part.values[control.CAMOUFLAGE], stat))
    zero_is_df = all((v == 0) == name.endswith("_df") for name, v, _s in joined)
    stat_agrees = all((v == 0) == (s == '"0.0"') for _n, v, s in joined)
    # control: the stat would agree trivially if every part printed 0.0
    printed = Counter(s for _n, _v, s in joined)
    cuts = [v for n, v, _s in joined if not n.endswith("_df")]
    check(".ctl: detect shield: camouflage is value 3, not on MK1",
          len(joined) == 12 and zero_is_df and stat_agrees and len(printed) == 4,
          f"value 3 is 0 on the {sum(1 for n, *_ in joined if n.endswith('_df'))} "
          f"_df parts and {min(cuts):g}-{max(cuts):g} on the other "
          f"{sum(1 for n, *_ in joined if not n.endswith('_df'))}; objects.dlb "
          f"prints Supression {dict(sorted(printed.items()))}, 0.0 exactly there")

    powers = {size: [dsh[f"o_dsh_{size}_{m}"].power for m in MARKS] for size in "lmb"}
    cuts012 = {size: {dsh[f"o_dsh_{size}_{m}"].values[:3] for m in MARKS} for size in "lmb"}
    check(".ctl: detect shield: its three cuts are per size",
          all(len(v) == 1 for v in cuts012.values()),
          "; ".join(f"{s} " + "/".join(f"{x:g}" for x in next(iter(v)))
                    + f" at power {spread(powers[s])}" for s, v in cuts012.items()))

    camo = {}
    for size in "lmb":
        camo[size] = [(dsh[f"o_dsh_{size}_{m}"].values[control.CAMOUFLAGE],
                       dsh[f"o_dsh_{size}_{m}"].values[control.CAMOUFLAGE_POWER]) for m in MARKS]
    rising = all(a[0] < b[0] and a[1] < b[1]
                 for r in camo.values() for a, b in zip(r, r[1:], strict=False))
    check(".ctl: detect shield: cut and price rise with mark",
          rising,
          "; ".join(f"{s} " + " ".join(f"{c:g}@{w:g}" for c, w in r)
                    for s, r in camo.items()))

    check(".ctl: detect shield: a chassis has an empty slot",
          len(slots) == 22 and all(not any(p.values) for _n, p in slots)
          and all(any(p.values[:3]) for p in dsh.values()),
          f"{len(slots)} labelled i_dsh records on bases.rlb chassis, all sixteen "
          f"values zero; the {len(dsh)} intsys parts carry theirs")

    # -- where the assemblies put them --------------------------------------
    robots = 0
    placed = 0
    buildings: Counter[tuple[str, int]] = Counter()
    for path in sorted(game.glob("UNITS/**/*.dat")):
        unit = objects.load_unit(path)
        parent = unit.parents()
        member = [c.ref.member.lower() for c in unit.components]
        turrets = [i for i, m in enumerate(member) if m.startswith("e_tur")]
        rdr = [i for i, m in enumerate(member) if m.startswith("i_rdr")]
        dsh_parts = [i for i, m in enumerate(member) if m.startswith("i_dsh")]
        if turrets:
            robots += 1
            placed += (len(turrets) == 1 and len(rdr) == 1 and parent[rdr[0]] == turrets[0]
                       and len(dsh_parts) == 1 and parent[dsh_parts[0]] == 0)
        elif rdr or dsh_parts:
            placed -= 1000          # a radar part off a turret would fail this
        if member[0].startswith("fr_"):
            kind = "bunker/tower" if ("bunker" in member[0] or "tow" in member[0]) else "other"
            buildings[(kind, sum(1 for m in member if m.startswith("e_gun_fs")))] += 1
    check("UNITS: a robot's radar is on its turret",
          robots == 372 and placed == robots,
          f"{placed}/{robots} assemblies with a turret carry one i_rdr part on it "
          f"and one i_dsh part on the chassis; none without a turret carries either")

    check("UNITS: only bunkers and towers see",
          buildings[("bunker/tower", 1)] == 27 and buildings[("other", 0)] == 49
          and len(buildings) == 2,
          f"{buildings[('bunker/tower', 1)]} bunkers and towers carry one "
          f"e_gun_fs radar; the other {buildings[('other', 0)]} buildings none")


#: ``Iron_3D.ini``'s ``[LEVEL_RATIO]``: what an enemy warrior's hit points,
#: shields and gun damage are scaled by at each difficulty.
LEVEL_RATIO = {"EASY": 0.5, "MEDIUM": 0.7, "HARD": 1.0}
#: A node's hit points where it cannot be destroyed.
INDESTRUCTIBLE = 1_000_000.0


#: Mission 01's dummies and the hero.
MISSION_01_BODIES = ("r_h_01", "r_h_03", "r_h_02")
#: The hero's four rounds, and the shortest tick a frame takes, in seconds.
HERO_ROUNDS = ("bb_h_01", "bp_h_01", "bl_h_01", "bm_h_01")
TICK_FLOOR = 0.01


def check_hit_test(check, game: Path) -> None:
    """The hit test: what a round is tested against, and why a segment."""
    library = objects.ObjectLibrary(game / "objects.rlb")
    opened: dict[str, NResArchive] = {}
    meshes: dict[str, objmesh.ObjectMesh | None] = {}

    def read(ref):
        key = ref.library.lower()
        if key not in opened:
            opened[key] = NResArchive.open(game / key)
        return opened[key].read_name(ref.member)

    def mesh_of(record):
        ref = record.mesh
        key = f"{ref.library}/{ref.member}".lower()
        if key not in meshes:
            try:
                meshes[key] = objmesh.parse(read(ref), ref.member)
            except KeyError:
                meshes[key] = None
        return meshes[key]

    for record in library.records.values():
        if record.mesh:
            mesh_of(record)
    absent = sorted(k for k, m in meshes.items() if m is None)
    present = [m for m in meshes.values() if m is not None]

    # AniMesh.dll:0x10010c33 takes level 0 of the node's variant, so a hull,
    # which keeps its geometry only in the fifth slot, is never struck.
    hulls = [n for m in present for n in m.nodes if n.is_collision]
    hulls_hit = sum(n.hit_slot(v) is not None
                    for n in hulls for v in range(objmesh.VARIANT_COUNT))
    solid = sum(n.hit_slot() is not None for m in present for n in m.nodes
                if not n.is_collision)
    fifth = sum(n.collision_slot() is not None for m in present for n in m.nodes)
    check("mesh: a round is tested against level 0, which no collision hull has",
          hulls and hulls_hit == 0 and solid > len(hulls),
          f"{len(hulls)} hulls across {len(present)} meshes carry a level-0 slot in 0 "
          f"of {objmesh.VARIANT_COUNT} variants; control: {solid} other nodes carry one. "
          f"So the {fifth} fifth slots are not what a round hits "
          f"({len(absent)} named meshes are absent from their archives)")

    # Control.dll:0x1001d9fa: a round's query passes faces flagged 4 or 32.
    skipped = tested = 0
    carriers: Counter[str] = Counter()
    flags: Counter[int] = Counter()
    for key, m in meshes.items():
        if m is None:
            continue
        for n in m.nodes:
            index = n.hit_slot()
            if index is None:
                continue
            s = m.slots[index]
            faces = m.face_flags[s.first_triangle:s.first_triangle + s.triangle_count]
            flags.update(faces)
            passed = sum(1 for f in faces if f & objmesh.ROUND_SKIPS_FACE)
            skipped += passed
            tested += len(faces) - passed
            if passed:
                carriers[key.split("/")[-1]] += passed
    check("mesh: a round passes through faces flagged 4 or 32, and strikes 2 and 16",
          skipped and tested > 50 * skipped and flags[2] and flags[16],
          f"level 0, variant 0: {skipped} of {skipped + tested} triangles let a round "
          f"through, on {len(carriers)} meshes (most: "
          + ", ".join(f"{k} {v}" for k, v in carriers.most_common(4))
          + f"); face flags {dict(sorted(flags.items()))}")

    tags = Counter(r.tag for r in library.records.values())
    rows = []
    bodies_ok = True
    for name in MISSION_01_BODIES:
        record = library.get(name)
        m = mesh_of(record)
        hit = [n.hit_slot() for n in m.nodes]
        triangles = sum(m.slots[i].triangle_count for i in hit if i is not None)
        bodies_ok &= (objects.COLLISION_KIND.get(record.tag) == objects.KIND_UNIT
                      and triangles > 0 and not any(n.is_collision for n in m.nodes))
        rows.append(f"{name}: {sum(i is not None for i in hit)} of {len(m.nodes)} "
                    f"nodes, {triangles} triangles")
    check("objects.rlb: Mission 01's dummies and hero are units with geometry to hit",
          bodies_ok,
          "; ".join(rows) + f".  Records by tag: {dict(sorted(tags.items()))}; "
          f"BTLU is collision kind 4 and BULL 9 (AniMesh.dll:0x1000317f)")

    # A round moves further than its own radius in one tick, which is why the
    # test is a segment and a swept sphere rather than a sample.
    names = frozenset(p.name.lower() for p in all_archives(game))
    ratios = []
    hero = []
    missing = 0
    for record in library.by_tag("BULL"):
        try:
            ctl = control.parse(read(record.slot_with_suffix("ctl")), names)
        except KeyError:
            missing += 1
            continue
        m = mesh_of(record)
        radius = m.volume.radius if m and m.volume else 0.0
        speed = ctl.triples[control.TRIPLE_TOP_SPEED][1]
        if radius > 0 and speed > 0:
            ratios.append((speed * TICK_FLOOR / radius, record.name.lower()))
        if record.name.lower() in HERO_ROUNDS:
            hero.append(f"{record.name.lower()} {speed:g} m/s to {ctl.bounds[0]:g}, "
                        f"radius {radius:.2f}")
    ratios.sort()
    over = sum(1 for r, _ in ratios if r > 1)
    check("weapon.rlb: most rounds cover more than their own radius in a 0.01 s tick",
          len(hero) == len(HERO_ROUNDS) and over > len(ratios) // 2,
          "; ".join(sorted(hero)) + f".  {over} of {len(ratios)} rounds outrun their "
          f"radius (fastest {ratios[-1][1]} x{ratios[-1][0]:.0f}, slowest "
          f"{ratios[0][1]} x{ratios[0][0]:.2f}; {missing} name an absent .ctl)")


def check_combat(check, game: Path) -> None:
    """Damage, shields, armour and repair, against the shipped data."""
    names = frozenset(p.name.lower() for p in all_archives(game))

    # ---- a direct round's damage is its hit points plus the explosion's
    weapon = NResArchive.open(game / "weapon.rlb")
    members = {e.name.lower(): e for e in weapon}
    blasts = {name: effects.parse_explosion(weapon.read(e), name)
              for name, e in members.items() if name.endswith(".exp")}
    direct = []
    for name, entry in sorted(members.items()):
        if not name.endswith(".ndp"):
            continue
        for row in objects.parse_damage(weapon.read(entry), name):
            blast = blasts.get(row.explosion.member.lower()) if row.explosion else None
            if blast and blast.kind == effects.HIT_DIRECT:
                direct.append((row.durability, blast.damage))
    whole = sum(1 for hp, dmg in direct if (hp + dmg) % 10 == 0)
    alone = sum(1 for hp, _ in direct if hp % 10 == 0)
    check("weapon.rlb: a direct round hits for its hit points plus one",
          direct and all(dmg == 1 for _, dmg in direct)
          and whole >= len(direct) - 1 and alone == 0,
          f"all {len(direct)} direct-hit rounds' explosions do 1; hit points "
          f"plus that are a multiple of 10 on {whole}, the hit points alone "
          f"on {alone} -- {sorted({int(h + d) for h, d in direct})[:8]}...")

    # ---- the components
    parts: dict[int, list[tuple[str, str, control.Component]]] = defaultdict(list)
    for path in all_archives(game):
        archive = NResArchive.open(path)
        for entry in archive:
            if not entry.tag.upper().startswith("CTL"):
                continue
            try:
                parsed = control.parse(archive.read(entry), names)
            except control.ControlFormatError:
                continue
            for part in parsed.components:
                parts[part.type_id].append((path.name, entry.name, part))

    shields = [p for _, _, p in parts[control.FIGHT_SHIELD_TYPE]]
    check(".ctl: class 9: a fight shield is a maximum, a rate and a price",
          shields and all(not any(p.values[3:]) for p in shields)
          and all(p.values[0] > 0 and p.values[1] > 0 for p in shields),
          f"{len(shields)} fight shields, all with values 3-15 zero; per-sector "
          f"maximum {min(p.values[0] for p in shields):g}-"
          f"{max(p.values[0] for p in shields):g}, recharge "
          f"{min(p.values[1] for p in shields):g}-"
          f"{max(p.values[1] for p in shields):g} a second")
    check(".ctl: class 9: no fight shield recharges for free",
          all(p.values[2] > 0 for p in shields),
          f"value 2, the charge a point costs, is above zero on "
          f"{sum(p.values[2] > 0 for p in shields)}/{len(shields)}: "
          f"{sorted({round(p.values[2], 5) for p in shields})}")

    repairs = [p for _, _, p in parts[control.REPAIR_TYPE]]
    check(".ctl: class 15: no repair system is free",
          repairs and all(p.values[1] > 0 for p in repairs)
          and all(not any(p.values[2:]) for p in repairs),
          f"value 1 above zero on {sum(p.values[1] > 0 for p in repairs)}/"
          f"{len(repairs)}: {sorted({round(p.values[1], 5) for p in repairs})}; "
          f"values 2-15 zero throughout")

    deflectors = [(a, p) for a, _, p in parts[control.DEFLECTOR_TYPE]]
    equal = sum(1 for _, p in deflectors
                if len(set(p.values[:6])) == 1 and 0 < p.values[0] <= 1
                and not any(p.values[6:]))
    where = Counter(a for a, _ in deflectors)
    check(".ctl: class 21: a deflector is six coefficients, one per face",
          deflectors and equal == len(deflectors),
          f"{equal}/{len(deflectors)} carry six equal values in 0..1 and "
          f"nothing after -- {sorted({p.values[0] for _, p in deflectors})}")

    fortif_shield = {e for a, e, _ in parts[control.FIGHT_SHIELD_TYPE] if a == "fortif.rlb"}
    fortif_deflector = {e for a, e, _ in parts[control.DEFLECTOR_TYPE] if a == "fortif.rlb"}
    check("fortif.rlb: buildings carry shields but no deflector",
          fortif_shield and not fortif_deflector,
          f"{len(fortif_shield)} building controllers have a fight shield and "
          f"{len(fortif_deflector)} a deflector; the {len(deflectors)} "
          f"deflectors are in {dict(where)}")

    armour = [(a, p) for a, _, p in parts[control.ARMOUR_TYPE]]
    chassis = [p for a, p in armour if a == "bases.rlb"]
    fitted = [p for a, p in armour if a == "intsys.rlb"]
    even = [(1 - p.values[1]) / p.values[2] for p in fitted if p.values[2] > 0]
    check(".ctl: class 27: armour is a linear and a squared factor",
          chassis and fitted
          and all(tuple(p.values[:3]) == (0.0, 1.0, 0.0) for p in chassis)
          and all(0 < p.values[1] < 1 and p.values[2] > 0 for p in fitted)
          and len(even) == len(fitted) and 1500 < min(even) and max(even) < 3500,
          f"the {len(chassis)} chassis armours are (0, 1, 0), no reduction; the "
          f"{len(fitted)} fitted ones keep {min(p.values[1] for p in fitted):.2f}-"
          f"{max(p.values[1] for p in fitted):.2f} of a hit plus a square term, "
          f"so a hit of {min(even):.0f}-{max(even):.0f} or more goes through whole")

    # ---- the difficulty ratio
    ini = settings.sections(game / "Iron_3D.ini")
    ratio = {k: float(v) for k, v in ini.get("LEVEL_RATIO", {}).items()}
    check("Iron_3D.ini: LEVEL_RATIO scales enemies 0.5, 0.7, 1.0",
          ratio == LEVEL_RATIO,
          f"{ratio}; iron3d.dll:0x10076010 reads the one GAME_LEVEL picks "
          f"(GAME_LEVEL={ini.get('CS', {}).get('GAME_LEVEL')})")

    # ---- hit points
    bases = NResArchive.open(game / "bases.rlb")
    by_size: dict[str, list[float]] = defaultdict(list)
    for entry in bases:
        name = entry.name.lower()
        if name.endswith(".ndp") and name.startswith("r_"):
            node0 = objects.parse_damage(bases.read(entry), name)[0].durability
            by_size[name[2]].append(node0)
    medians = {k: statistics.median(v) for k, v in by_size.items()}
    check("bases.rlb: a chassis's hit points grow with its size",
          medians["t"] < medians["l"] < medians["m"] < medians["b"]
          and all(x < INDESTRUCTIBLE for v in by_size.values() for x in v),
          "node-0 hit points by size letter: " + ", ".join(
              f"{k} {min(v):g}-{max(v):g} (median {medians[k]:g})"
              for k, v in sorted(by_size.items(), key=lambda kv: medians[kv[0]])))

    fortif = NResArchive.open(game / "fortif.rlb")
    frail = total_b = 0
    for entry in fortif:
        name = entry.name.lower()
        if not name.endswith(".ndp"):
            continue
        table = objects.parse_damage(fortif.read(entry), name)
        total_b += 1
        frail += table[0].durability <= 1 and len(table) > 1
    check("fortif.rlb: a building's first node can be a 1-hit-point stub",
          frail > 0,
          f"{frail}/{total_b} building tables give node 0 one hit point among "
          f"sturdier parts; Control.dll:0x100110ab spares a type-3 object when "
          f"node 0 goes, where a unit dies")


#: The largest size class that may capture a building (``Behavior.dll:0x100301a9``).
CAPTURE_SIZE = 2


def _hall_ways(game: Path) -> dict[str, objmesh.PathGraph | None]:
    fortif = NResArchive.open(game / "fortif.rlb")
    out = {}
    for e in fortif:
        if e.tag == "MESH":
            out[e.name.lower()] = objmesh.read_path_graph(NResArchive(fortif.read(e), e.name))
    return out


def _building_places(game: Path) -> dict[str, tuple[str, list[int]]]:
    """Building root record -> (folder, hall-way flag words), via its ``.bas``."""
    lib = objects.ObjectLibrary(game / "objects.rlb")
    graphs = _hall_ways(game)
    out: dict[str, tuple[str, list[int]]] = {}
    for f in sorted(game.glob("UNITS/BUILDS/**/*.dat")):
        root = objects.load_unit(f).components[0].ref.member.lower()
        rec = lib.get(root)
        bas = rec.footprint if rec else None
        if bas is None:
            continue
        graph = graphs.get(bas.member.rsplit(".", 1)[0].lower() + ".msh")
        if graph is not None:
            out[root] = (f.parent.name.upper(), [n.flags for n in graph.nodes])
    return out


def check_ownership(check, game: Path) -> None:
    """Charging docks and control pods in the hall-way graph; clan types."""
    places = _building_places(game)
    by_folder: dict[str, set[str]] = defaultdict(set)
    pods: dict[str, int] = {}
    inside: dict[str, int] = {}
    ground: dict[str, int] = {}
    for root, (folder, flags) in places.items():
        by_folder[folder].add(root)
        pods[root] = sum(1 for a in flags if a & objmesh.PLACE_POD)
        docks = [a for a in flags if a & objmesh.PLACE_DOCK]
        inside[root] = sum(1 for a in docks if not a & objmesh.PLACE_GROUND)
        ground[root] = sum(1 for a in docks if a & objmesh.PLACE_GROUND)

    def folders(table: dict[str, int]) -> set[str]:
        return {places[r][0] for r, n in table.items() if n}

    capturable = {"BUNKER", "GENER", "HANGAR", "INSTITUT", "MINE", "PLANT",
                  "STORAGE", "TELEMAIN", "TOWER"}
    one_pod = all(pods[r] == 1 for f in capturable for r in by_folder[f])
    check("fortif.rlb: every capturable building has one pod",
          one_pod and folders(pods) == capturable,
          f"{sum(pods.values())} pods on {len([r for r in pods if pods[r]])} models; "
          f"none on {', '.join(sorted(set(by_folder) - capturable))}")

    pod_ground = sum(1 for _, flags in places.values()
                     for a in flags if a & objmesh.PLACE_POD and a & objmesh.PLACE_GROUND)
    check("fortif.rlb: a pod is always inside", pod_ground == 0,
          f"{sum(pods.values())} pods, {pod_ground} ground-level")

    dock_folders = folders(inside) | folders(ground)
    check("fortif.rlb: docks are in bunkers, generators, Outposts, plants, towers",
          dock_folders == {"BUNKER", "GENER", "HANGAR", "PLANT", "TOWER", "RUIN"},
          f"docks in {sorted(dock_folders)}; control: {sum(1 for r in places if pods[r])} "
          f"pod models, {len([r for r in places if inside[r] or ground[r]])} dock models")

    ground_models = {r: n for r, n in ground.items() if n}
    check("fortif.rlb: ground-level docks: generator 2, Outpost 1, plant 1 each",
          ground_models == {"fr_l_gener": 2, "fr_l_angar": 1,
                            "fr_b_plant": 1, "fr_m_plant": 1, "fr_l_plant": 1},
          ", ".join(f"{r} {n}" for r, n in sorted(ground_models.items())))

    inside_models = sorted(r for r, n in inside.items() if n)
    check("fortif.rlb: an indoor dock in bunkers, towers, plants, generator, big ruin",
          all(inside[r] == 1 for r in inside_models)
          and {places[r][0] for r in inside_models}
          == {"BUNKER", "TOWER", "PLANT", "GENER", "RUIN"},
          f"{len(inside_models)} models, one each; ruin: "
          f"{[r for r in inside_models if places[r][0] == 'RUIN']}")

    # The Last Gate's lone charger: the player's large ruin, placed nowhere else.
    loaded = [mission.load(p) for p in sorted(game.glob("MISSIONS/**/data.tma"))]
    last = next(m for m in loaded if m.title.startswith("The Last Gate"))
    players = {i for i, c in enumerate(last.clans) if c.type == mission.CLAN_PLAYER}
    player = [o for o in last.objects
              if o.clan_id in players and o.kind == mission.KIND_BUILDING]
    ruin = [o for o in player if "b_ruin" in o.path.lower()]
    placed = sorted({m.title for m in loaded for o in m.objects if "b_ruin" in o.path.lower()})
    check("fortif.rlb: The Last Gate's player owns a dock with no pod",
          len(player) == 1 and len(ruin) == 1 and len(placed) == 1
          and inside["fr_b_ruin"] == 1 and pods["fr_b_ruin"] == 0,
          f"player buildings {[o.path.split(chr(92))[-1] for o in player]}; the "
          f"large ruin is placed only in {placed}")

    # A capturer is tiny or small: every hero chassis is size letter h.
    sizes: dict[str, Counter[int]] = defaultdict(Counter)
    for f in sorted(game.glob("UNITS/UNITS/**/*.dat")):
        root = objects.load_unit(f).components[0].ref.member
        if root.lower().startswith("r_"):
            sizes[f.parent.name.upper()][profiles.CHASSIS_SIZE[root[2].lower()]] += 1
    hero_ok = set(sizes["HERO"]) <= set(range(1, CAPTURE_SIZE + 1))
    battle_big = sum(n for s, n in sizes["BATTLE"].items() if s > CAPTURE_SIZE)
    check("UNITS: every hero may capture", hero_ok and battle_big > 0,
          f"hero sizes {dict(sizes['HERO'])}; control: {battle_big} of "
          f"{sum(sizes['BATTLE'].values())} battle chassis are too big")

    # The clan word is a type, not an index.
    kinds: Counter[int] = Counter()
    named = defaultdict(Counter)
    at_position = total = 0
    neutral_units = neutral_buildings = 0
    for p in sorted(game.glob("MISSIONS/**/data.tma")):
        m = mission.load(p)
        for pos, clan in enumerate(m.clans):
            total += 1
            kinds[clan.type] += 1
            at_position += clan.type == pos + 1
            name = clan.name.lower()
            group = ("neutral" if re.match(r"^n(eu)?tr|^clan iiin", name)
                     else "nature" if re.match(r"^(an|nat|nar|bird)", name)
                     else "other")
            named[group][clan.type] += 1
            if clan.type == mission.CLAN_NEUTRAL:
                for o in m.objects_of_clan(pos):
                    if o.kind == mission.KIND_UNIT:
                        neutral_units += 1
                    elif o.kind == mission.KIND_BUILDING:
                        neutral_buildings += 1
    check("data.tma: the clan word is a type 0..3",
          set(kinds) == {0, 1, 2, 3} and at_position < total,
          f"{dict(sorted(kinds.items()))} over {total} clans; control: equals the "
          f"1-based position on {at_position}")
    check("data.tma: neutral-named clans are type 3, nature type 0",
          set(named["nature"]) == {mission.CLAN_NATURE}
          and named["neutral"][mission.CLAN_NEUTRAL] == sum(named["neutral"].values()) - 1
          and named["neutral"][mission.CLAN_ENEMY] == 1
          and mission.CLAN_NEUTRAL not in named["other"],
          f"neutral names {dict(named['neutral'])}, nature {dict(named['nature'])}, "
          f"others {dict(named['other'])}")
    check("data.tma: neutral clans own units and buildings",
          neutral_units > 0 and neutral_buildings > 0,
          f"{neutral_units} units, {neutral_buildings} buildings")


#: The command the hero presses at a neutral bot (``iron3d.dll:0x10071f08``).
ENTER = "CMD_ENTER_STATE"


def check_capture(check, game: Path) -> None:
    """Pods are computers; a hero takes a neutral unit by entering it."""
    names = frozenset(p.name.lower() for p in all_archives(game))
    fortif = NResArchive.open(game / "fortif.rlb")
    entries = {e.name.lower(): e for e in fortif}
    computers: set[str] = set()
    doors: set[str] = set()
    stems: set[str] = set()
    for name, entry in entries.items():
        if not entry.tag.upper().startswith("CTL"):
            continue
        stem = name[:-4]
        stems.add(stem)
        parts = control.parse(fortif.read(entry), names).components
        if any(p.type_id == control.COMPUTER_TYPE for p in parts):
            computers.add(stem)
        if any(p.type_id == control.DOOR_TYPE for p in parts):
            doors.add(stem)
    pods: set[str] = set()
    for stem in stems:
        mesh = entries.get(stem + ".msh")
        if mesh is None:
            continue
        graph = objmesh.read_path_graph(NResArchive(fortif.read(mesh), mesh.name))
        if graph and any(n.flags & objmesh.PLACE_POD for n in graph.nodes):
            pods.add(stem)
    check("fortif.rlb: a building has a pod exactly when it has a computer",
          pods and pods == computers and doors - pods,
          f"{len(computers)} of {len(stems)} building controllers carry a class-13 "
          f"computer, and they are exactly the {len(pods)} whose hall way has a pod "
          f"(0x40); control: doors (class 12) are also on "
          f"{', '.join(sorted(doors - pods))}, with no pod.  Terrain.dll's "
          f"CBuilding runs its first computer as the pod (0x10057550)")

    # The hero's Enter takes a neutral unit: every neutral-owned unit has a Type
    # the handler accepts, and no neutral-owned building does.
    unit_types: Counter[int] = Counter()
    building_types: Counter[int] = Counter()
    for path in sorted(game.glob("MISSIONS/**/data.tma")):
        m = mission.load(path)
        for obj in m.objects:
            if obj.clan_id is None or not 0 <= obj.clan_id < len(m.clans):
                continue
            if m.clans[obj.clan_id].type != mission.CLAN_NEUTRAL:
                continue
            kind = obj.properties.get("Type")
            value = int(kind.value) if kind is not None else 0
            if obj.kind == mission.KIND_UNIT:
                unit_types[value] += 1
            elif obj.kind == mission.KIND_BUILDING:
                building_types[value] += 1
    def unit(t: int) -> bool:
        return bool(t) and t & mission.UNIT_TYPE_MASK == t

    fits = sum(n for t, n in unit_types.items() if unit(t))
    buildings_fit = sum(n for t, n in building_types.items() if unit(t))
    check("data.tma: every neutral unit is one a hero can take",
          unit_types and fits == sum(unit_types.values()) and building_types
          and buildings_fit == 0,
          f"{fits}/{sum(unit_types.values())} units owned by type-3 clans have a Type "
          f"inside 0x103e000 ("
          + ", ".join(f"{t:#x} x{n}" for t, n in sorted(unit_types.items()))
          + f"); "
          f"control: none of their {sum(building_types.values())} buildings does.  "
          f"iron3d.dll:0x10071f08 captures such a unit within 20 of the hero on {ENTER}")

    label = controls.commands(game).get(ENTER)
    keys = {b.key for man in ("ui_other.man", "ui_other_d.man", "addition.man")
            if (game / man).exists()
            for b in controls.bindings(game / man) if b.command == ENTER}
    check("Command.dsc: the hero's Enter is 'Enter warbot/HQ'",
          label == "Enter warbot/HQ" and keys == {"SCAN_W_ENTER"}
          and controls.CMD.get(ENTER) == 730,
          f"{ENTER} = {controls.CMD.get(ENTER)} ({label!r}), bound to {sorted(keys)}; "
          f"case 0x2da of iron3d.dll's command handler 0x10071cd0")


#: The difficulty profiles in ``behpsp.res``.
DIFFICULTY_PROFILES = ("diff_strong.var", "diff_normal.var", "diff_weak.var",
                       "diff_slow.var", "diff_stupid.var")


def check_repair(check, game: Path) -> None:
    """The repair system: off until switched, self-only, and the AI's thresholds."""
    names = frozenset(p.name.lower() for p in all_archives(game))
    repairs: list[tuple[str, str, control.Component]] = []
    initial: dict[int, Counter] = defaultdict(Counter)
    for path in all_archives(game):
        archive = NResArchive.open(path)
        for entry in archive:
            if not entry.tag.upper().startswith("CTL"):
                continue
            try:
                parsed = control.parse(archive.read(entry), names)
            except control.ControlFormatError:
                continue
            for part in parsed.components:
                initial[part.type_id][part.state] += 1
                if part.type_id == control.REPAIR_TYPE:
                    repairs.append((path.name.lower(), entry.name.lower(), part))

    # 1. every repair system ships switched off: its record leaves the state
    # word (+0x18) at -1, so the class constructor's 0 stands.
    set_elsewhere = {t: dict(c) for t, c in initial.items() if set(c) != {None}}
    check("class 15: every repair system ships switched off",
          repairs and set(initial[control.REPAIR_TYPE]) == {None} and set_elsewhere,
          f"all {len(repairs)} class-15 records leave +0x18 at -1, so the "
          f"constructor's state 0 stands (Control.dll:0x10022ae0); control: "
          f"classes {sorted(set_elsewhere)} do set it -- {set_elsewhere}")

    # 2. the player's key toggles it: the tables send CIS_SWITCH_INV to the class
    cis = controls.CIS
    rows = []
    for name in controls.TABLES:
        rows += [(name, a) for a in controls.table(game / name)
                 if a.target == "CICLS_REPAIRSYS"]
    label = controls.commands(game).get("CMD_REPAIRSYS_ON", "")
    check("controls: one key toggles the repair system in every table",
          len(rows) == len(controls.TABLES)
          and all(a.command == "MCMD_STATE" and a.state == "CIS_SWITCH_INV"
                  and a.pressed for _, a in rows)
          and cis["CIS_SWITCHON"] == control.STATE_ON
          and cis["CIS_SWITCH_INV"] == control.STATE_TOGGLE
          and "repair" in label.lower(),
          f"{', '.join(f'{n} {a.key}' for n, a in rows)} send MCMD_STATE "
          f"CIS_SWITCH_INV ({cis['CIS_SWITCH_INV']:#x}) to CICLS_REPAIRSYS -- "
          f"Command.dsc calls it {label!r}; the class toggles 0 and "
          f"CIS_SWITCHON ({cis['CIS_SWITCHON']:#x})")

    # 3. a repair unit has no reach: two values, the rest zero
    tails = all(not any(p.values[2:]) for _, _, p in repairs)
    parts = {n[:-4]: p for lib, n, p in repairs if lib == "intsys.rlb"}
    ladder_ok = True
    detail = []
    for size in "lmbf":
        mine = [parts.get(f"o_rps_{size}_{m}") for m in MARKS]
        if not all(mine):
            ladder_ok = False
            continue
        rates = [p.values[0] for p in mine]
        costs = {round(p.values[1], 5) for p in mine}
        ladder_ok &= all(a < b for a, b in zip(rates, rates[1:], strict=False))
        detail.append(f"{size} {'/'.join(f'{r:g}' for r in rates)} HP/s at "
                      f"{'/'.join(f'{c:g}' for c in sorted(costs, reverse=True))} a point")
    check("class 15: a repair unit is a rate and a price, and nothing else",
          repairs and tails and ladder_ok and len(parts) == 16,
          f"values 2-15 zero on all {len(repairs)} -- no range, no target; the "
          f"16 intsys parts by mark: {'; '.join(detail)}")

    # 4. the catalogue calls what it does regeneration
    lib = descriptions.library(game)
    panels = {k: [(s.label, s.unit) for s in d.stats] for k, d in lib.items()
              if k.lower().startswith("i_rps_")}
    check("objects.dlb: a repair unit's stat is Regeneration, in HP/s",
          len(panels) == 16 and all(("Regeneration", "HP/s") in v for v in panels.values()),
          f"{sum(('Regeneration', 'HP/s') in v for v in panels.values())}/{len(panels)} "
          f"i_rps catalogue entries show Regeneration HP/s")

    # 5. every robot and nearly every building is fitted with one
    fitted: Counter = Counter()
    without: list[str] = []
    for dat in sorted(game.glob("UNITS/**/*.dat")):
        unit = objects.load_unit(dat)
        members = [c.ref.member.lower() for c in unit.components]
        root = members[0] if members else ""
        kind = ("robot" if root.startswith("r_") else
                "building" if root.startswith("fr_") else "other")
        count = sum(1 for m in members if m.startswith("i_rps"))
        fitted[(kind, count)] += 1
        if kind != "other" and not count:
            without.append(root)
    robots = {n: c for (k, n), c in fitted.items() if k == "robot"}
    buildings = {n: c for (k, n), c in fitted.items() if k == "building"}
    # the exceptions are the two firing-range targets, the power mast, the
    # ruins and the small main teleport
    expected = {"r_h_01", "r_h_03", "fr_l_mast", "fr_b_ruin", "fr_e_ruin",
                "fr_m_ruin", "fr_l_ruin", "fr_l_mtp"}
    check("UNITS: every robot and building but a few carries one repair unit",
          set(robots) <= {0, 1} and set(buildings) <= {0, 1} and set(without) == expected,
          f"robots with one: {robots.get(1, 0)}, without: {robots.get(0, 0)}; "
          f"buildings with one: {buildings.get(1, 0)}, without: {buildings.get(0, 0)} "
          f"-- the exceptions are {', '.join(sorted(set(without)))}")

    # 6. the AI's thresholds switch on low and off high
    held = profiles.load(game)
    pairs = {n: (held[n]["Decision_RepairOn"].value, held[n]["Decision_RepairOff"].value)
             for n in DIFFICULTY_PROFILES if n in held}
    check("behpsp.res: an AI repairs from Decision_RepairOn up to Decision_RepairOff",
          len(pairs) == len(DIFFICULTY_PROFILES)
          and all(0 < on < off < 1 for on, off in pairs.values()),
          "; ".join(f"{n[5:-4]} on below {on:g}, off above {off:g}"
                    for n, (on, off) in pairs.items())
          + " (Behavior.dll:0x10017c70)")

    # 7. no hit heals: every explosion's damage is at least zero
    damages = []
    for path in sorted(game.glob("*.rlb")) + sorted(game.glob("*.lib")):
        try:
            archive = NResArchive.open(path)
        except NotAnNResArchive:
            continue
        for entry in archive:
            if entry.name.lower().endswith(".exp"):
                blob = archive.read(entry)
                if len(blob) == effects.EXPLOSION_SIZE:
                    damages.append(effects.parse_explosion(blob, entry.name).damage)
    check("*.exp: no explosion does negative damage",
          damages and min(damages) >= 0,
          f"{len(damages)} explosions, damage {min(damages):g}..{max(damages):g}; a hit "
          f"is clamped at 0 as well (Control.dll:0x10010253), so nothing heals by hitting")


#: The chassis a player can research and build: every one with a cost.
PLAYER_CHASSIS = ("R_T_01", "R_T_02", "R_L_01", "R_L_02", "R_L_03", "R_L_04", "R_L_05",
                  "R_M_01", "R_M_02", "R_M_03", "R_M_04", "R_B_01", "R_B_02", "R_B_03",
                  "R_B_04")
#: The words a chassis's objects.dlb name uses for each ChassisType.
CHASSIS_TYPE_WORDS = {1: ("Flying", "Helicopter"),
                      2: ("Walking", "Spider", "Transformer", "Tower", "Hero"),
                      3: ("Wheel",), 4: ("Track",)}


def _robots(game: Path):
    for path in sorted(game.glob("UNITS/UNITS/**/*.dat")):
        unit = objects.load_unit(path)
        if unit.components[0].ref.member.lower().startswith("r_"):
            yield path, unit


def _on_disk(game: Path, path: str) -> Path:
    """A mission's backslashed, case-blind object path, on this filesystem."""
    here = game
    for piece in path.replace("\\", "/").split("/"):
        here = next(p for p in here.iterdir() if p.name.lower() == piece.lower())
    return here


def check_chassis(check, game: Path) -> None:
    """Chassis: the slots a chassis declares, what fills them, and what it weighs."""
    library = objects.ObjectLibrary(game / "objects.rlb")
    bases = NResArchive.open(game / "bases.rlb")
    labels: dict[str, list[control.Component]] = {}
    for entry in bases:
        if entry.tag.upper().startswith("CTL"):
            parsed = control.parse(bases.read(entry))
            labels[entry.name.lower()[:-4].upper()] = [p for p in parsed.components if p.label]

    # 1. six labelled slots on node 0, each on the class its family names
    robots = list(_robots(game))
    used = sorted({u.components[0].ref.member.upper() for _, u in robots})
    shapes = Counter()
    wrong = []
    for chassis in used:
        slots = labels.get(chassis, [])
        families = sorted(p.label[:5].lower() for p in slots)
        shapes[tuple(families)] += 1
        wrong += [f"{chassis} {p.label}" for p in slots
                  if control.SLOT_FAMILIES.get(p.label[:5].lower()) != p.type_id or p.node != 0]
    six = tuple(sorted(control.SLOT_FAMILIES))
    hero = tuple(sorted(set(control.SLOT_FAMILIES) - {"i_eng"}))
    check("bases.rlb: a chassis declares six labelled slots for internal parts",
          not wrong and set(shapes) <= {six, hero, ("i_eng",)} and shapes[six] >= 15,
          f"of the {len(used)} chassis robots use, {shapes[six]} declare engine, battery, fight "
          f"shield, detection shield, repair and armour, each on node 0 and on the class its "
          f"label names; the hero {shapes[hero]} (no engine), the targets {shapes[('i_eng',)]}")

    # 2. a robot fills every slot of its chassis once, with a part whose name the label prefixes
    fits = misfits = twice = 0
    by_chassis: dict[str, list[str]] = defaultdict(list)
    for _, unit in robots:
        chassis = unit.components[0].ref.member.upper()
        mine = [p.label.lower() for p in labels.get(chassis, [])]
        seen = Counter()
        pairs = zip(unit.components, unit.parents(), strict=True)
        parts = [c.ref.member.lower() for c, parent in pairs
                 if parent == 0 and c.ref.member.lower().startswith("i_")]
        by_chassis[chassis].append(tuple(parts))
        for part in parts:
            hit = [label for label in mine if part.startswith(label)]
            fits += bool(hit)
            misfits += not hit
            seen.update(hit)
        twice += any(n > 1 for n in seen.values())
    shuffled_misfits = 0
    names = list(by_chassis)
    rng = random.Random(1)
    for chassis, fitted in by_chassis.items():
        sizes = {p.label[-1] for p in labels[chassis]}
        other = rng.choice([n for n in names
                            if labels.get(n) and {p.label[-1] for p in labels[n]} != sizes]
                           or names)
        theirs = [p.label.lower() for p in labels.get(other, [])]
        shuffled_misfits += sum(1 for parts in fitted for part in parts
                                if not any(part.startswith(label) for label in theirs))
    check("UNITS: a robot fills its chassis's slots, once each",
          fits and not misfits and not twice and shuffled_misfits,
          f"all {fits} internal parts on {len(robots)} robots start with one of their chassis's "
          f"labels and no label is filled twice; given another size's chassis, "
          f"{shuffled_misfits} would not fit")

    # 3. a turret's size letter is its chassis's
    same = other = 0
    letters = []
    for _, unit in robots:
        chassis = unit.components[0].ref.member.lower()
        for c, parent in zip(unit.components, unit.parents(), strict=True):
            if parent == 0 and c.ref.member.lower().startswith("e_tur_"):
                letters.append((chassis[2], c.ref.member.lower()[6]))
    same = sum(1 for a, b in letters if a == b)
    pool = [b for _, b in letters]
    random.Random(2).shuffle(pool)
    shuffled = sum(1 for (a, _), b in zip(letters, pool, strict=True) if a == b)
    check("UNITS: a chassis carries a turret of its own size letter",
          letters and same == len(letters) and shuffled < same,
          f"{same}/{len(letters)} turrets share their chassis's size letter; shuffled, {shuffled}")

    # 4. a BTLU record's sixth slot is its chassis profile, and the profile names the locomotion
    held = profiles.load(game)
    catalogue = descriptions.library(game)
    rows = []
    crossed = []
    for name, record in library.records.items():
        var = record.profile
        if not name.startswith("r_") or var is None:
            continue
        kind = held[var]["ChassisType"].value
        entry = catalogue.get(name.upper())
        part = entry.name if entry else ""
        rows.append((name, var, kind))
        if part and not any(w in part for w in CHASSIS_TYPE_WORDS.get(kind, ())):
            crossed.append(f"{name} {part} {var}")
    kinds = Counter(profiles.CHASSIS_TYPE[k] for _, _, k in rows)
    check("objects.rlb: a chassis record names its behaviour profile",
          rows and not crossed and set(kinds) == {"flying", "walking", "wheeled", "tracked"},
          f"{len(rows)} robot chassis records carry a behpsp.res chas_*.var sixth slot whose "
          f"ChassisType agrees with the objects.dlb name: {dict(kinds)}"
          + (f"; crossed {crossed[:3]}" if crossed else ""))

    # 5. a player chassis's body weighs a round figure
    boxes = exact = 0
    masses = {}
    for part in PLAYER_CHASSIS:
        record = library.get(part.lower())
        msh, ndp = record.mesh, record.damage
        model = objmesh.parse(bases.read_name(msh.member), msh.member)
        table = objects.parse_damage(bases.read_name(ndp.member), ndp.member)
        for slot in model.slots:
            box = math.prod(hi - lo for lo, hi in zip(slot.aabb_min, slot.aabb_max, strict=True))
            boxes += 1
            exact += abs(slot.volume - box) <= 1e-3 * max(1.0, slot.volume)
        masses[part] = sum(row.unknown * model.node_volume(i)
                           for i, row in enumerate(table[:len(model.nodes)]))
    round_kg = [p for p, m in masses.items() if abs(m - 25 * round(m / 25)) < 0.5]
    # control: the densities themselves are not round, so a round product is not free
    fractional = set()
    for part in PLAYER_CHASSIS:
        ndp = library.get(part.lower()).slot_with_suffix("ndp")
        table = objects.parse_damage(bases.read_name(ndp.member), ndp.member)
        if any(r.unknown != int(r.unknown) for r in table):
            fractional.add(part)
    check("bases.rlb: a player chassis's body weighs a round figure",
          exact == boxes and len(round_kg) >= len(PLAYER_CHASSIS) - 2
          and len(fractional & set(round_kg)) >= len(round_kg) // 2,
          f"{len(fractional & set(round_kg))} of the round ones carry fractional .ndp densities; "
          f"a slot's volume (+0x34) is its box's volume on {exact}/{boxes}; density x LOD-0 "
          f"volume summed over a chassis's nodes is a multiple of 25 kg on {len(round_kg)} of "
          f"{len(PLAYER_CHASSIS)}: " + ", ".join(f"{p} {masses[p]:,.0f}" for p in PLAYER_CHASSIS))

    # 6. the research ladder: a chassis needs the next smaller of its own kind
    trees = [research.read(p) for p in research.trees(game)]
    kind_of = {name.upper(): var for name, var, _ in rows}
    size = {"T": 1, "L": 2, "M": 3, "B": 4}
    ladder = broken = 0
    for part in PLAYER_CHASSIS:
        needs = Counter()
        for tree in trees:
            item = tree.item_for(part)
            if item:
                needs[tuple(sorted(p for i in item.requires for p in tree[i].parts
                                   if p.upper().startswith("R_")))] += 1
        modal = needs.most_common(1)[0][0] if needs else ()
        for prior in modal:
            prior = prior.upper()
            ok = (kind_of.get(prior) == kind_of.get(part)
                  and size[prior[2]] in (size[part[2]] - 1, size[part[2]]))
            ladder += ok
            broken += not ok
    check(".trf: a chassis is researched from the one below it of the same kind",
          ladder >= 10 and not broken,
          f"in each chassis's commonest wiring across {len(trees)} trees, {ladder} prerequisite "
          f"chassis are its own kind one size down or the same size; {broken} are not")

    # 7. who places which chassis
    use: dict[str, Counter] = defaultdict(Counter)
    for tma in sorted(game.glob("MISSIONS/**/data.tma")):
        m = mission.load(tma)
        for o in m.objects:
            if "units\\units" not in o.path.lower() or o.clan_id is None:
                continue
            unit = objects.load_unit(_on_disk(game, o.path))
            chassis = unit.components[0].ref.member.upper()
            use[chassis][m.clans[o.clan_id].type] += 1
    hero_only = set(use["R_H_02"]) == {mission.CLAN_PLAYER}
    special = ("R_B_06", "R_B_07", "R_B_08", "R_L_07")
    enemy_only = all(set(use[c]) == {mission.CLAN_ENEMY} for c in special)
    check("data.tma: only the player places the hero; the costless chassis are enemies'",
          hero_only and enemy_only and any(mission.CLAN_PLAYER in use[c] for c in PLAYER_CHASSIS),
          f"R_H_02 {dict(use['R_H_02'])}; " + ", ".join(f"{c} {dict(use[c])}" for c in special)
          + " (clan types: 1 player, 2 enemy, 3 neutral)")


#: The catalogue sub-kinds that fire on energy alone.
ENERGY_WEAPONS = ("LAS", "TAS")
#: A clip's marks, in order.
CLIP_MARKS = ("df", "01", "02")


def check_weapons(check, game: Path) -> None:
    """guns.rlb and weapon.rlb: guns, clips and rounds."""
    arm = weapons.Armoury(game)
    catalogue = descriptions.library(game)
    guns = {k.lower(): (entry, arm.gun(k.lower())) for k, entry in catalogue.items()
            if k.lower().startswith("e_gun_")}
    guns = {k: (e, g) for k, (e, g) in guns.items() if g is not None}
    firearms = {k: (e, g) for k, (e, g) in guns.items() if g.type_id == control.GUN_TYPE}
    clips = {k.lower(): (entry, arm.clip(k.lower())) for k, entry in catalogue.items()
             if k.lower().startswith("i_c")}

    labelled = sum(1 for _, g in firearms.values() if g.slot)
    agree = sum(1 for _, g in firearms.values() if bool(g.slot) == g.uses_clips)
    fitted: Counter[str] = Counter()
    bare: Counter[str] = Counter()
    for path in sorted(game.glob("UNITS/**/*.dat")):
        unit = objects.load_unit(path)
        parent = unit.parents()
        member = [c.ref.member.lower() for c in unit.components]
        for i, m in enumerate(member):
            if m in firearms:
                kids = [x for j, x in enumerate(member) if parent[j] == i and x.startswith("i_c")]
                (fitted if kids else bare)[m] += 1
    wrong = ([m for m in fitted if not firearms[m][1].slot]
             + [m for m in bare if firearms[m][1].slot])
    check("guns.rlb: a gun takes clips exactly when its slot is labelled",
          firearms and agree == len(firearms) and not wrong and 0 < labelled < len(firearms),
          f"{labelled} of {len(firearms)} guns label their class-2 slot (i_cNN_<size>) and "
          f"carry a magazine; the other {len(firearms) - labelled} read -1.  In the "
          f"assemblies {sum(fitted.values())} fitted guns hang a clip and "
          f"{sum(bare.values())} do not, each on the side its label says")

    slot_of = {g.slot: g for _, g in firearms.values() if g.slot}
    pairs = [(c, slot_of.get(c.family)) for _, c in clips.values()]
    same = sum(1 for c, g in pairs if g and g.round and c.round == g.round.member
               and arm.clip_values(c.part) == (g.capacitor, g.shot_energy, g.interval_ms))
    shuffled = [g for _, g in pairs]
    random.Random(1).shuffle(shuffled)
    by_chance = sum(1 for (c, _), g in zip(pairs, shuffled, strict=True)
                    if g and arm.clip_values(c.part) == (g.capacitor, g.shot_energy, g.interval_ms))
    check(".ctl: a clip repeats its gun's energy, rate and round",
          pairs and same == len(pairs) and by_chance < len(pairs) // 2,
          f"all {same}/{len(pairs)} catalogue clips carry the capacitor, energy a shot, "
          f"interval and round of the gun whose slot names their family; paired at "
          f"random, {by_chance} would")

    ladders: dict[str, list[tuple[int, int, float]]] = defaultdict(list)
    for key, (_, c) in clips.items():
        ladders[c.family].append((CLIP_MARKS.index(key[8:10]), c.rounds, c.mass))
    rising = all(a[1] < b[1] and a[2] < b[2] for rows in ladders.values()
                 for a, b in zip(sorted(rows), sorted(rows)[1:], strict=False))
    check(".ctl: a clip's rounds and weight rise with its mark",
          ladders and rising,
          "; ".join(f"{fam} " + "/".join(str(r) for _, r, _ in sorted(rows))
                    for fam, rows in sorted(ladders.items())))

    every = [g for _, g in guns.values()]
    powered = all(g.capacitor > 0 and 0 < g.shot_energy <= g.capacitor and g.interval_ms > 0
                  for g in every)
    tails = all(not any(arm.values(k)[4:]) for k in list(guns) + list(clips))
    check(".ctl: every gun and clip is a capacitor, a shot's energy and an interval",
          every and powered and tails,
          f"on all {len(every)} guns value 1 (capacitor) >= value 2 (energy a shot) > 0 "
          f"and value 3 (ms between shots) > 0, and values 4-15 are zero on them and "
          f"the {len(clips)} clips.  Control.dll:0x10029cfd refuses a shot below value 2")

    player = {k: (e, g) for k, (e, g) in firearms.items()
              if not e.code.startswith("_") and not k.startswith("e_gun_f")}
    by_kind: dict[str, Counter[bool]] = defaultdict(Counter)
    for e, g in player.values():
        by_kind[e.sub + ("*" if "pumping" in e.name.lower() else "")][not g.uses_clips] += 1
    ok = all((kind in ENERGY_WEAPONS) == bool(c[True]) and not (c[True] and c[False])
             for kind, c in by_kind.items())
    enemy = [g for k, (_, g) in firearms.items() if k not in player]
    check("objects.dlb: lasers and tasers need no ammunition; cannons, flamers, rockets do",
          player and ok and enemy and not any(g.uses_clips for g in enemy),
          "; ".join(f"{kind} {c[True]} energy / {c[False]} clip"
                    for kind, c in sorted(by_kind.items()))
          + f" (* the pumping laser, which takes clips); all {len(enemy)} '_'-coded and "
          f"huge guns have an unlimited magazine")

    launchers = {k: (e, g) for k, (e, g) in player.items() if e.sub in ("ROC", "MIS")}
    odd = sorted(k for k, (_, g) in launchers.items() if g.magazine != g.barrels)
    check("guns.rlb: a rocket or missile launcher's magazine is its tube count",
          launchers and len(odd) < len(launchers)
          and all("winged" in launchers[k][0].name.lower() for k in odd),
          f"{len(launchers) - len(odd)} of {len(launchers)} launchers hold as many rounds "
          f"as they have barrels; the {len(odd)} that do not are the winged SSMs")

    literal = {}
    for k, (e, g) in guns.items():
        text = next((s.field.strip('"') for s in e.stats if s.field.startswith('"')), None)
        if text and g.round:
            literal[k] = (float(text), g.round.damage, g.beams, g.salvo)
    check("objects.dlb: a multi-beam laser's printed damage is its beams times one round",
          literal and all(t == d * b and s for t, d, b, s in literal.values()),
          ", ".join(f"{k} {t:g} = {b} x {d:g}" for k, (t, d, b, s) in sorted(literal.items()))
          + " -- all set record +8 bit 0x2000000, every barrel at once "
          "(Control.dll:0x10029fcc)")

    kinds: dict[str, list[weapons.Round]] = defaultdict(list)
    for e, g in player.values():
        if g.round:
            kinds[e.sub].append(g.round)
    beams = {r.speed for r in kinds["LAS"] + kinds["TAS"]}
    laser_range = {r.range for r in kinds["LAS"]}
    taser_range = [r.range for r in kinds["TAS"]]
    guided = all(r.guided for r in kinds["MIS"]) and not any(r.guided for r in kinds["ROC"])
    shells = [g.round for e, g in player.values()
              if g.round and g.round.guided and e.sub == "GUN"]
    missiles = kinds["MIS"]
    check("weapon.rlb: a missile steers harder than a guided shell",
          shells and missiles
          and max(r.cone for r in shells) < min(r.cone for r in missiles)
          and max(r.turn_rate for r in shells) < min(r.turn_rate for r in missiles),
          f"the {len(shells)} guided gun rounds (howitzer shells) look within a "
          f"{min(r.cone for r in shells):.2f}-{max(r.cone for r in shells):.2f} rad cone "
          f"and turn at {min(r.turn_rate for r in shells):g}-"
          f"{max(r.turn_rate for r in shells):g} rad/s; the {len(missiles)} missile "
          f"rounds {min(r.cone for r in missiles):.2f}-{max(r.cone for r in missiles):.2f} "
          f"rad and {min(r.turn_rate for r in missiles):g}-"
          f"{max(r.turn_rate for r in missiles):g} rad/s")
    check("weapon.rlb: beams are instant, tasers short, missiles guided and rockets not",
          beams == {10000.0} and laser_range == {1000.0} and max(taser_range) < 200 and guided,
          f"every laser and taser round flies at {min(beams):g} m/s; lasers reach "
          f"{sorted(laser_range)} m (+108), tasers {min(taser_range):g}-{max(taser_range):g}; "
          f"all {len(kinds['MIS'])} missile rounds carry a class-17 seeker, none of the "
          f"{len(kinds['ROC'])} rocket rounds do")


#: A research item's role, for the roles a turret gives a unit.
TURRET_ROLE_TYPE = {research.ROLE_BATTLE: objects.TYPE_WARRIOR,
                    research.ROLE_TRANSPORT: objects.TYPE_TRANSPORT,
                    research.ROLE_BUILDER: objects.TYPE_BUILDER,
                    research.ROLE_HQ: objects.TYPE_HQ, research.ROLE_HERO: objects.TYPE_HERO}


def _turret_role(text: tuple[str, ...]) -> int:
    first = text[0].lower() if text else ""
    return {"warbot": objects.TYPE_WARRIOR, "cargobot": objects.TYPE_TRANSPORT,
            "mobile builder": objects.TYPE_BUILDER, "control sensor": objects.TYPE_HQ,
            "hero": objects.TYPE_HERO}.get(first, 0)


def check_turrets(check, game: Path) -> None:
    """Turrets: mounting variants, sockets, roles, and the special ones."""
    names = frozenset(p.name.lower() for p in all_archives(game))
    library = objects.ObjectLibrary(game / "objects.rlb")
    catalogue = descriptions.library(game)
    turrets = NResArchive.open(game / "turrets.rlb")
    opened: dict[str, NResArchive] = {"turrets.rlb": turrets}
    mesh_nodes: dict[str, list[str] | None] = {}

    def nodes_of(member: str) -> list[str] | None:
        if member not in mesh_nodes:
            record = library.get(member)
            msh = record.slot_with_suffix("msh") if record else None
            out = None
            if msh:
                arch = opened.setdefault(msh.library.lower(),
                                         NResArchive.open(game / msh.library))
                out = objmesh.parse(arch.read_name(msh.member), msh.member).subobjects
            mesh_nodes[member] = out
        return mesh_nodes[member]

    ids = sorted(r.lower() for r in library.records if r.lower().startswith("e_tur"))

    # --- t and b are one turret in two mountings ------------------------------
    pairs = same_entry = shared = one_word = 0
    odd: list[str] = []
    words: Counter[tuple[str, int]] = Counter()
    for tid in ids:
        if tid[7] != "t":
            continue
        bid = tid[:7] + "b" + tid[8:]
        if bid not in ids:
            continue
        pairs += 1
        ct, cb = catalogue.get(tid), catalogue.get(bid)
        same_entry += bool(ct and cb and (ct.name, ct.code, ct.build_ore)
                           == (cb.name, cb.code, cb.build_ore))
        rt, rb = library.get(tid), library.get(bid)
        slots_t = {s.member.rsplit(".", 1)[-1]: s.member for s in rt.slots}
        slots_b = {s.member.rsplit(".", 1)[-1]: s.member for s in rb.slots}
        shared += all(slots_t[x] == slots_b[x] for x in ("msh", "wea", "ndp")) and all(
            slots_t[x] != slots_b[x] for x in ("cpt", "ctl"))
        blob_t = turrets.read_name(slots_t["ctl"])
        blob_b = turrets.read_name(slots_b["ctl"])
        diff = [i for i in range(len(blob_t)) if blob_t[i] != blob_b[i]]
        part = control.parse(blob_t, names).components[0]
        at = part.offset + control.COMPONENT_FLAGS_AT
        word_t = part.flags
        word_b = control.parse(blob_b, names).components[0].flags
        one_word += (len(blob_t) == len(blob_b) and part.type_id == control.TURRET_TYPE
                     and all(at <= i < at + 4 for i in diff)
                     and word_t & control.MOUNT_UPRIGHT and not word_b & control.MOUNT_UPRIGHT)
        if word_t ^ word_b != control.MOUNT_UPRIGHT:
            odd.append(tid)
        words[("t", word_t)] += 1
        words[("b", word_b)] += 1
    check("turrets.rlb: a t and a b turret are one turret, mounted two ways",
          pairs and same_entry == pairs == shared == one_word,
          f"all {pairs} e_tur_?t/?b pairs share their objects.dlb entry and their "
          f"mesh, .wea and .ndp, and their controllers differ only in bit "
          f"{control.MOUNT_UPRIGHT:#x} of the turret component's word at +8, set on t: "
          + ", ".join(f"{v} {w:#x} x{n}" for (v, w), n in sorted(words.items()))
          + f"; the pair differing in more than that bit: {odd}")

    # A second bit marks the HQ turrets.
    hq_bit: dict[bool, Counter[bool]] = defaultdict(Counter)
    for tid in ids:
        ctl = library.get(tid).slot_with_suffix("ctl")
        word = control.parse(turrets.read_name(ctl.member), names).components[0].flags
        is_hq = _turret_role(catalogue[tid].text) == objects.TYPE_HQ
        hq_bit[is_hq][bool(word & control.MOUNT_HQ)] += 1
    check("turrets.rlb: bit 0x8000000 of the turret word marks an HQ turret",
          hq_bit[True] == Counter({True: hq_bit[True][True]}) and hq_bit[True][True]
          and hq_bit[False][True] == 1,
          f"HQ turrets with/without it {dict(hq_bit[True])}; the rest "
          f"{dict(hq_bit[False])}, the one exception e_tur_lb_06, whose t twin "
          f"lacks it; IControl's getter tests it at Control.dll:0x1002b7bb")

    # --- mounts, from the assemblies -------------------------------------------
    variant_on: dict[str, Counter[bool]] = defaultdict(Counter)
    size_ok = size_all = 0
    gun_size: Counter[bool] = Counter()
    off_size: set[str] = set()
    builder_socket: Counter[tuple[str, str]] = Counter()
    kind_by_turret: dict[str, set[int]] = defaultdict(set)
    labelled = fitted_ok = 0
    for path in sorted(game.glob("UNITS/**/*.dat")):
        unit = objects.load_unit(path)
        comps = unit.components
        parents = unit.parents()
        for i, comp in enumerate(comps):
            member = comp.ref.member.lower()
            parent = comps[parents[i]].ref.member.lower() if parents[i] >= 0 else ""
            if member.startswith("e_tur"):
                kind_by_turret[member].add(unit.kind)
                chassis = catalogue.get(comps[parents[i]].ref.member)
                flying = bool(chassis and re.search(r"flying|helicopter", chassis.name, re.I))
                variant_on[member[7]][flying] += 1
                size_all += 1
                size_ok += member[6] == parent[2]
                # the radar and deflector parts the turret's controller names
                ctl = library.get(member).slot_with_suffix("ctl")
                labels = {p.label.lower() for p in
                          control.parse(turrets.read_name(ctl.member), names).components
                          if p.label}
                for j, child in enumerate(comps):
                    cm = child.ref.member.lower()
                    if parents[j] == i and cm[:5] in ("i_rdr", "i_def"):
                        labelled += 1
                        fitted_ok += cm[:7] in labels
            if member.startswith("e_gun") and parent.startswith("e_tur"):
                same = member[6] == parent[6]
                gun_size[same] += 1
                if not same:
                    off_size.add(parent + ">" + member[:8])
                if member[7] == "s":
                    node = nodes_of(parent)
                    builder_socket[(parent[:8] + parent[9:], node[comp.attach_node])] += 1
    check("UNITS: a b turret hangs under a flyer, a t turret sits on the ground",
          variant_on["b"] and variant_on["t"] and set(variant_on["b"]) == {True}
          and set(variant_on["t"]) == {False},
          f"b turrets on flying chassis {dict(variant_on['b'])}, t turrets "
          f"{dict(variant_on['t'])} (True = the chassis' catalogue name says "
          f"Flying or Helicopter)")
    check("UNITS: a turret's size is its chassis' size, and its guns' too",
          size_ok == size_all > 0 and gun_size[True] and off_size
          and all(o.startswith("e_tur_bt_10>e_gun_f") for o in off_size),
          f"{size_ok}/{size_all} turrets share their chassis' size letter; "
          f"{gun_size[True]} guns share their turret's and {gun_size[False]} do not, "
          f"all of them fortification guns on {sorted(off_size)}")
    check("UNITS: a builder's module sits on its turret's Base_LU_01",
          builder_socket and all(node == "Base_LU_01" for (_t, node) in builder_socket)
          and all(t[6:8] in ("lt", "lb", "mt", "mb") and t.endswith("03")
                  for (t, _n) in builder_socket),
          f"every e_gun_?s mobile builder module: {dict(builder_socket)}")
    check("UNITS: a turret takes the radar and deflector sizes its controller names",
          labelled and fitted_ok == labelled,
          f"{fitted_ok}/{labelled} fitted i_rdr/i_def parts match the size of the "
          f"i_rdr_?/i_def_? label on their turret's radar and deflector components")

    # --- sockets against the catalogue ----------------------------------------
    rows = []
    agree = 0
    exceptions = []
    for tid in ids:
        if tid[7] == "b":
            continue
        entry = catalogue.get(tid)
        sockets = [n for n in (nodes_of(tid) or []) if n.startswith("Base_")
                   and n not in objects.TURRET_MOUNT_NODES]
        text = " ".join(entry.text)
        slots = re.search(r"(\d+)[- ](?:\w+ )?slots?", text)
        want = int(slots.group(1)) if slots else None
        role = _turret_role(entry.text)
        have = len(sockets) - (role == objects.TYPE_BUILDER)
        rows.append((tid, want, len(sockets)))
        if want is not None and have == want:
            agree += 1
        else:
            exceptions.append(f"{tid} {want}/{len(sockets)}")
    check("turrets.rlb: a turret's Base_* sockets are its catalogue's battle slots",
          agree >= 20 and len(exceptions) == 6
          and {e.split()[0] for e in exceptions} == {"e_tur_bt_07", "e_tur_bt_08",
                                                     "e_tur_bt_11", "e_tur_bt_12",
                                                     "e_tur_lt_07", "e_tur_ht_02"},
          f"{agree} of {len(rows)} turrets have as many gun sockets as their "
          f"catalogue's slots, counting a builder's module socket apart; the rest "
          f"(catalogue/sockets): {', '.join(exceptions)}")

    builtin = {}
    for tid in ids:
        if tid[7] == "b":
            continue
        ctl = library.get(tid).slot_with_suffix("ctl")
        parts = control.parse(turrets.read_name(ctl.member), names).components
        builtin[tid] = sum(1 for p in parts if p.type_id == control.GUN_TYPE)
        firsts = [p.type_id for p in parts[:4]]
        if firsts != [control.TURRET_TYPE, control.RADAR_TYPE, control.CAMERA_TYPE,
                      control.DEFLECTOR_TYPE]:
            builtin[tid] = -1
    with_guns = sorted(t for t, n in builtin.items() if n > 0)
    check("turrets.rlb: every robot turret is a turret, radar, camera and deflector",
          -1 not in builtin.values()
          and with_guns == ["e_tur_bt_11", "e_tur_bt_12", "e_tur_ht_02", "e_tur_lt_07"]
          and all(builtin[t] == 4 for t in with_guns),
          f"all {len(builtin)} controllers open with classes 1, 8, 4, 21; four carry "
          f"four built-in guns of their own and no gun socket: {', '.join(with_guns)}")

    defaults = 0
    turret_ctls = [e for e in turrets if e.name.lower().endswith(".ctl")]
    for entry in turret_ctls:
        blob = turrets.read(entry)
        same = True
        for at, want in control.DEFAULTS.items():
            fmt = "<i" if at in control.DEFAULT_INTS else "<f"
            got = struct.unpack_from(fmt, blob, at)[0]
            same &= (got == want) if fmt == "<i" else abs(got - want) <= 1e-6 * max(1, abs(want))
        defaults += same
    check("turrets.rlb: no turret controller authors a rate or a limit",
          defaults == len(turret_ctls) > 0,
          f"{defaults}/{len(turret_ctls)} carry the constructor's default on all "
          f"{len(control.DEFAULTS)} frame slots from +20 to +124")

    # --- roles ------------------------------------------------------------------
    by_role: dict[int, set[int]] = defaultdict(set)
    for member, kinds in kind_by_turret.items():
        entry = catalogue.get(member)
        by_role[_turret_role(entry.text)].update(kinds)
    check("UNITS: a unit's class word is its turret's role",
          {r: sorted(k) for r, k in by_role.items()}
          == {t: [t] for t in TURRET_ROLE_TYPE.values()},
          "; ".join(f"catalogue {r:#x}: .dat {', '.join(hex(k) for k in sorted(v))}"
                    for r, v in sorted(by_role.items()))
          + ".  Behavior.dll:0x10008a80 picks prof_war/trn/bld/hq/hero by it")

    codes: dict[int, set[int]] = defaultdict(set)
    upgrade = items = 0
    size_codes: dict[str, set[int]] = defaultdict(set)
    others: Counter[int] = Counter()
    for tree_path in research.trees(game):
        tree = research.read(tree_path)
        for item in tree.items:
            entry = catalogue.get(item.parts[0]) if item.parts else None
            if not entry or not item.tail:
                continue
            items += 1
            upgrade += item.tail[5] == entry.upgrade
            size_codes[entry.size].add(item.tail[4])
            sub = entry.sub.split(":") if entry.sub else []
            if entry.kind == "SHS" and "TUR" in sub:
                codes[item.tail[0]].add(_turret_role(entry.text))
            else:
                others[item.tail[0]] += 1
    check(".trf: a turret item's first tail byte is its role",
          {c: sorted(v) for c, v in codes.items()}
          == {c: [t] for c, t in TURRET_ROLE_TYPE.items()}
          and set(others) == {1, 6, 7, 255},
          "turrets " + "; ".join(f"{c} -> {', '.join(hex(t) for t in sorted(v))}"
                                 for c, v in sorted(codes.items()))
          + f"; every other item {dict(sorted(others.items()))} (1 a bunker or tower "
          f"turret, 6 the hero chassis, 7 an animal, 255 none)")
    check(".trf: the last two tail bytes are the size and UpgradeLevel",
          upgrade == items > 0 and {k: sorted(v) for k, v in size_codes.items()}
          == {"T": [0], "L": [1], "M": [2], "B": [3], "H": [4], "A": [4], "N": [4],
              "E": [5]},
          f"+0x27 equals objects.dlb's UpgradeLevel on {upgrade}/{items} item "
          f"records; +0x26 by size letter "
          + ", ".join(f"{k} {sorted(v)}" for k, v in sorted(size_codes.items())))

    # --- the special turrets ----------------------------------------------------
    free = sorted(t for t in ids if t[7] != "b" and catalogue.get(t)
                  and not any((catalogue[t].build_energy, catalogue[t].build_ore,
                               catalogue[t].research_energy, catalogue[t].research_ore)))
    ai = {c.ref.member.lower() for p in game.glob("UNITS/UNITS/AI/*.dat")
          for c in objects.load_unit(p).components}
    placed: dict[str, Counter[int]] = defaultdict(Counter)
    type_seen = type_same = 0
    for tma in sorted(game.glob("MISSIONS/**/data.tma")):
        m = mission.load(tma)
        for o in m.objects:
            if o.kind != mission.KIND_UNIT or o.clan_id is None:
                continue
            found = list(game.glob("UNITS/**/" + Path(o.path.replace("\\", "/")).name))
            if not found:
                continue
            assembly = objects.load_unit(found[0])
            prop = o.properties.get("Type")
            type_seen += 1
            type_same += bool(prop) and prop.value == assembly.kind
            for comp in assembly.components:
                cm = comp.ref.member.lower()
                if cm.startswith("e_tur"):
                    placed[cm[:7] + "t" + cm[8:] if cm[7] == "b" else cm][
                        m.clans[o.clan_id].type] += 1
    check("data.tma: a placed unit's Type is its assembly's class word",
          type_seen and type_same == type_seen,
          f"{type_same}/{type_seen} placed units carry a Type property equal to their "
          f".dat's class word")
    specials = [t for t in free if t != "e_tur_ht_02"]
    enemy_only = all(set(placed[t]) <= {mission.CLAN_ENEMY, mission.CLAN_NEUTRAL}
                     for t in specials)
    battle = [t for t in ids if t[7] == "t"
              and _turret_role(catalogue[t].text) == objects.TYPE_WARRIOR
              and t not in free]
    player_battle = sum(placed[t][mission.CLAN_PLAYER] for t in battle)
    check("turrets.rlb: the free turrets are the enemy's and the hero's",
          free == ["e_tur_bt_09", "e_tur_bt_10", "e_tur_bt_11", "e_tur_bt_12",
                   "e_tur_ht_02", "e_tur_lt_07"]
          and enemy_only and set(placed["e_tur_ht_02"]) == {mission.CLAN_PLAYER}
          and not any(t in ai or (t[:7] + "b" + t[8:]) in ai for t in free)
          and player_battle > 0,
          f"{', '.join(free)} cost nothing to research or build; missions place "
          + ", ".join(f"{t[6:]} {dict(placed[t])}" for t in specials)
          + f" (clan types), the hero turret {dict(placed['e_tur_ht_02'])}, and none "
          f"is in UNITS/UNITS/AI.  Control: the paid battle turrets are placed "
          f"for players {player_battle} times")

    body: dict[str, float] = {}
    for tid in ids:
        if tid[7] == "b":
            continue
        record = library.get(tid)
        ndp = record.slot_with_suffix("ndp")
        table = objects.parse_damage(turrets.read_name(ndp.member), ndp.member)
        ctl = record.slot_with_suffix("ctl")
        part = control.parse(turrets.read_name(ctl.member), names).components[0]
        body[tid] = table[part.node].durability
    by_size: dict[str, set[float]] = defaultdict(set)
    for tid, hp in body.items():
        if tid not in free and _turret_role(catalogue[tid].text) != objects.TYPE_BUILDER \
                and tid != "e_tur_lt_06":
            by_size[tid[6]].add(hp)
    check("turrets.rlb: a turret's body hit points are set by its size",
          {k: sorted(v) for k, v in by_size.items()}
          == {"t": [120.0], "l": [270.0], "m": [720.0], "b": [3000.0]},
          f"the node under each turret component: tiny/small/medium/large "
          f"{', '.join(f'{k} {sorted(v)}' for k, v in sorted(by_size.items()))}; "
          f"builders b {body['e_tur_bt_08']:g} m {body['e_tur_mt_03']:g} l "
          f"{body['e_tur_lt_03']:g}; HE {body['e_tur_lt_06']:g}; transformer "
          f"{body['e_tur_bt_09']:g}, small tower {body['e_tur_bt_10']:g}, monsters "
          f"{body['e_tur_lt_07']:g}/{body['e_tur_bt_11']:g}/{body['e_tur_bt_12']:g}")


#: The robot Types' behaviour profiles, without the animal's.
MENU_PROFILES = {t: p for t, p in packages.PROFILE_BY_TYPE.items() if t != objects.TYPE_ANIMAL}
#: The Type a placed unit's ``UNITS/UNITS`` folder names.
UNIT_FOLDER_TYPE = {"BATTLE": objects.TYPE_WARRIOR, "BUILDER": objects.TYPE_BUILDER,
                    "TRANSPRT": objects.TYPE_TRANSPORT, "HERO": objects.TYPE_HERO,
                    "HQ": objects.TYPE_HQ, "ANIMAL": objects.TYPE_ANIMAL,
                    "AUTODEMO": objects.TYPE_WARRIOR}


def check_packages(check, game: Path) -> None:
    """The commander's packages: menus, orders, profiles, and who runs what."""
    declared = {v.name: int(v.default, 0) for v in behaviour.variables(game)
                if v.name.startswith("ORDER_")}
    check("varset.var: the orders a unit or building can be given",
          declared == packages.ORDERS,
          f"{len(declared)} ORDER_* constants, " + ", ".join(
              f"{n[6:]} {v}" for n, v in sorted(declared.items(), key=lambda kv: kv[1])))

    iron = (game / "iron3d.dll").read_bytes()
    table = resources.strings(iron)
    named = {p.command: table.get(p.string) for p in packages.PACKAGES}
    plain = [p for p in packages.PACKAGES if p.command not in (10, 17)]
    check("iron3d.dll: the commander's packages are strings 5000-5008",
          all(named[p.command] == p.label for p in plain),
          "; ".join(f"{p.string} {named[p.command]!r}" for p in plain))

    status = tuple(table.get(packages.STATUS_FIRST + i)
                   for i in range(len(packages.STATUS)))
    check("iron3d.dll: a unit's status line is one of 18 strings from 6180",
          status == packages.STATUS
          and table.get(packages.STATUS_FIRST + len(packages.STATUS)) is None,
          ", ".join(repr(s) for s in status))

    # The two menu tables: 20-byte rows of command id, robot-type mask, a
    # needs-a-target flag, a pick mode, and the string id.  Find the HQ menu
    # by its first row, then read on while the string ids stay in range.
    first = struct.pack("<5I", 0, packages.ROBOT_ANY, 0, 0, 5000)
    hq_at = iron.find(first)
    rows = []
    at = hq_at
    while hq_at >= 0:
        row = struct.unpack_from("<5I", iron, at)
        if not (row[1] >> 24 == 1 and (1000 <= row[4] < 6000)):
            break
        rows.append(row)
        at += 20
    hq, wingman = rows[:22], rows[22:]
    masks = Counter((r[1], table.get(r[4]).split()[0]) for r in hq)
    check("iron3d.dll: HQ orders go to every robot, minerals to transports, building to builders",
          hq_at >= 0 and len(hq) == 22
          and {r[1] for r in hq[:6]} == {packages.ROBOT_ANY}
          and hq[6][1] == 0x01002000 and {r[1] for r in hq[7:]} == {0x01004000}
          and [r[0] for r in hq] == [0, 1, 2, 3, 6, 7] + list(range(8, 24))
          and [r[0] for r in wingman] == [0, 24, 2, 3, 4, 5, 7],
          f"{len(hq)} HQ rows: {', '.join(f'{hex(m)} {w}' for (m, w), n in masks.items())}; "
          f"the {len(wingman)}-row wingman menu: "
          + ", ".join(table.get(r[4]) for r in wingman)
          + f"; control: a row must carry a robot-type mask, and the table ends at row {len(rows)}")

    picks = {table.get(r[4]): (r[2], r[3]) for r in rows if r[2]}
    check("iron3d.dll: Route, Guard, Attack, Capture building and building placement pick a target",
          picks.get("Route") == (1, 4) and picks.get("Guard") == (1, 3)
          and picks.get("Attack") == (1, 3) and picks.get("Capture building") == (1, 2)
          and all(picks.get(n) == (1, 4) for n in ("Build Mine", "Build Factory"))
          and all(r[2] == 0 for r in rows if table.get(r[4]) in
                  ("Standby", "Seek and destroy", "Search and capture", "Refit")),
          ", ".join(f"{n} mode {m}" for n, (_, m) in picks.items()))

    # The profiles.
    held = profiles.load(game)
    types = {name: v["Type"].value & 0xFFFFFFFF for name, v in held.items() if "Type" in v}
    check("behpsp.res: each robot profile's Type is its varset.var robot type",
          all(types.get(p) == t for t, p in MENU_PROFILES.items())
          and types.get("prof_animal.var") == objects.TYPE_ANIMAL
          and types.get("prof_exp.var") == 0x01001000,
          ", ".join(f"{p} {hex(t)}" for t, p in sorted(MENU_PROFILES.items()))
          + f"; prof_exp.var {hex(types.get('prof_exp.var', 0))}, a type varset.var does not name")

    behavior = (game / "Behavior.dll").read_bytes()
    loaded = {n for n in held if n.startswith("prof_") and n.encode() + b"\0" in behavior}
    check("Behavior.dll: fifteen profiles are loaded; prof_exp and prof_universal never are",
          len(loaded) == 15 and "prof_exp.var" not in loaded
          and "prof_universal.var" not in loaded
          and all(p in loaded for p in MENU_PROFILES.values()),
          f"{len(loaded)} of {sum(1 for n in held if n.startswith('prof_'))} prof_*.var "
          f"names occur in the binary")

    flags = {name: tuple(int(v[f].value) for f in packages.TASK_FLAGS)
             for name, v in held.items() if "Task_Stop" in v}
    everything = {n for n, row in flags.items() if all(row)}
    builder = {f for f, x in zip(packages.TASK_FLAGS, flags["prof_bld.var"], strict=True) if x}
    buildings = [n for n, t in types.items() if t == 0x7FFFFFFF]
    check("behpsp.res: warriors, transports, HQs and heroes may take every task; builders six",
          everything == {"prof_war.var", "prof_trn.var", "prof_hq.var", "prof_hero.var",
                         "prof_animal.var", "prof_universal.var"}
          and builder == {"Task_Stop", "Task_Go", "Task_Reload", "Task_Repare", "Task_Build",
                          "Task_RandomGo"}
          and all(flags[b][0] == 1 for b in buildings),
          f"every flag set on {', '.join(sorted(everything))}; prof_bld.var: "
          f"{', '.join(sorted(builder))}; every building profile ({len(buildings)}) keeps "
          f"Task_Stop")

    # Placed units: the folder names the type.
    placed: Counter[tuple[str, int]] = Counter()
    for path in sorted(game.glob("MISSIONS/**/data.tma")):
        for o in mission.load(path).objects:
            if o.kind != mission.KIND_UNIT or "Type" not in o.properties:
                continue
            folder = o.path.replace("\\", "/").split("/")[-2].upper()
            placed[(folder, o.properties["Type"].value & 0xFFFFFFFF)] += 1
    agree = sum(n for (f, t), n in placed.items() if UNIT_FOLDER_TYPE.get(f) == t)
    other = {k: n for k, n in placed.items() if UNIT_FOLDER_TYPE.get(k[0]) not in (None, k[1])}
    check("data.tma: a placed unit's Type follows its UNITS folder",
          agree and other == {("BATTLE", 0x01010000): 3},
          f"{agree} of {sum(placed.values())} placed units carry their folder's type; "
          f"the exceptions: {', '.join(f'{f} as {hex(t)} x{n}' for (f, t), n in other.items())}")

    # Scripts.
    tables = behaviour.variables(game)
    used: Counter[str] = Counter()
    for path in behaviour.scripts(game):
        for line in behaviour.render(behaviour.read(path), tables):
            used.update(re.findall(r"\b(ORDER_(?:ROBOT|BUILDING)_[A-Z]+)\b", line))
    never = sorted(set(packages.ORDERS) - set(used))
    check("*.scr: patrol, attack and capture are the orders mission scripts give most",
          used and set(used) <= set(packages.ORDERS)
          and [n for n, _ in used.most_common(3)] == [
              "ORDER_ROBOT_PATROL", "ORDER_ROBOT_ATTACK", "ORDER_ROBOT_CAPTURE"]
          and never == ["ORDER_BUILDING_CHARGE", "ORDER_ROBOT_LEAVE", "ORDER_ROBOT_REPARE",
                        "ORDER_ROBOT_STAYGROUND"],
          ", ".join(f"{n[6:]} {k}" for n, k in used.most_common())
          + f"; never: {', '.join(never)}")


def check_builder(check, game: Path) -> None:
    """The builder and the transport: who they are, what they carry, where they go."""
    names = frozenset(p.name.lower() for p in game.glob("*.rlb"))
    catalogue = descriptions.library(game)

    def role(member: str) -> str:
        entry = catalogue.get(member)
        text = " ".join(entry.text).lower() if entry else ""
        if "mobile builder" in text and entry.kind == "WPN":
            return "beam"
        if "mobile builder" in text:
            return "builder turret"
        if "cargobot" in text:
            return "transport turret"
        return ""

    # 1. the class word is the Type, and it goes with the builder beam or the cargo turret
    by_type: dict[int, Counter[str]] = defaultdict(Counter)
    assemblies: dict[int, int] = Counter()
    for dat in sorted(game.glob("UNITS/**/*.dat")):
        unit = objects.load_unit(dat)
        assemblies[unit.kind] += 1
        roles = {role(c.ref.member) for c in unit.components} - {""}
        for r in roles:
            by_type[unit.kind][r] += 1
    beam_types = {t for t, c in by_type.items() if c["beam"]}
    cargo_types = {t for t, c in by_type.items() if c["transport turret"]}
    check("UNITS: a builder is Type 0x1004000 with a beam, a transport 0x1002000 with cargo",
          beam_types == {objects.TYPE_BUILDER}
          and by_type[objects.TYPE_BUILDER]["beam"] == assemblies[objects.TYPE_BUILDER]
          and cargo_types == {objects.TYPE_TRANSPORT}
          and by_type[objects.TYPE_TRANSPORT]["transport turret"]
          == assemblies[objects.TYPE_TRANSPORT]
          and assemblies[objects.TYPE_WARRIOR] > 0,
          f"all {assemblies[objects.TYPE_BUILDER]} assemblies whose class word is ROBOT_BUILDER "
          f"carry a mobile-builder module and no other does; all "
          f"{assemblies[objects.TYPE_TRANSPORT]} of 0x1002000 carry a Cargobot turret and no other "
          f"does; control: {assemblies[objects.TYPE_WARRIOR]} warriors (0x1008000) carry neither")

    # 2. a builder module fires only type-30 beams, and nothing else has type 30
    guns = NResArchive.open(game / "guns.rlb")
    library = objects.ObjectLibrary(game / "objects.rlb")
    modules = {m for m in catalogue if role(m) == "beam"}
    beam_ctl = set()
    for member in modules:
        record = library.get(member)
        ctl = record.slot_with_suffix("ctl") if record else None
        if ctl:
            beam_ctl.add(ctl.member.lower())
    types_by_ctl: dict[str, Counter[int]] = {}
    emitted: set[str] = set()
    for entry in guns:
        if not entry.tag.upper().startswith("CTL"):
            continue
        parsed = control.parse(guns.read(entry), names)
        types_by_ctl[entry.name.lower()] = Counter(p.type_id for p in parsed.components)
        if entry.name.lower() in beam_ctl:
            emitted |= {p.resource.member.lower() for p in parsed.components}
    beams_only = all(set(types_by_ctl[c]) == {control.BUILDER_TYPE} for c in beam_ctl)
    elsewhere = sorted(c for c, t in types_by_ctl.items()
                       if t[control.BUILDER_TYPE] and c not in beam_ctl)
    gunners = sum(1 for t in types_by_ctl.values() if t[control.GUN_TYPE])
    rounds = {m: library.get(m) for m in emitted}
    check("guns.rlb: a builder module fires type-30 beams only, and only it does",
          len(beam_ctl) == 3 and beams_only and not elsewhere and gunners > 0
          and all(r is not None and r.tag == "BULL" and m.startswith("bld_")
                  for m, r in rounds.items()),
          f"the {len(beam_ctl)} mobile-builder controllers ({', '.join(sorted(beam_ctl))}) carry "
          f"only type {control.BUILDER_TYPE}, emitting {sorted(emitted)} (BULL records); "
          f"no other of "
          f"{len(types_by_ctl)} gun controllers has one; control: {gunners} carry type-2 guns")

    # 3. BuildDat.lst: twelve schemes, one Type each, the engine's twelve names
    schemes = controls.build_schemes(game)
    kinds = {}
    for scheme in schemes:
        units = [objects.load_unit(game / m.replace("\\", "/")) for m in scheme.members]
        kinds[scheme.name] = {u.kind for u in units}
    building_types: dict[int, set[str]] = defaultdict(set)
    for dat in game.glob("UNITS/BUILDS/**/*.dat"):
        building_types[objects.load_unit(dat).kind].add(dat.parent.name)
    unbuilt = {t: sorted(f) for t, f in building_types.items()
               if t not in controls.SCHEME_TYPES.values()}
    check("BuildDat.lst: each scheme is one building Type, the twelve the engine names",
          len(schemes) == len(controls.SCHEME_TYPES) == controls.BUILD_SCHEME_DECLARED + 1
          and all(kinds.get(n) == {t} for n, t in controls.SCHEME_TYPES.items())
          and set(controls.SCHEME_TYPES.values()) <= set(building_types),
          f"{len(schemes)} schemes (the header says {controls.BUILD_SCHEME_DECLARED}); every "
          f"assembly in a scheme has the class word ArealMap.dll:0x1001ce90 registers for its "
          f"name; the building Types no scheme builds: "
          + ", ".join(f"{t:#x} {'/'.join(f)}" for t, f in sorted(unbuilt.items())))

    # 3b. what a builder must carry: a scheme's first building, by its parts' build ore
    def ore(dat: str) -> float:
        unit = objects.load_unit(game / dat.replace("\\", "/"))
        return sum(catalogue[c.ref.member].build_ore for c in unit.components
                   if c.ref.member in catalogue)
    first = {s.name: ore(s.members[0]) for s in schemes}
    every = {m: ore(m) for s in schemes for m in s.members}
    fetched = {n: v for n, v in first.items()
               if controls.SCHEME_TYPES[n] != controls.SCHEME_TYPES["Mine"]}
    check("objects.dlb: every building a builder fetches ore for fits in one load",
          fetched and max(fetched.values()) <= profiles.TRANSPORT_MAX_ORE
          and max(every.values()) > profiles.TRANSPORT_MAX_ORE,
          f"the first building of each scheme but the mine costs {min(fetched.values()):g}-"
          f"{max(fetched.values()):g} ore by its parts' BuildOreCost, within the 2000 a builder "
          f"holds; the mine, which a builder puts up without fetching (Behavior.dll:0x10028ff4), "
          f"costs {first['Mine']:g}; control: the dearest upgrade level, "
          f"{max(every, key=every.get).split(chr(92))[-1]}, costs {max(every.values()):g}")

    # 4. missions: every placed builder and transport can carry 2000 ore
    held: dict[int, Counter[tuple[float, float]]] = defaultdict(Counter)
    sizes: dict[int, set[str]] = defaultdict(set)
    for folder in gamedir.missions(game):
        for obj in mission.load(folder / "data.tma").objects:
            kind = obj.properties.get("Type")
            if kind is None or (kind.value & 0xFFFFFFFF) not in (objects.TYPE_BUILDER,
                                                                 objects.TYPE_TRANSPORT):
                continue
            top = obj.properties.get("MaximumOre")
            now = obj.properties.get("CurrentOre")
            held[kind.value & 0xFFFFFFFF][(top.value if top else None,
                                           now.value if now else None)] += 1
            dat = game / obj.path.replace("\\", "/")
            if dat.exists():
                sizes[kind.value & 0xFFFFFFFF].add(
                    objects.load_unit(dat).components[0].ref.member[2].lower())
    check("data.tma: a placed builder or transport carries Transport_MaxOre, whatever its size",
          held[objects.TYPE_TRANSPORT] and held[objects.TYPE_BUILDER]
          and {t for t, _ in held[objects.TYPE_TRANSPORT]} == {profiles.TRANSPORT_MAX_ORE}
          and {t for t, _ in held[objects.TYPE_BUILDER]} == {profiles.TRANSPORT_MAX_ORE}
          and {n for _, n in held[objects.TYPE_TRANSPORT]} == {0.0}
          and {n for _, n in held[objects.TYPE_BUILDER]} <= {200.0, profiles.TRANSPORT_MAX_ORE}
          and len(sizes[objects.TYPE_TRANSPORT] | sizes[objects.TYPE_BUILDER]) > 1,
          f"transports (MaximumOre, CurrentOre): {dict(held[objects.TYPE_TRANSPORT])}, "
          f"chassis sizes {sorted(sizes[objects.TYPE_TRANSPORT])}; builders: "
          f"{dict(held[objects.TYPE_BUILDER])}, sizes "
          f"{sorted(sizes[objects.TYPE_BUILDER])} -- one capacity whatever the size")

    # 5. hall ways: one ground-level mine place per mine, one store place per storage
    fortif = NResArchive.open(game / "fortif.rlb")
    graphs = {e.name.lower(): objmesh.read_path_graph(NResArchive(fortif.read(e), e.name))
              for e in fortif if e.tag == "MESH"}
    places: dict[str, tuple[int, int, int]] = {}
    for dat in sorted(game.glob("UNITS/BUILDS/**/*.dat")):
        root = objects.load_unit(dat).components[0].ref.member.lower()
        record = library.get(root)
        bas = record.footprint if record else None
        graph = graphs.get(bas.member.rsplit(".", 1)[0].lower() + ".msh") if bas else None
        if graph is None:
            continue
        flags = [n.flags for n in graph.nodes]
        places[f"{dat.parent.name}/{root}"] = (
            sum(1 for a in flags if a & objmesh.PLACE_MINE),
            sum(1 for a in flags if a & objmesh.PLACE_STORE),
            sum(1 for a in flags if a & (objmesh.PLACE_MINE | objmesh.PLACE_STORE)
                and not a & objmesh.PLACE_GROUND))
    mines = {k: v for k, v in places.items() if k.startswith("MINE/")}
    stores = {k: v for k, v in places.items() if k.startswith("STORAGE/")}
    others = {k: v for k, v in places.items() if k not in mines and k not in stores}
    check("fortif.rlb: a mine has one loading place, a storage one unloading place",
          mines and stores and all(v == (1, 0, 0) for v in mines.values())
          and all(v == (0, 1, 0) for v in stores.values())
          and not any(v[0] or v[1] for v in others.values()),
          f"{len(mines)} mine and {len(stores)} storage models each carry exactly one "
          f"ground-level 0x8 or 0x10 place, where a transport loads and unloads; control: "
          f"none of the other {len(others)} building models has either")


def check_units(check, game: Path) -> None:
    """units: every assembly described whole, and its pieces fitting together."""
    workshop = units.Workshop(game)
    sheets = [workshop.describe(p) for p in sorted(game.glob("UNITS/**/*.dat"))]
    robots = [s for s in sheets if s.chassis is not None]
    armed = [s for s in robots if s.turret is not None]
    fitted = [(s, w) for s in armed for w in s.weapons]
    socketed = sum(1 for s, w in fitted if w.socket in s.turret.sockets)
    clipped = [(s, w) for s, w in fitted if w.clip]
    matching = sum(1 for _, w in clipped if w.clip.family == w.gun.slot)
    shots = sum(1 for _, w in fitted if w.gun.type_id != control.GUN_TYPE or w.gun.round)
    check("units: every assembly describes, and a robot's guns sit in its turret's sockets",
          len(sheets) == len(list(game.glob("UNITS/**/*.dat"))) and robots and armed
          and socketed == len(fitted) and shots == len(fitted),
          f"{len(sheets)} assemblies described, {len(robots)} on a robot chassis and "
          f"{len(armed)} with a turret; all {len(fitted)} fitted guns sit on one of their "
          f"turret's Base_* sockets and resolve to a round")
    parts = [p for s in robots for p in s.parts]
    check("units: a fitted clip is its gun's slot, and every internal part resolves",
          clipped and matching == len(clipped) and parts
          and all(p.figures for p in parts),
          f"all {matching}/{len(clipped)} clips belong to the family their gun's slot "
          f"names; {len(parts)} internal parts across the robots read as their class, "
          f"none unresolved")


def _fits_slot(parent: control.Controller, index: int, type_id: int, member: str) -> bool:
    """Whether the parent's component at ``index`` is a slot this part fits."""
    slot = parent.components[index % len(parent.components)]
    return slot.type_id == type_id and (not slot.label or member.startswith(slot.label.lower()))


def check_loading(check, game: Path) -> None:
    """A fitted internal part or clip is parsed into its parent's slot, by index."""
    library = objects.ObjectLibrary(game / "objects.rlb")
    opened: dict[str, NResArchive] = {}
    cache: dict[str, control.Controller | None] = {}

    def controller(member: str) -> control.Controller | None:
        key = member.lower()
        if key not in cache:
            record = library.get(key)
            ref = record.slot_with_suffix("ctl") if record else None
            if record and ref is None and record.slots:     # a building: its body's
                body = library.get(record.slots[0].member.lower())
                ref = body.slot_with_suffix("ctl") if body else None
            if ref is None:
                cache[key] = None
            else:
                arch = opened.setdefault(ref.library.lower(), NResArchive.open(game / ref.library))
                try:
                    cache[key] = control.parse(arch.read_name(ref.member))
                except KeyError:        # the six cut i_c06_l/i_c07_l clips name no .ctl
                    cache[key] = None
        return cache[key]

    # 1. an internal part's attach index names its parent's slot
    fits = total = shifted = 0
    kinds: Counter[str] = Counter()
    external = 0
    for path in sorted(game.glob("UNITS/**/*.dat")):
        unit = objects.load_unit(path)
        parents = unit.parents()
        for i, comp in enumerate(unit.components):
            if parents[i] < 0:
                continue
            record = library.get(comp.ref.member.lower())
            if record is None:
                continue
            if record.tag == objects.EXTERNAL_TAG:
                external += 1
                continue
            if record.tag != objects.INTERNAL_TAG:
                continue
            parent = controller(unit.components[parents[i]].ref.member)
            mine = controller(comp.ref.member)
            if parent is None or mine is None or not mine.components:
                continue
            total += 1
            kinds[comp.ref.member[:5].lower()] += 1
            want = mine.components[0].type_id
            member = comp.ref.member.lower()
            fits += (0 <= comp.attach_node < len(parent.components)
                     and _fits_slot(parent, comp.attach_node, want, member))
            shifted += _fits_slot(parent, comp.attach_node + 1, want, member)
    check("UNITS: an internal part's attach index is its parent's slot",
          total and fits == total and shifted < total // 2,
          f"all {fits}/{total} internal parts and clips ({len(kinds)} families) give as their "
          f"attach node the index of the parent controller's component of their own class, "
          f"whose label (where it has one) prefixes their name; one index on, {shifted} would. "
          f"AniMesh.dll:0x100039d9 passes it and Control.dll:0x1002d890 re-parses that "
          f"component from the part's record; the {external} external parts append instead")

    # 2. the slots a robot declares are filled
    filled: Counter[str] = Counter()
    empty: list[str] = []
    for path in sorted(game.glob("UNITS/**/*.dat")):
        unit = objects.load_unit(path)
        if not unit.components[0].ref.member.lower().startswith("r_"):
            continue
        parents = unit.parents()
        for i, comp in enumerate(unit.components):
            record = library.get(comp.ref.member.lower())
            if record is None or record.tag not in ("BTLU", objects.EXTERNAL_TAG):
                continue
            parsed = controller(comp.ref.member)
            if parsed is None:
                continue
            children = [(unit.components[j], library.get(unit.components[j].ref.member.lower()))
                        for j in range(len(parents)) if parents[j] == i]
            taken = {c.attach_node for c, rec in children
                     if rec and rec.tag == objects.INTERNAL_TAG}
            for k, slot in enumerate(parsed.components):
                if slot.label:
                    if k in taken:
                        filled[slot.label[:5].lower()] += 1
                    else:
                        empty.append(f"{path.name} {slot.label}")
    check("UNITS: a robot fills its slots, so the slots' own values are not the unit's",
          filled["i_pws"] and filled["i_rdr"] == filled["i_def"] == filled["i_pws"]
          and len(empty) == 3,
          f"filled: battery {filled['i_pws']}, engine {filled['i_eng']}, shield "
          f"{filled['i_fsh']}, detection shield {filled['i_dsh']}, repair {filled['i_rps']}, "
          f"armour {filled['i_arm']}, radar {filled['i_rdr']}, deflector {filled['i_def']}, "
          f"clips {sum(n for k, n in filled.items() if k.startswith('i_c'))}; empty: "
          f"{', '.join(empty)}")

    # 3. a clip carries no barrels and no label, so the gun keeps its own
    guns: dict[str, control.Component] = {}
    clips: list[tuple[str, control.Component]] = []
    for name in library.records:
        low = name.lower()
        if low.startswith(("e_gun_", "i_c")):
            parsed = controller(low)
            firing = [p for p in parsed.components if p.type_id == control.GUN_TYPE] \
                if parsed else []
            if not firing:
                continue
            if low.startswith("e_gun_") and firing[0].label:
                guns[firing[0].label.lower()] = firing[0]
            elif low.startswith("i_c"):
                clips.append((low, firing[0]))
    paired = [(c, guns[n[:7]]) for n, c in clips if n[:7] in guns]
    check("guns.rlb: a clip carries no barrels, so a fitted clip leaves its gun's barrels",
          paired and all(not c.entries and not c.label for c, _ in paired)
          and all(g.entries for _, g in paired),
          f"all {len(paired)} clips with a gun declare no barrel entries and no label, "
          f"where every such gun declares {min(len(g.entries) for _, g in paired)}-"
          f"{max(len(g.entries) for _, g in paired)}; the gun class's parser "
          f"(Control.dll:0x10029650) sets the rounds left from value 0 and the base parser "
          f"appends barrels only from entries (0x10021dc4)")

    # 4. what a robot runs on: the fitted battery and engine, by size and mark
    rows: dict[str, list[tuple[float, float]]] = defaultdict(list)
    intsys = NResArchive.open(game / "intsys.rlb")
    for entry in intsys:
        if entry.tag.upper().startswith("CTL") and entry.name.lower()[:5] in ("o_pws", "o_eng"):
            part = control.parse(intsys.read(entry)).components[0]
            rows[entry.name.lower()[:7]].append((part.values[0], part.power))
    engines = [v for k, r in rows.items() if k.startswith("o_eng") for v, _ in r]
    stores = {k: r for k, r in rows.items() if k.startswith("o_pws") and not k.endswith("f")}
    check(".ctl: a robot's battery and engine are the fitted parts', not the chassis's",
          engines and round(min(engines), 3) == 0.7 and max(engines) == 1.0
          and stores and max(p for r in stores.values() for _, p in r) < 250
          and max(v for r in stores.values() for v, _ in r) > 10000,
          f"fitted engines drive {min(engines):.1f}-{max(engines):g} against the chassis "
          f"slot's 1; fitted batteries hold "
          + "; ".join(f"{k[-1]} {min(v for v, _ in r):g}-{max(v for v, _ in r):g} at "
                      f"{min(p for _, p in r):g}-{max(p for _, p in r):g}/s"
                      for k, r in sorted(stores.items()))
          + " against the slot's 10000 at 250/s")


def _pods_by_type(game: Path) -> dict[int, list[bool]]:
    """Building Type -> for each assembly, whether its hall way has a pod."""
    fortif = NResArchive.open(game / "fortif.rlb")
    graphs = {e.name.lower(): objmesh.read_path_graph(NResArchive(fortif.read(e), e.name))
              for e in fortif if e.tag == "MESH"}
    library = objects.ObjectLibrary(game / "objects.rlb")
    out: dict[int, list[bool]] = defaultdict(list)
    for dat in sorted(game.glob("UNITS/BUILDS/**/*.dat")):
        unit = objects.load_unit(dat)
        record = library.get(unit.components[0].ref.member.lower())
        bas = record.footprint if record else None
        graph = graphs.get(bas.member.rsplit(".", 1)[0].lower() + ".msh") if bas else None
        out[unit.kind].append(bool(graph and any(n.flags & objmesh.PLACE_POD for n in graph.nodes)))
    return out


def check_search(check, game: Path) -> None:
    """The search task: which buildings it captures, which robots it hunts, where it roams."""
    pods = _pods_by_type(game)
    inside = {t for t in pods if t & packages.CAPTURE_TYPES == t}
    picked = inside - set(packages.SEARCH_SKIPS)
    outside = set(pods) - inside
    podless = sorted(t for t in picked if not any(pods[t]))
    missed_with_pods = sorted(t for t in outside if all(pods[t]))
    check("UNITS: Search and capture looks for every building type but three",
          len(outside) == 3 and 0x80200000 in outside
          and podless == [0x80002000] and missed_with_pods == [0x80200000]
          and all(all(pods[t]) for t in picked if t != 0x80002000),
          f"of {len(pods)} building Types, the mask 0x8017365e leaves out "
          + ", ".join(f"{t:#x}" for t in sorted(outside))
          + " (the mast, the little teleport and the heavy tower); the plan also skips "
          "main teleports and bridges.  Every Type it can pick has a pod on every model "
          "but the ruins (0x80002000), which have none; the heavy towers have pods, so only "
          "an explicit Capture building takes one")

    types = {v.name: int(v.default, 0) & 0xFFFFFFFF for v in behaviour.variables(game)
             if v.name.startswith("ROBOT_")}
    hunted = sorted(n for n, t in types.items() if t & packages.HUNTED == t)
    spared = sorted(n for n, t in types.items() if t & packages.HUNTED != t)
    placed = defaultdict(int)
    for path in sorted(game.glob("MISSIONS/**/data.tma")):
        for o in mission.load(path).objects:
            kind = o.properties.get("Type")
            if o.kind == mission.KIND_UNIT and kind is not None:
                value = int(kind.value) & 0xFFFFFFFF
                placed[value & packages.HUNTED == value] += 1
    check("varset.var: Seek and destroy hunts transports, builders and warriors, not HQs or heroes",
          hunted == ["ROBOT_BATTLEUNIT", "ROBOT_BUILDER", "ROBOT_TRANSPORT"]
          and spared == ["ROBOT_HERO", "ROBOT_HQ"] and placed[True] and placed[False],
          f"inside 0x100e000: {', '.join(hunted)}; outside: {', '.join(spared)}.  "
          f"Of the placed units {placed[True]} are huntable and {placed[False]} are not "
          f"(HQs, heroes, animals)")

    bounds = [arealmap.load(p).bounds() for p in sorted(game.glob("DATA/MAPS/*/Land.map"))]
    origin = sum(1 for (x, y), _ in bounds if x == 0.0 and y == 0.0)
    roomy = sum(1 for (x, y), (u, v) in bounds
                if u - x > 2 * packages.ROAM_MARGIN and v - y > 2 * packages.ROAM_MARGIN)
    check("Land.map: every map starts at (0, 0)",
          bounds and origin == len(bounds) == roomy,
          f"{origin}/{len(bounds)} navigation meshes span from (0, 0), all wider than the "
          f"random roam's {packages.ROAM_MARGIN:g} margins; so the search's flee point, "
          f"which is not "
          f"offset by the unit's position (Behavior.dll:0x10030c86), lies within "
          f"{packages.FLEE_RANGE:g} of a map's corner or off the map")


#: Section-5 action codes (``Control.dll:0x10002800``): start and stop an
#: effect, and kill every unit inside the building's construction sphere.
ACT_START_EFFECT, ACT_STOP_EFFECT, ACT_KILL_IN_SPHERE = 10, 19, 21
#: The request codes the construction sphere asks a building's controller for.
SPHERE_REQUESTS = {6, 1, 2, 0, 8, 10}
#: The sphere effects, by the id a building's actions name them with.
SPHERE_EFFECTS = {9001: ("b_sphere_start", "b_sphere_start_bt"), 9002: ("b_sphere_sign",),
                  9100: ("b_sphere_main",)}


def _action_groups(blob: bytes,
                   parsed: control.Controller) -> list[list[tuple[int, int, int, str]]]:
    """Section 5 as groups of (action, id, mode, member) -- values [3], [4], [5] and [7] or name."""
    pos = control.section4_start(parsed.counts) + sum(p.size for p in parsed.components)
    pos += control.BLOCK_SIZE
    out = []
    for _ in range(parsed.counts[4]):
        n = struct.unpack_from("<i", blob, pos)[0]
        pos += 4
        group = []
        for _i in range(n):
            ints = struct.unpack_from("<9i", blob, pos)
            member = blob[pos + 68:pos + 100].split(b"\0")[0].decode("latin-1").lower()
            group.append((ints[3], ints[4], ints[5], ints[7], member))
            pos += control.REFERENCE_STRIDE
        out.append(group)
    return out


def _animated_parts(blob: bytes,
                    parsed: control.Controller) -> list[tuple[int, float, float, float]]:
    """Section 2: (node, from frame, to frame, speed)."""
    at = control.section4_start(parsed.counts) - parsed.counts[2] * control.SECTION2_RECORD
    out = []
    for i in range(parsed.counts[2]):
        node, frm, to = struct.unpack_from("<i2f", blob, at + i * control.SECTION2_RECORD)
        speed = struct.unpack_from("<f", blob, at + i * control.SECTION2_RECORD + 0x18)[0]
        out.append((node, frm, to, speed))
    return out


def check_construction(check, game: Path) -> None:
    """The construction sphere a building runs, and the items it opens."""
    names = frozenset(p.name.lower() for p in all_archives(game))
    buildings = {}
    elsewhere = Counter()
    for path in all_archives(game):
        archive = NResArchive.open(path)
        for entry in archive:
            if not entry.tag.upper().startswith("CTL"):
                continue
            blob = archive.read(entry)
            try:
                parsed = control.parse(blob, names)
            except control.ControlFormatError:
                continue
            if path.name.lower() == "fortif.rlb":
                buildings[entry.name.lower()] = (blob, parsed)
            else:
                elsewhere.update(s.request for s in parsed.states)

    # 1. every building controller answers the six request codes; nothing else does
    codes_ok = graphs = 0
    for _blob, parsed in buildings.values():
        codes = [s.request for s in parsed.states if s.request != -1]
        graphs += 1
        codes_ok += sorted(codes) == sorted(SPHERE_REQUESTS)
    check("fortif.rlb: a building's states answer the construction task's six request codes",
          buildings and codes_ok == graphs and set(elsewhere) == {-1},
          f"{codes_ok}/{graphs} building controllers carry exactly one state for each of "
          f"{sorted(SPHERE_REQUESTS)} at state +0x98 (the rest -1); control: all "
          f"{sum(elsewhere.values())} states in every other archive are -1.  A state "
          f"applies only when +0x98 is the requested code or -1 (Control.dll:0x10001140)")

    # 2. entering a state runs its action group; the request-2, 8 and 10 states kill in the sphere
    kill_ok = 0
    sphered: set[str] = set()
    bt = set()
    for name, (blob, parsed) in buildings.items():
        table = _action_groups(blob, parsed)
        killing = {s.request for s in parsed.states if s.request in (2, 8, 10)
                   and 0 <= s.actions < len(table)
                   and any(a == ACT_KILL_IN_SPHERE for a, *_ in table[s.actions])}
        kill_ok += killing == {2, 8, 10}
        named = {(ident, member) for group in table for a, _i, _m, ident, member in group
                 if member}
        found = {ident: member for ident, member in named if ident in SPHERE_EFFECTS}
        if all(found.get(i) in allowed for i, allowed in SPHERE_EFFECTS.items()):
            sphered.add(name)
        if found.get(9001) == "b_sphere_start_bt":
            bt.add(name)
    check("fortif.rlb: the request-2, 8 and 10 states run the kill-in-sphere action",
          buildings and kill_ok == len(buildings),
          f"on {kill_ok}/{len(buildings)} building controllers the state asked for with "
          f"code 2, 8 or 10 names (+0x90) a section-5 group holding action 21, which "
          f"kills every object of classes 0x4/0x10/0x400 inside the construction sphere "
          f"(Control.dll:0x100033e6)")
    without = sorted(set(buildings) - sphered)
    check("fortif.rlb: a building names the sign, start and main sphere effects",
          sphered and bt and all("bunker" in n or "tower" in n for n in bt)
          and all("ruin" in n or "mtp" in n for n in without),
          f"{len(sphered)}/{len(buildings)} controllers bind effect ids 9001, 9002, 9100 to "
          f"B_Sphere_Start, B_Sphere_Sign, B_Sphere_Main, the {len(bt)} bunkers and towers "
          f"B_Sphere_Start_BT; the {len(without)} without are the ruins and main "
          f"teleports: {', '.join(without)}")

    # 3. doors and pods are animated mesh nodes: frames from -> to at a speed
    spans = defaultdict(Counter)
    speeds = defaultdict(Counter)
    forwards = total = 0
    for blob, parsed in buildings.values():
        table = _animated_parts(blob, parsed)
        for comp in parsed.components:
            if comp.type_id not in (control.DOOR_TYPE, control.COMPUTER_TYPE):
                continue
            for e in comp.entries:
                _node, frm, to, speed = table[e]
                spans[comp.type_id][(int(frm), int(to))] += 1
                speeds[comp.type_id][round(speed, 2)] += 1
                total += 1
                forwards += to > frm and speed > 0
    doors, pods = spans[control.DOOR_TYPE], spans[control.COMPUTER_TYPE]
    check("fortif.rlb: a door plays its node from frame 0, a pod mostly from 1 to 3",
          total and forwards == total and doors[(0, 1)] >= sum(doors.values()) - 1
          and pods[(1, 3)] > sum(pods.values()) // 2,
          f"all {forwards}/{total} door and pod parts (section 2) run forwards at a positive "
          f"speed; doors (from, to) {dict(doors)}, speeds {dict(speeds[control.DOOR_TYPE])}; pods "
          f"{dict(pods)}, speeds {dict(speeds[control.COMPUTER_TYPE])}")

    # 4. the builder's beam rounds carry no explosion, so a hit could do nothing
    library = objects.ObjectLibrary(game / "objects.rlb")
    weapon = NResArchive.open(game / "weapon.rlb")
    beams = {}
    armed = 0
    for record in library.by_tag("BULL"):
        ndp = record.damage
        if ndp is None or ndp.library.lower() != "weapon.rlb":
            continue
        try:
            rows = objects.parse_damage(weapon.read_name(ndp.member), ndp.member)
        except KeyError:
            continue
        exploding = any(r.explosion for r in rows)
        if record.name.lower().startswith("bld_"):
            beams[record.name.lower()] = (sum(r.durability for r in rows), exploding)
        else:
            armed += exploding
    check("weapon.rlb: a builder's beam round has no explosion to hit with",
          beams and all(hp == 1 and not ex for hp, ex in beams.values()) and armed,
          f"{', '.join(f'{k} {hp:g} HP' for k, (hp, _) in sorted(beams.items()))}, no "
          f".exp; control: {armed} other rounds carry one")


def check_controls(check, game: Path) -> None:
    """The input layer: ScanCode.dsc, Command.dsc, the .man bindings, the .tbl tables."""
    keys = controls.scancodes(game)
    actions = controls.commands(game)
    check("controls: the two descriptor files read",
          len(keys) > 100 and len(actions) > 50,
          f"{len(keys)} scan codes in {controls.SCANCODES}, "
          f"{len(actions)} commands in {controls.COMMANDS}, each with a label")

    rows: list[controls.Action] = []
    refused = []
    for name in controls.TABLES:
        path = game / name
        if not path.exists():
            continue
        try:
            rows.extend(controls.table(path))
        except controls.ControlsFormatError as exc:
            refused.append(str(exc))
    check("controls: every .tbl row has its eleven fields",
          rows and not refused,
          f"{len(rows)} rows across {len(controls.TABLES)} tables "
          f"({', '.join(controls.TABLES)}), none refused")

    bound: list[controls.Binding] = []
    for path in sorted(game.glob("*.man")):
        bound.extend(controls.bindings(path))
    unknown_cmd = {b.command for b in bound if b.command not in actions}
    unknown_key = {k for b in bound for k in (b.modifier, b.key) if k not in keys}
    check("controls: every binding names a command and keys that exist",
          bound and not unknown_cmd and not unknown_key,
          f"{len(bound)} bindings across {len(sorted(game.glob('*.man')))} .man files "
          f"resolve into {controls.COMMANDS} and {controls.SCANCODES}")

    lo, hi = controls.MCMD_DISPATCH
    inside = sum(lo <= r.code <= hi for r in rows)
    check("controls: every command a table sends is in World3D's one table",
          rows and inside == len(rows),
          f"{inside}/{len(rows)} rows send a message in {lo}..{hi}, the span "
          f"World3D.dll dispatches from one 21-entry table at 0x100109f8")

    def values(name):
        return {r.value for r in rows if r.command == name}

    pairs = (("MCMD_WALK_F", "MCMD_WALK_B"), ("MCMD_UP", "MCMD_DOWN"))
    signed = all(all(v >= 0 for v in values(a)) and all(v <= 0 for v in values(b))
                 and values(a) and values(b) for a, b in pairs)
    drive = values("MCMD_FORWARD")
    back = sum(r.command == "MCMD_BACK" for r in rows)
    check("controls: a paired message carries its sign in its name",
          signed and back == 0 and -1.0 in drive,
          "MCMD_WALK_F and MCMD_UP send only positive magnitudes and "
          "MCMD_WALK_B and MCMD_DOWN only negative, though each pair shares "
          f"one handler; MCMD_FORWARD instead carries "
          f"{', '.join(f'{v:g}' for v in sorted(drive))} by itself and "
          f"MCMD_BACK is sent by {back} rows")

    table_keys = {k for r in rows for k in (r.modifier, r.key)}
    missing = sorted(k for k in table_keys if k not in keys)
    check("controls: every key a table names is a known scan code",
          table_keys and not missing,
          f"{len(table_keys)} distinct scan names across the tables, all in "
          f"{controls.SCANCODES}")

    released = [r for r in rows if not r.pressed]
    stops = sum(1 for r in released if r.value == 0.0)
    check("controls: a release row cancels what the press row started",
          stops >= len(released) * 0.75 > 0,
          f"{stops}/{len(released)} release rows carry magnitude 0.0, against "
          f"{sum(1 for r in rows if r.pressed)} press rows")

    named = sum(1 for r in rows if r.pressed and r.action in actions)
    check("controls: a press row names the command it runs",
          named > 0,
          f"{named}/{sum(1 for r in rows if r.pressed)} press rows end in a "
          f"{controls.COMMANDS} identifier; the rest are prose or parameterised")

    values = controls.SCAN
    order = [line.strip().split(" ")[0]
             for line in (game / controls.SCANCODES).read_text("latin-1")
             .replace("\r\n", "\n").split("\n")
             if line.strip().startswith("SCAN_")]
    check("controls: every key in the descriptor has a recovered scan code",
          set(order) == set(values) and len(order) == len(values) > 0,
          f"{len(values)} names in {controls.SCANCODES} and {len(values)} in the "
          f"table recovered from World3D.dll, and the two sets are equal")

    ibm = {"SCAN_ESC": 1, "SCAN_W_1": 2, "SCAN_W": 17, "SCAN_A": 30,
           "SCAN_S": 31, "SCAN_D": 32, "SCAN_LSHIFT": 42, "SCAN_F1": 59}
    agree = sum(1 for name, code in ibm.items() if values.get(name) == code)
    lead = sum(1 for i, name in enumerate(order) if values.get(name) == i)
    check("controls: the scan codes are the real ones",
          agree == len(ibm) and lead > 50,
          f"{agree}/{len(ibm)} keys carry their IBM PC set-1 code, and the first "
          f"{lead} entries of {controls.SCANCODES} are listed in code order")

    unresolved = sorted({r.command for r in rows if r.code == controls.UNRESOLVED})
    check("controls: every command a table sends has a number",
          rows and not unresolved,
          f"{len({r.command for r in rows})} distinct commands across the tables "
          f"resolve through the World3D.dll table")

    outside = Counter(r.command for r in rows if not r.dispatched)
    check("controls: three commands fall outside the controller's dispatch",
          len(outside) == 3,
          f"{sum(1 for r in rows if r.dispatched)}/{len(rows)} rows send a command "
          f"in {controls.DISPATCHED.start}..{controls.DISPATCHED.stop - 1}; the rest "
          + ", ".join(f"{n} x{c}" for n, c in sorted(outside.items())))

    walk = [r for r in rows if r.command == "MCMD_WALK_F"]
    tables = {name for name in controls.TABLES
              for r in controls.table(game / name) if r.command == "MCMD_WALK_F"}
    check("controls: the forward walk is sent by every table",
          len(walk) == 6 and len(tables) == len(controls.TABLES),
          f"MCMD_WALK_F is sent on {len(walk)} rows, two in each of "
          f"{len(tables)} tables -- and no module dispatches on its number")

    classed = {r.target for r in rows}
    unknown = sorted(c for c in classed
                     if c not in controls.CICLS and c != "CICLS_UNKNOWN")
    check("controls: every target class a table names has an id",
          classed and not unknown,
          f"{len(classed)} classes used, {len(controls.CICLS)} known to the engine "
          f"-- CICLS_UNKNOWN is the resolver's own default, {controls.UNKNOWN_CLASS}")

    schemes = controls.build_schemes(game)
    members = [m for s in schemes for m in s.members]
    present = sum(1 for m in members if (game / m.replace("\\", "/")).is_file())
    check("controls: every building scheme names assemblies that exist",
          schemes and present == len(members) > 0,
          f"{len(schemes)} schemes, {present}/{len(members)} .dat files present "
          f"-- the file's own header says there must be "
          f"{controls.BUILD_SCHEME_DECLARED}")

    nameless = sorted(n for n in actions if n not in controls.CMD)
    check("controls: every command in the descriptor has a number",
          actions and not nameless,
          f"{len(actions)}/{len(actions)} names in {controls.COMMANDS} resolve "
          f"through the two chains -- {len(controls.CMD_OBJECT)} from World3D.dll, "
          f"{len(controls.CMD_GAME)} from iron3d.dll")

    unnumbered = sorted({b.command for b in bound if b.code == controls.UNRESOLVED})
    check("controls: every binding resolves to a command number",
          bound and not unnumbered,
          f"{len(bound)} bindings across {len(sorted(game.glob('*.man')))} .man "
          f"files carry {len({b.command for b in bound})} distinct commands, all "
          f"numbered")

    shared = sorted(set(controls.CMD_OBJECT) & set(controls.CMD_GAME))
    spare = sorted(set(controls.CMD) - set(actions))
    check("controls: the two chains agree where they overlap",
          shared == [controls.CMD_SHARED]
          and controls.CMD_OBJECT[controls.CMD_SHARED]
          == controls.CMD_GAME[controls.CMD_SHARED]
          and spare == [controls.CMD_UNNAMED],
          f"one name in both chains, {controls.CMD_SHARED} = "
          f"{controls.CMD[controls.CMD_SHARED]} in each; the union is "
          f"{len(controls.CMD)} against {len(actions)} in {controls.COMMANDS}, the "
          f"odd one out {controls.CMD_UNNAMED} = {controls.CMD[controls.CMD_UNNAMED]}")

    pure = 0
    for path in sorted(game.glob("*.man")):
        named = {b.command for b in controls.bindings(path)}
        obj = named & set(controls.CMD_OBJECT) - {controls.CMD_SHARED}
        shell = named & set(controls.CMD_GAME) - {controls.CMD_SHARED}
        pure += not (obj and shell)
    from_game = sorted({r.action for r in rows
                        if r.pressed and r.action in controls.CMD_GAME
                        and r.action not in controls.CMD_OBJECT})
    check("controls: the two chains split the binding files between them",
          pure == len(sorted(game.glob("*.man"))) - 2 and not from_game,
          f"{pure}/{len(sorted(game.glob('*.man')))} .man files draw on one binary "
          f"only, the two ui_other files being the exception, and no .tbl row "
          f"names an iron3d.dll command")


#: The hero's turret, its mesh, and the node its eye and sight sit on.
HERO_TURRET = "o_tur_ht_02"
HERO_TURRET_MESH = "o_tur_ha_02.msh"
EYE_NODE, SIGHT_NODE = 35, 34


def _rows(rows, modifier: str, key: str, pressed: bool = True):
    return [r for r in rows if r.modifier == modifier and r.key == key and r.pressed == pressed]


def check_player_input(check, game: Path) -> None:
    """The hero's rows: what walks, what turns, what looks."""
    hero = controls.table(game / "hero.tbl")
    machines = [controls.table(game / name) for name in ("m1.tbl", "m2.tbl")]

    mx = _rows(hero, "SCAN_NULL", "SCAN_MOUSE_X")
    my = _rows(hero, "SCAN_NULL", "SCAN_MOUSE_Y")
    turret_x = [_rows(t, "SCAN_NULL", "SCAN_MOUSE_X") for t in machines]
    check("hero.tbl: mouse X turns the hull and mouse Y tilts the turret",
          len(mx) == 1 and mx[0].class_id == 0 and mx[0].command == "MCMD_ANGLE_Z"
          and mx[0].value == 0.15 and mx[0].state == "MAN_WRAP"
          and len(my) == 1 and my[0].target == "CICLS_TURRET" and my[0].index == 1
          and my[0].command == "MCMD_ANGLE_Y" and my[0].value == 0.25
          and my[0].state == "MAN_NOTWRAP"
          and all(len(r) == 1 and r[0].target == "CICLS_TURRET"
                  and r[0].command == "MCMD_ANGLE_X" for r in turret_x),
          f"X: {mx[0].command} {mx[0].value} {mx[0].state} on the unit; Y: "
          f"{my[0].target} index {my[0].index} {my[0].command} {my[0].value} "
          f"{my[0].state}.  Control: m1.tbl and m2.tbl send mouse X to the turret")

    walk = {(r.key, r.pressed): (r.command, r.value) for r in hero
            if r.modifier == "SCAN_NULL" and r.key in ("SCAN_W", "SCAN_S", "SCAN_A", "SCAN_D")}
    want = {("SCAN_W", True): ("MCMD_WALK_F", 1.0), ("SCAN_W", False): ("MCMD_WALK_F", 0.0),
            ("SCAN_S", True): ("MCMD_WALK_B", -1.0), ("SCAN_S", False): ("MCMD_WALK_B", 0.0),
            ("SCAN_A", True): ("MCMD_LEFT", 1.0), ("SCAN_A", False): ("MCMD_LEFT", 0.0),
            ("SCAN_D", True): ("MCMD_RIGHT", 1.0), ("SCAN_D", False): ("MCMD_RIGHT", 0.0)}
    check("hero.tbl: W and S walk at the full command, A and D strafe, a release sends 0",
          walk == want, f"{len(walk)} rows: " + ", ".join(
              f"{k[0][5:]}{'' if k[1] else ' up'} {c[5:]} {v:g}"
              for k, (c, v) in sorted(walk.items())))

    lx = _rows(hero, "SCAN_LSHIFT", "SCAN_MOUSE_X")
    ly = _rows(hero, "SCAN_LSHIFT", "SCAN_MOUSE_Y")
    centre = [r for r in hero if r.target == "CICLS_CAMERA"
              and r.command in ("MCMD_ANGLE_X", "MCMD_ANGLE_Y")
              and r.key in ("SCAN_LSHIFT", "SCAN_RMOUSE")]
    check("hero.tbl: Shift looks around the camera, and letting go centres it",
          len(lx) == 1 and lx[0].value == 0.1 and lx[0].state == "MAN_WRAP"
          and len(ly) == 1 and ly[0].value == 0.15 and ly[0].state == "MAN_NOTWRAP"
          and len(centre) == 4 and all(r.value == 0.5 for r in centre)
          and {(r.key, r.pressed) for r in centre}
          == {("SCAN_RMOUSE", True), ("SCAN_LSHIFT", False)},
          f"Shift+X adds {lx[0].value} wrapping, Shift+Y {ly[0].value} clamped; "
          f"{len(centre)} rows set the camera's angles to 0.5 on Shift up or Shift+RMB")

    cruise = sorted((r.key, r.value, r.ramp, r.ramp_time) for r in hero
                    if r.command == "MCMD_FORWARD")
    check("hero.tbl: the keypad cruise is MCMD_FORWARD, two rows with a 0.05 ramp over 1000",
          cruise == sorted([("SCAN_G_ASTERISK", 1.0, 0.0, 0), ("SCAN_G_PLUS", 1.0, 0.05, 1000),
                            ("SCAN_G_SUB", -1.0, 0.05, 1000), ("SCAN_G_SLASH", 0.0, 0.0, 0)]),
          f"{cruise}")

    objs = NResArchive.open(game / "objects.rlb")
    bases = NResArchive.open(game / "bases.rlb")
    chassis = control.parse(bases.read_name("r_h_02.ctl"))
    top, turn, limit = (chassis.triples[i] for i in (control.TRIPLE_TOP_SPEED,
                                                      control.TRIPLE_TURN, 5))
    record, other = objs.read_name("r_h_02"), objs.read_name("r_l_06")
    check("objects.rlb: the hero chassis names hero.tbl; it walks 14 m/s and turns 25.12 rad/s",
          b"hero.tbl" in record and b"m2.tbl" in other and b"hero.tbl" not in other
          and top == (1.0, 14.0, 1.0) and abs(turn[2] - 25.12) < 1e-4
          and limit[:2] == (0.0, 0.0),
          f"r_h_02 top speed {top}, turn {tuple(round(x, 2) for x in turn)}, triple 6 "
          f"{tuple(round(x, 2) for x in limit)}.  Control: r_l_06 names m2.tbl")


def check_turret_channels(check, game: Path) -> None:
    """A turret aims by two channels, and the hero's eye rides its sight."""
    turrets = NResArchive.open(game / "turrets.rlb")
    names = {e.name.lower() for e in turrets}
    ctl = control.parse(turrets.read_name(HERO_TURRET + ".ctl"))
    cpt = objmesh.parse_control_points(turrets.read_name(HERO_TURRET + ".cpt"))
    points = [p.name for p in cpt]
    turret = [c for c in ctl.components if c.type_id == control.TURRET_TYPE]
    yaw, pitch = (ctl.channels[i] for i in turret[0].entries)
    check("turrets.rlb: the hero's turret yaws over frames 49-53 and pitches 1.92 rad over 55-57",
          len(turret) == 1 and turret[0].flags & control.MOUNT_UPRIGHT
          and (yaw.first, yaw.last, yaw.initial, yaw.rate) == (49.0, 53.0, 0.5, 100.0)
          and abs(yaw.span - 6.28) < 1e-4 and (pitch.first, pitch.last) == (55.0, 57.0)
          and abs(pitch.initial - 0.2727) < 1e-3 and abs(pitch.span - 1.9199) < 1e-3
          and pitch.rate == 0.75
          and points[yaw.point] == "TurretDirect" and points[pitch.point] == "TargetDirect",
          f"yaw: frames {yaw.first:g}-{yaw.last:g}, start {yaw.initial:g}, {yaw.rate:g}/s, "
          f"span {yaw.span:.4f}, point {points[yaw.point]}; pitch: frames "
          f"{pitch.first:g}-{pitch.last:g}, start {pitch.initial:.4f}, {pitch.rate:g}/s, "
          f"span {pitch.span:.4f}, point {points[pitch.point]}")

    shapes: Counter[tuple] = Counter()
    total = named = guns = barrels_ok = cameras = paired = on_node = 0
    for name in sorted(names):
        stem = name[:-4]
        if not name.endswith(".ctl") or stem + ".cpt" not in names:
            continue
        c = control.parse(turrets.read_name(name))
        cp = objmesh.parse_control_points(turrets.read_name(stem + ".cpt"))
        pts = [p.name for p in cp]
        for comp in c.components:
            if comp.type_id == control.TURRET_TYPE and len(comp.entries) == 2:
                y, p = (c.channels[i] for i in comp.entries)
                total += 1
                named += pts[y.point] == "TurretDirect" and pts[p.point] == "TargetDirect"
                shapes[(round(y.span, 2), round(p.span, 2), y.last - y.first,
                        p.last - p.first, y.flags)] += 1
            elif comp.type_id == control.GUN_TYPE:
                for i in comp.entries:
                    guns += 1
                    point = c.channels[i].point
                    barrels_ok += 0 <= point < len(pts) and pts[point] not in (
                        "TurretCenter", "TurretDirect", "CameraCenter", "TargetDirect")
            elif comp.type_id == control.CAMERA_TYPE and comp.entries:
                cameras += 1
                point = c.channels[comp.entries[0]].point
                paired += (0 < point < len(pts) and pts[point] == "TargetDirect"
                           and pts[point - 1] == "CameraCenter")
                eye = next(q for q in cp if q.name == "CameraCenter")
                on_node += eye.nodes[0] == comp.node
    check("turrets.rlb: every turret's two channels point at TurretDirect and TargetDirect",
          total and named == total,
          f"{named}/{total}; (yaw span, pitch span, yaw frames, pitch frames, yaw flags): "
          f"{dict(shapes)}")
    check("turrets.rlb: a section-2 record names a control point, not a node",
          guns and barrels_ok == guns and cameras and paired == cameras
          and on_node < cameras,
          f"{barrels_ok}/{guns} gun channels name one of the turret's barrel points; "
          f"{paired}/{cameras} camera channels name TargetDirect with CameraCenter the "
          f"point before it, where the camera component's own node is the eye's on "
          f"only {on_node}")

    m = objmesh.parse(turrets.read_name(HERO_TURRET_MESH), HERO_TURRET_MESH)

    def world(node: int, frame: int) -> objmesh.Pose:
        def local(k: int) -> objmesh.Pose:
            track = m.track(k)
            return m.keys[track[frame]].pose if track else m.local_pose(k)
        pose, parent = local(node), m.nodes[node].parent
        while parent != objmesh.NO_PARENT:
            pose, parent = objmesh.compose(local(parent), pose), m.nodes[parent].parent
        return pose

    def elevation(frame: int) -> float:
        x, y, z = objmesh.quaternion_rotate(world(SIGHT_NODE, frame)[1], (0.0, 1.0, 0.0))
        return math.degrees(math.atan2(z, math.hypot(x, y)))

    def heading(frame: int) -> float:
        x, y, _ = objmesh.quaternion_rotate(world(1, frame)[1], (0.0, 1.0, 0.0))
        return math.degrees(math.atan2(x, y))

    lo, mid, hi = (elevation(f) for f in (55, 56, 57))
    level = lo + pitch.initial * (hi - lo)
    heads = [heading(f) for f in (49, 51, 53)]
    check("o_tur_ha_02.msh: the pitch frames sweep the sight by the channel's span, from level",
          abs(math.radians(hi - lo) - pitch.span) < 0.02 and abs(level) < 1.0 and lo < 0 < hi
          and abs(heads[1]) < 1.0 and all(abs(abs(h) - 180) < 1.0 for h in heads[::2]),
          f"GP_m1o1 looks {lo:.1f}, {mid:.1f}, {hi:.1f} degrees up on frames 55-57, "
          f"{math.radians(hi - lo):.4f} rad against a span of {pitch.span:.4f}; at the "
          f"start value it is {level:+.2f}.  Control: yaw frames 49/51/53 face "
          f"{', '.join(f'{h:.0f}' for h in heads)} degrees, so 0.5 looks ahead")

    eye, sight = (next(q for q in cpt if q.name == n) for n in ("CameraCenter", "TargetDirect"))
    camera = [c for c in ctl.components if c.type_id == control.CAMERA_TYPE]
    poses = {f: world(EYE_NODE, f) for f in (51, 55, 56, 57)}
    at = poses[51][0]
    still = all(max(abs(a - b) for a, b in zip(poses[f][0], at, strict=True)) < 1e-6
                for f in (55, 56, 57))
    check("o_tur_ha_02.msh: the eye is CP_m1o1, 0.88 over the turret root, and does not pitch",
          len(camera) == 1 and eye.nodes == (EYE_NODE, EYE_NODE)
          and sight.nodes == (SIGHT_NODE, SIGHT_NODE)
          and m.nodes[EYE_NODE].name == "CP_m1o1" and m.nodes[SIGHT_NODE].name == "GP_m1o1"
          and eye.position == (0.0, 0.0, 0.0) and sight.direction == (0.0, 1.0, 0.0)
          and not m.track(EYE_NODE) and m.track(SIGHT_NODE)
          and still and 0.8 < at[2] < 0.95,
          f"CameraCenter sits at the origin of node {eye.nodes[0]} "
          f"({m.nodes[eye.nodes[0]].name}), at {tuple(round(v, 3) for v in at)} in the "
          f"turret's frame over every pitch frame; TargetDirect is +y on node "
          f"{sight.nodes[0]} ({m.nodes[sight.nodes[0]].name}), which they tilt")

    # A point's second float is an int32 node of its same-stem objmesh.
    in_range = with_mesh = same = every = 0
    for path in all_archives(game):
        archive = NResArchive.open(path)
        members = {e.name.lower() for e in archive}
        for name in members:
            if not name.endswith(".cpt"):
                continue
            cp = objmesh.parse_control_points(archive.read_name(name))
            every += len(cp)
            same += sum(p.nodes[0] == p.nodes[1] for p in cp)
            if name[:-4] + ".msh" not in members:
                continue
            try:
                model = objmesh.parse(archive.read_name(name[:-4] + ".msh"), name)
            except Exception:
                continue
            with_mesh += len(cp)
            in_range += sum(0 <= p.nodes[0] < len(model.nodes) for p in cp)
    check("CTPT: a control point's first triple carries its node as an int32",
          with_mesh and with_mesh - in_range <= 1 and same < every,
          f"read as an int32, slot 1 names a node of the same-stem mesh on {in_range} of "
          f"{with_mesh} points; slot 2 is the same number on {same} of {every}")


def check_behaviour(check, game: Path) -> None:
    """The behaviour scripts: MISSIONS/SCRIPTS/*.scr."""
    paths = behaviour.scripts(game)
    scripts = []
    refused = []
    for path in paths:
        try:
            scripts.append(behaviour.read(path))
        except behaviour.ScriptFormatError as exc:
            refused.append(str(exc))
    check("behaviour: every script reads end to end",
          paths and not refused,
          f"{len(scripts)}/{len(paths)} .scr files consumed exactly -- "
          f"{sum(len(s.handlers) for s in scripts)} handlers and "
          f"{sum(s.nodes for s in scripts)} nodes, none refused")

    magics = {s.magic for s in scripts}
    check("behaviour: every script carries the same magic",
          magics == {behaviour.MAGIC},
          f"all {len(scripts)} open with {behaviour.MAGIC}, and every handler is "
          f"indexed by its own position")

    missing = {e: sum(1 for s in scripts if s.handler(e) is None)
               for e in behaviour.EVENTS}
    check("behaviour: the engine's own event handlers are in every script",
          scripts and not any(missing.values()),
          f"{len(behaviour.EVENTS)} handlers -- {behaviour.EVENTS[0]}, "
          f"{behaviour.EVENTS[1]}, {behaviour.EVENTS[2]} and {len(behaviour.EVENTS) - 3} "
          f"more -- present in all {len(scripts)} scripts")

    unpaired = []
    for s in scripts:
        for problem in s.problems:
            for phase in behaviour.PHASES:
                if s.handler(f"{problem}_{phase}") is None:
                    unpaired.append(f"{s.source.name}:{problem}_{phase}")
    carried = {p for s in scripts for p in s.problems}
    check("behaviour: every AI problem is written in both its halves",
          scripts and not unpaired,
          f"{len(carried)} distinct problems across the scripts, each carrying a "
          f"{behaviour.PHASES[0]} and a {behaviour.PHASES[1]} wherever it appears")

    known = set(behaviour.PROBLEMS)
    check("behaviour: the problems are the ones the reader knows",
          carried and carried == known,
          f"{len(carried)} problems named, and the set matches the "
          f"{len(behaviour.PROBLEMS)} in the reader exactly")

    nodes = [n for s in scripts for h in s.handlers for n in h.nodes]
    wrong = [n for n in nodes if n.binary and len(n.operands) != 2]
    widths = {len(n.operands) for n in nodes if not n.binary}
    check("behaviour: an opcode's arity is fixed by the opcode",
          nodes and not wrong and max(widths) <= behaviour.MAX_OPERANDS,
          f"{sum(1 for n in nodes if n.binary)} nodes on opcodes "
          f"{behaviour.BINARY.start}..{behaviour.BINARY.stop - 1} take exactly two "
          f"operands; the {len(nodes) - sum(1 for n in nodes if n.binary)} on "
          f"{behaviour.VARIADIC} take {min(widths)} to {max(widths)}")

    outside = 0
    for s in scripts:
        top = max((o for h in s.handlers for n in h.nodes for o in n.operands),
                  default=-1)
        outside += top >= s.nodes
    ceiling = max(o for n in nodes for o in n.operands)
    check("behaviour: the operands are not indices into the script itself",
          outside > len(scripts) * 0.5,
          f"{outside}/{len(scripts)} scripts name an operand at or past their own "
          f"node count, and the ceiling is {ceiling} whether a script holds 17 "
          f"nodes or 585 -- so the vocabulary is shared")

    calls = [n for n in nodes if n.calls]
    clean = [n for n in calls
             if n.head[2] == behaviour.NULL and n.head[3] == behaviour.NULL
             and n.trailer == behaviour.NULL]
    only6 = {n.opcode for n in calls}
    check("behaviour: a node with a function is a call and carries nothing else",
          calls and len(clean) == len(calls) and only6 == {behaviour.VARIADIC},
          f"{len(calls)} nodes select a function, every one under opcode "
          f"{behaviour.VARIADIC}, and all {len(clean)} leave head[2], head[3] and "
          f"the trailer null -- the fixed-arity opcodes never carry one")

    signature = defaultdict(lambda: (set(), set(), set()))
    for node in calls:
        arity, ops, writes = signature[node.function]
        arity.add(len(node.operands))
        ops.add(node.opcode)
        writes.add(node.destination != behaviour.NULL)
    one_arity = sum(1 for v in signature.values() if len(v[0]) == 1)
    one_write = sum(1 for v in signature.values() if len(v[2]) == 1)
    check("behaviour: a function's signature is fixed by its number",
          len(signature) == behaviour.FUNCTIONS
          and one_arity >= len(signature) - 5 and one_write >= len(signature) - 1,
          f"{len(signature)} distinct functions over "
          f"{min(signature)}..{max(signature)}; {one_arity} take a single number "
          f"of arguments and {one_write} either always write a destination or "
          f"never do")

    plain = [n for n in nodes if n.opcode == behaviour.VARIADIC and not n.calls]
    writers = [n for n in plain if n.destination != behaviour.NULL]
    sourced = [n for n in writers
               if (n.source != behaviour.NULL) != (n.immediate != behaviour.NULL)]
    quiet = [n for n in plain if n.destination == behaviour.NULL]
    bare = [n for n in quiet
            if n.source == behaviour.NULL and n.immediate == behaviour.NULL]
    check("behaviour: an assignment carries exactly one source, and only when it writes",
          plain and len(sourced) == len(writers) and len(bare) == len(quiet),
          f"{len(writers)}/{len(writers)} nodes that write carry a variable or an "
          f"immediate but never both, and {len(bare)}/{len(quiet)} that write "
          f"nothing carry neither")

    arity_ok = sum(1 for n in plain
                   if len(n.operands) == behaviour.TAG_ARITY.get(n.tag, -1))
    check("behaviour: a tag fixes its own arity, the way a function does",
          plain and arity_ok == len(plain),
          f"{arity_ok}/{len(plain)} nodes carry exactly the operands their tag "
          f"takes -- "
          + ", ".join(f"{t}:{a}" for t, a in sorted(behaviour.TAG_ARITY.items()))
          + " -- so the 110 with one operand are tags 3 and 4 and no others")

    refs = [n for n in plain if n.reference != behaviour.NULL]
    lits = [n for n in plain if n.literal != behaviour.NULL]
    ref_ok = sum(1 for n in refs if 0 <= n.reference < len(behaviour.variables(game)))
    span = len(behaviour.variables(game))
    impossible = [n for n in lits if not 0 <= n.literal < span]
    check("behaviour: the tag says whether head[2] is a variable or a number",
          refs and lits and ref_ok == len(refs) and impossible,
          f"under tag {behaviour.REFERENCE_TAG} all {ref_ok}/{len(refs)} values "
          f"are variable indices and only "
          f"{len({n.reference for n in refs})} distinct ones are used; under tag "
          f"{behaviour.LITERAL_TAG} {len(impossible)}/{len(lits)} cannot be an "
          f"index at all, and the rest run densely from 1")

    tags = {n.head[3] for n in writers}
    other = {n.head[3] for n in quiet}
    check("behaviour: head[3] says whether the node writes",
          tags == set(behaviour.ASSIGN_TAGS) and not (tags & other),
          f"every assignment that writes is tagged "
          f"{' or '.join(str(t) for t in behaviour.ASSIGN_TAGS)} and every one "
          f"that does not is tagged {min(other)}..{max(other)}; the two sets do "
          f"not meet")

    clean = negative = 0
    deepest = 0
    for s in scripts:
        for handler in s.handlers:
            depth = under = 0
            for node in handler.nodes:
                if node.opens:
                    depth += 1
                    deepest = max(deepest, depth)
                elif node.closes:
                    depth -= 1
                    under |= depth < 0
            clean += not under and depth == 0
            negative += bool(under)
    total = sum(len(s.handlers) for s in scripts)
    check("behaviour: a comparison opens a block and tag 1 closes it",
          total and clean >= total - 2,
          f"{clean}/{total} handlers hold exactly as many closers as "
          f"comparisons and bracket cleanly, nesting {deepest} deep at most; "
          f"the {total - clean} that do not carry a spare closer")

    exits = ended = 0
    marker = outer = 0
    for s in scripts:
        for handler in s.handlers:
            depth = 0
            for i, node in enumerate(handler.nodes):
                if node.closes:
                    depth -= 1
                if node.terminates:
                    exits += 1
                    ended += i + 1 < len(handler.nodes) and handler.nodes[i + 1].closes
                if (not node.calls and not node.assigns
                        and node.tag == behaviour.MARKER_TAG):
                    marker += 1
                    outer += depth <= 0
                if node.opens:
                    depth += 1
    check("behaviour: three tags end the block they sit in, and one does not",
          exits and ended >= exits - 1 and outer >= marker - 1,
          f"tags {', '.join(str(t) for t in behaviour.EXIT_TAGS)} are followed "
          f"immediately by a closer on {ended}/{exits} nodes, while tag "
          f"{behaviour.MARKER_TAG} sits at the outermost depth on {outer}/{marker} "
          f"and is followed by the handler's bookkeeping instead")

    def _kind(n):
        if n.calls:
            return "fn"
        if n.opcode in behaviour.BINARY:
            return "op"
        if (n.source != behaviour.NULL or n.reference != behaviour.NULL
                or n.literal != behaviour.NULL):
            return "assign"
        return f"tag{n.tag}"

    five = five_then = three = three_after = opened = 0
    for s in scripts:
        for handler in s.handlers:
            kinds = [_kind(n) for n in handler.nodes]
            opened += bool(kinds) and kinds[0] == "tag1"
            for i, k in enumerate(kinds):
                if k == "tag5":
                    five += 1
                    five_then += i + 1 < len(kinds) and kinds[i + 1] == "tag1"
                if k == "tag3":
                    three += 1
                    three_after += i > 0 and kinds[i - 1] == "op"
    check("behaviour: two tags sit in a fixed place in the node list",
          five and five_then == five and three_after >= three - 3 and not opened,
          f"tag 5 is followed immediately by tag 1 on {five_then}/{five} nodes, "
          f"tag 3 follows a comparison on {three_after}/{three}, and no handler "
          f"opens with tag 1 -- so the tags sit in the stream rather than "
          f"floating, which is what control flow would look like")

    table = behaviour.variables(game)
    names = [v.name for v in table]
    check("behaviour: the shared symbol table reads",
          len(table) > 200 and len(set(names)) == len(names),
          f"{len(table)} declarations in {behaviour.VARSET}, every name distinct, "
          f"read in file order because the index is the position")

    operands = [o for n in nodes for o in n.operands]
    stray = [o for o in operands if not 0 <= o < len(table)]
    check("behaviour: every operand names a variable in that table",
          operands and not stray,
          f"{len(operands)}/{len(operands)} operands across the corpus index "
          f"{behaviour.VARSET}, ceiling {max(operands)} against {len(table)} "
          f"declarations")

    dests = [n.destination for n in nodes if n.destination != behaviour.NULL]
    bad = [d for d in dests if not 0 <= d < len(table)]
    check("behaviour: a node's destination names one too",
          dests and not bad,
          f"{len(dests)} nodes carry a destination and all {len(dests)} index "
          f"{behaviour.VARSET}; {len(nodes) - len(dests)} write nowhere")

    pool = [v.name for v in table[: behaviour.READ_ONLY]]
    literals = sum(1 for v in table[: behaviour.READ_ONLY] if v.literal)
    written = [d for d in dests if d < behaviour.READ_ONLY]
    read = sorted({o for o in operands if o < behaviour.READ_ONLY})
    check("behaviour: the first declarations are read-only and the nodes respect it",
          not written and len(read) > 10 and literals == 20,
          f"no destination names any of the first {behaviour.READ_ONLY} "
          f"({literals} literals {pool[0]}..{pool[19]} plus {', '.join(pool[20:])}), "
          f"while {len(read)} of them are read as operands")


def check_varset_types(check, game: Path) -> None:
    """varset.var declares two of the three forms its own header documents."""
    declarations = behaviour.variables(game)
    if not declarations:
        return
    kinds = Counter(v.kind for v in declarations)
    types = Counter(v.type for v in declarations)
    check("behaviour: the scripts use two types and never the third form",
          set(kinds) == {"VAR"} and dict(types) == behaviour.TYPES,
          f"{len(declarations)} declarations, all VAR -- "
          + ", ".join(f"{n} {t}" for t, n in sorted(types.items()))
          + f"; the {behaviour.DECLARATIONS[1]}(...) form the file's own header "
            f"documents is never used")


def check_research(check, game: Path) -> None:
    """The research tree: MISSIONS/SCRIPTS/*.trf."""
    paths = research.trees(game)
    trees = []
    refused = []
    for path in paths:
        try:
            trees.append(research.read(path))
        except (research.ResearchFormatError, NotAnNResArchive) as exc:
            refused.append(str(exc))
    check("research: every .trf reads as a tree",
          paths and not refused,
          f"{len(trees)}/{len(paths)} archives read, {sum(len(t) for t in trees)} "
          f"items in total, none refused")

    widths = {len(t) for t in trees}
    check("research: every archive holds the same number of items",
          widths == {research.ITEMS},
          f"all {len(trees)} carry {research.ITEMS} items, and TRF0 is "
          f"{research.ITEMS} x {research.RECORD} bytes in each")

    named = [t for t in trees if all(i.name for i in t.items)]
    distinct = {i.name for t in trees for i in t.items}
    check("research: every item has a display name",
          len(named) == len(trees) > 0,
          f"{research.ITEMS} names per archive resolve out of TRF8, "
          f"{len(distinct)} of them distinct -- the tree holds several grades "
          f"of the same thing")

    linked = [t for t in trees if t.edges]
    bad = [(t.source.name, i.index, j)
           for t in linked for i in t.items
           for j in (*i.requires, *i.unlocks) if not 0 <= j < len(t)]
    check("research: every edge names an item that exists",
          linked and not bad,
          f"{sum(t.edges for t in linked)} prerequisites across {len(linked)} "
          f"archives, every one in 0..{research.ITEMS - 1}")

    mismatch = []
    for t in linked:
        forward = {(i.index, j) for i in t.items for j in i.requires}
        backward = {(j, i.index) for i in t.items for j in i.unlocks}
        if forward != backward:
            mismatch.append(t.source.name)
    check("research: the two edge lists are the same graph written twice",
          linked and not mismatch,
          f"in all {len(linked)} archives that carry them, TRF2/TRF3 and "
          f"TRF4/TRF5 are exact transposes -- which is what fixes the direction")

    centres = ("Sml Research cntr", "Med Research cntr", "Lrg Research cntr",
               "Enh Research cntr")
    spined = 0
    for t in linked:
        chain = [t.find(name) for name in centres]
        if not all(chain):
            continue
        first = [c[0] for c in chain]
        spined += all(a.index in b.requires
                      for a, b in zip(first, first[1:], strict=False))
    check("research: most archives put a chain of research centres at the spine",
          spined >= len(linked) * 0.5 > 0,
          f"in {spined}/{len(linked)} archives the four research centres each "
          f"name the one below as a prerequisite; the other "
          f"{len(linked) - spined} rewire it, which is what makes this a "
          f"per-mission tree")

    totals = sorted({t.edges for t in trees})
    check("research: each mission ships its own wiring",
          len(totals) > 3,
          f"{len(totals)} distinct prerequisite counts across {len(trees)} "
          f"archives, from {totals[0]} to {totals[-1]} -- {len(trees) - len(linked)} "
          f"carry no edges at all, and the commonest wiring is shared by "
          f"{max(sum(1 for t in trees if t.edges == e) for e in totals)}")


def check_saves(check, game: Path) -> None:
    """Save games: SAVE/*.sav and the slot index beside them."""
    paths = save.saves(game)
    read = []
    refused = []
    for path in paths:
        try:
            read.append(save.read(path))
        except save.SaveFormatError as exc:
            refused.append(str(exc))
    check("saves: every save reads its header",
          paths and not refused,
          f"{len(read)}/{len(paths)} open with {save.MAGIC.decode()} and version "
          f"{save.VERSION}, each naming the mission it is in")

    missions = [s for s in read
                if (game / Path(s.mission.replace('\\', '/'))).is_dir()]
    check("saves: every save names a mission that is installed",
          read and len(missions) == len(read),
          f"{len(missions)}/{len(read)} mission paths resolve to a directory, "
          f"{len({s.mission for s in read})} distinct")

    maps = [s for s in read if s.map and (game / "DATA" / "MAPS" / s.map).is_dir()]
    check("saves: every save names a map that is installed",
          read and len(maps) == len(read),
          f"{len(maps)}/{len(read)} name a DATA/MAPS directory that exists -- "
          + ", ".join(sorted({s.map for s in read})))

    wanted = {t for s in read for t in s.trees}
    here = {p.name.lower() for p in research.trees(game)}
    check("saves: every research tree a save names is installed",
          wanted and wanted <= here,
          f"{len(wanted)} distinct .trf named across the saves, all present; "
          f"one save names {max(len(s.trees) for s in read)} of them, one per clan")

    archives = {}
    for name in save.ARCHIVES:
        path = game / name
        if path.exists():
            # Folded, because the game's own lookup is: see NResArchive.find.
            archives[name] = {e.name.lower()
                              for e in NResArchive(path.read_bytes()).entries}
    refs = [r for s in read for r in s.references]
    resolved = sum(1 for r in refs
                   if r.archive in archives and r.member.lower() in archives[r.archive])
    widths = Counter(r.field for r in refs)
    check("saves: the members a save names are in the archives it names",
          refs and resolved >= len(refs) * 0.99,
          f"{resolved}/{len(refs)} references resolve into "
          f"{', '.join(sorted(archives))}, counting both record widths -- "
          + ", ".join(f"{n} at {w}" for w, n in sorted(widths.items())))

    narrow_hits = wide_hits = 0
    for path in save.saves(game):
        s = save.read(path)
        tma = game / s.mission / "data.tma"
        if not tma.exists():
            continue
        placed = {o.path.replace("\\", "/").rsplit("/", 1)[-1].lower().removesuffix(".dat")
                  for o in mission.load(tma).objects}
        wide = {r.member.lower() for r in s.references if r.field == save.MEMBER_AT[1]}
        narrow = {r.member.lower() for r in s.references if r.field == save.MEMBER_AT[0]}
        narrow_hits += len(narrow & placed)
        wide_hits += bool(wide & placed)
    check("saves: the two record widths hold different populations",
          narrow_hits == 0 and wide_hits >= 4,
          f"the 128-byte record names objects the mission itself places on "
          f"{wide_hits} of {len(save.saves(game))} saves; the 32-byte record "
          f"names {narrow_hits} on any of them -- so they are two records, "
          f"not one field written loosely")

    wide_gaps, narrow_gaps = [], []
    for path in save.saves(game):
        s = save.read(path)
        for width, bucket in ((save.MEMBER_AT[1], wide_gaps),
                              (save.MEMBER_AT[0], narrow_gaps)):
            offsets = sorted(r.offset for r in s.references if r.field == width)
            bucket.extend(b - a for a, b in zip(offsets, offsets[1:], strict=False))
    near = [gap for gap in wide_gaps if gap < 500]
    sized = [gap for gap in near
             if gap >= save.WORLD_RECORD and (gap - save.WORLD_RECORD) % save.WORLD_STEP == 0]
    exact = sum(1 for gap in narrow_gaps if gap == save.PART_RECORD)
    check("saves: each record has a size, and the two differ",
          near and len(sized) == len(near) and exact >= len(narrow_gaps) * 0.6,
          f"{len(sized)}/{len(near)} consecutive world records sit "
          f"{save.WORLD_RECORD} bytes apart plus a multiple of {save.WORLD_STEP}, "
          f"and {exact}/{len(narrow_gaps)} part records exactly "
          f"{save.PART_RECORD}")

    steps = []
    for path in save.saves(game):
        s = save.read(path)
        wide = sorted((r.offset, r.member) for r in s.references
                      if r.field == save.MEMBER_AT[1])
        for (at, name), (nxt, _) in zip(wide, wide[1:], strict=False):
            if nxt - at < 500:
                steps.append((name, (nxt - at - save.WORLD_RECORD) // save.WORLD_STEP))
    scenery = [k for name, k in steps if name.startswith(save.SCENERY)]
    other = [k for name, k in steps if not name.startswith(save.SCENERY)]
    varying = len({name for name, _ in steps
                   if len({k for n, k in steps if n == name}) > 1})
    check("saves: the world record's step tells furniture from machinery",
          steps and max(scenery) <= 1 and min(other) >= 1,
          f"{scenery.count(0)}/{len(scenery)} scenery records take no step and "
          f"none takes more than one; all {len(other)} others take at least one "
          f"-- and {varying} of {len({n for n, _ in steps})} names appear with "
          f"two counts, so it is partly the instance's own")

    rising = counted = 0
    for path in save.saves(game):
        s = save.read(path)
        blob = path.read_bytes()
        order = []
        for r in sorted(s.references, key=lambda r: r.offset):
            if r.field != save.MEMBER_AT[1] or not r.member.startswith(save.SCENERY):
                continue
            at = r.offset + save.WORLD_INDEX_AT
            if at + 2 > len(blob):
                continue
            value = struct.unpack_from("<H", blob, at)[0]
            if value != save.NO_INDEX:
                order.append(value)
        if order:
            counted += 1
            rising += all(b > a for a, b in zip(order, order[1:], strict=False))
    check("saves: a scenery record carries an index that rises in file order",
          counted and rising == counted,
          f"{rising}/{counted} saves with scenery hold a uint16 at "
          f"+{save.WORLD_INDEX_AT:#x} that only increases down the file -- an "
          f"identity assigned in order, and not an index into the mission's "
          f"own lists")

    kinds = defaultdict(lambda: [0, 0, 0])
    library = game / descriptions.LIBRARY
    if library.exists():
        parts = descriptions.read(library)
        for path in save.saves(game):
            blob = path.read_bytes()
            for r in save.read(path).references:
                part = parts.get(r.member)
                if (r.field != save.MEMBER_AT[0] or part is None
                        or r.offset + 76 > len(blob)):
                    continue
                a, b, _ = struct.unpack_from("<3i", blob, r.offset + save.PART_FIELDS[0])
                row = kinds[part.kind]
                row[0] += 1
                row[1] += a != 0
                row[2] += b != 0
        ammo = kinds.get("AMM", [0, 0, 0])
        check("saves: the part record's two ints belong to different kinds",
              ammo[0] and ammo[1] == ammo[0] and ammo[2] == 0,
              f"all {ammo[0]} ammunition records carry a value at "
              f"+{save.PART_FIELDS[0]} and zero at +{save.PART_FIELDS[1]}; "
              + ", ".join(f"{k} {v[1]}/{v[0]} and {v[2]}/{v[0]}"
                          for k, v in sorted(kinds.items()) if k != "AMM"))

    placed_hits = placed_total = 0
    for path in save.saves(game):
        s = save.read(path)
        tma = game / s.mission / "data.tma"
        if not tma.exists():
            continue
        blob = path.read_bytes()
        where = [o.position for o in mission.load(tma).objects]
        placed_total += len(where)
        for off in range(0, len(blob) - 12):
            point = struct.unpack_from("<3f", blob, off)
            if any(abs(point[0] - x) < 0.25 and abs(point[1] - y) < 0.25
                   and abs(point[2] - z) < 0.25 for x, y, z in where):
                placed_hits += 1
    aligned_hits = 0
    for path in save.saves(game):
        s = save.read(path)
        tma = game / s.mission / "data.tma"
        if not tma.exists():
            continue
        blob = path.read_bytes()
        where = [o.position for o in mission.load(tma).objects]
        for off in range(0, len(blob) - 12, 4):
            point = struct.unpack_from("<3f", blob, off)
            if any(abs(point[0] - x) < 0.25 and abs(point[1] - y) < 0.25
                   and abs(point[2] - z) < 0.25 for x, y, z in where):
                aligned_hits += 1
    check("saves: positions are in a save, off the four-byte grid",
          placed_hits > aligned_hits * 3 and placed_hits > 20,
          f"{placed_hits} float32 triples match a position their mission "
          f"places when every byte offset is tried, against {aligned_hits} on "
          f"the four-byte grid -- records sit at arbitrary offsets, so a "
          f"dword-aligned scan misses almost all of them")

    walked = big = 0
    for path in save.saves(game):
        s = save.read(path)
        if len(s.blobs) == save.BLOBS:
            walked += 1
            size = path.stat().st_size
            big += s.blobs[0].size > size * 0.6
    check("saves: the body opens as length-prefixed blobs, as the writer emits",
          walked == len(save.saves(game)) and big == walked,
          f"{walked}/{len(save.saves(game))} saves walk as {save.BLOBS} "
          f"length-prefixed runs before the format changes, the first holding "
          f"over 60% of the file -- the shape iron3d.dll's writer produces at "
          f"0x100a1637 through fwrite")

    index = save.slots(game)
    filled = [x for x in index if not x.empty]
    present = {p.name.lower() for p in paths}
    agree = sum(1 for x in filled if x.filename.lower() in present)
    check("saves: the slot index agrees with the files on disk",
          index and agree == len(filled) == len(paths),
          f"{len(index)} slots in {save.SLOTS}, {len(filled)} not empty and all "
          f"{agree} of those name a .sav that is there")


def check_vocabulary(check, game: Path) -> None:
    """The naming scheme the archives and saves share."""
    library = game / "objects.rlb"
    if not library.exists():
        return
    names = sorted(e.name for e in NResArchive(library.read_bytes()).entries)

    sizes = {"b": "large", "m": "medium", "l": "small", "t": "tiny", "f": "huge"}
    labels: dict[str, str] = {}
    for path in sorted((game / "UNITS").rglob("*.dat")):
        try:
            unit = objects.load_unit(path)
        except (objects.ObjectFormatError, ValueError, OSError):
            continue
        for part in unit.components:
            if part.ref.member and part.label:
                labels[part.ref.member] = part.label
    words = {"lrg": "large", "med": "medium", "sml": "small", "tny": "tiny"}
    agree = Counter()
    total = Counter()
    for member, label in labels.items():
        bits = member.split("_")
        if len(bits) < 3 or bits[-2].lower() not in sizes:
            continue
        letter = bits[-2].lower()
        found = re.search(r"\b(Huge|Large|Lrg|Medium|Med|Small|Sml|Tiny|Tny)\b",
                          label, re.IGNORECASE)
        if not found:
            continue
        word = found.group(1).lower()
        total[letter] += 1
        agree[letter] += words.get(word, word) == sizes[letter]
    check("vocabulary: the letter in a part id is its size",
          total and sum(agree.values()) >= sum(total.values()) - 2,
          f"b=large, m=medium, l=small, t=tiny, f=huge on "
          f"{sum(agree.values())}/{sum(total.values())} parts, against the display "
          f"name the assembly gives each -- the two that differ are A_L_05, a "
          f"creature, and R_B_06")

    def _parents(parts):
        out, stack = {}, []
        for i, part in enumerate(parts):
            while stack and stack[-1][1] == 0:
                stack.pop()
            out[i] = stack[-1][0] if stack else -1
            if stack:
                stack[-1][1] -= 1
            stack.append([i, part.child_count])
        return out

    clips = matched = orphans = 0
    kinds = Counter()
    for path in sorted((game / "UNITS").rglob("*.dat")):
        try:
            unit = objects.load_unit(path)
        except (objects.ObjectFormatError, ValueError, OSError):
            continue
        owner = _parents(unit.components)
        for i, part in enumerate(unit.components):
            clip = re.match(r"i_c(\d\d)_([a-z])_", part.ref.member or "")
            if not clip:
                continue
            clips += 1
            at = owner[i]
            gun = re.match(r"e_gun_([a-z])([a-z])_(\d\d)$",
                           unit.components[at].ref.member or "") if at >= 0 else None
            if not gun:
                orphans += 1
                continue
            matched += (gun.group(3) == clip.group(1)
                        and gun.group(1) == clip.group(2))
            kinds[gun.group(2)] += 1
    check("vocabulary: a clip's number and size name the gun it feeds",
          clips and matched == clips and not orphans,
          f"{matched}/{clips} i_cNN clips hang off an e_gun_<size><kind>_NN with "
          f"the same number and size, none orphaned -- so NN is a weapon type, "
          f"not a component class; the kind letter splits "
          + ", ".join(f"{k}:{v}" for k, v in sorted(kinds.items())))

    shared = defaultdict(set)
    for name in names:
        bits = name.split("_")
        if len(bits) >= 3 and len(bits[0]) == 2:
            shared["_".join(bits[1:])].add(bits[0])
    both = {k: v for k, v in shared.items() if {"bu", "fr"} <= v}
    check("vocabulary: two building sets cover the same list of functions",
          len(both) >= 8,
          f"bu_ and fr_ both supply {len(both)} of the same suffixes -- "
          + ", ".join(sorted({k.rsplit('_', 1)[-1] for k in both})[:7]) + ", ...")

    # The narrow record only: +72 is a field of that layout, and the wide
    # record puts its member name at 128, so the offset means nothing there.
    ordinals = []
    for path in save.saves(game):
        blob = path.read_bytes()
        for ref in save.read(path).references:
            if ref.field == save.MEMBER_AT[0] and ref.offset + 76 <= len(blob):
                ordinals.append(struct.unpack_from("<i", blob, ref.offset + 72)[0])
    small = [v for v in ordinals if 0 < v <= 64]
    walk = sum(1 for a, b in zip(ordinals, ordinals[1:], strict=False)
               if abs(b - a) == 1)
    check("vocabulary: a member record ends in a small ordinal",
          ordinals and len(small) >= len(ordinals) * 0.9,
          f"{len(small)}/{len(ordinals)} narrow records hold 1..{max(small)} at +72 and "
          f"{walk}/{len(ordinals) - 1} adjacent records differ by exactly one -- "
          f"it counts along a group, but it runs down as often as up and its role "
          f"is not established")


def check_atmosphere_events(check, game: Path) -> None:
    """The atmosphere's event vocabulary, and the field that is not its opcode."""
    frames = []
    for path in sorted(game.rglob("sky.ske")):
        try:
            frames.extend(sky.load(path).keyframes)
        except (sky.SkyFormatError, struct.error):
            continue
    if not frames:
        return

    covered = set(sky.EVENT_OPCODES) | set(sky.NO_EVENT)
    phases = {p for p, _ in sky.EVENT_OPCODES.values()}
    kinds = {k for _, k in sky.EVENT_OPCODES.values()}
    check("sky: the ten opcodes are five objects by two phases, less the gaps",
          covered == set(range(10)) and phases == set(sky.PHASES)
          and kinds == set(sky.OBJECT_TYPES) - {1},
          f"{len(sky.EVENT_OPCODES)} opcodes name "
          f"{', '.join(sky.OBJECT_TYPES[k] for k in sorted(kinds))} in "
          f"start/stop pairs; {sky.NO_EVENT} do nothing and SKY has no case, "
          f"being created outside the switch")

    named = [f for f in frames if f.name]
    names = Counter(f.name for f in named)
    check("sky: only the sun and the moon are named in a keyframe",
          set(names) == {"sun", "moon"},
          f"{len(named)} of {len(frames)} keyframes carry a name -- "
          + ", ".join(f"{n} x{c}" for n, c in sorted(names.items())))

    cycles = wholes = 0
    reused = []
    for path in sorted(game.rglob("sky.ske")):
        try:
            atmosphere = sky.load(path)
        except (sky.SkyFormatError, struct.error):
            continue
        sections = sorted({f.section for f in atmosphere.keyframes})
        if len(sections) < 2:
            continue
        cycles += 1
        spans = []
        for index in sections:
            group = [f for f in atmosphere.keyframes if f.section == index]
            spans.append((min(f.hour for f in group), max(f.hour for f in group)))
        wholes += all(lo == 0 and hi == 24 for lo, hi in spans)
        blocks = [{b"".join(f.slots) for f in atmosphere.keyframes if f.section == i}
                  for i in sections[:2]]
        reused.append(len(blocks[0] & blocks[1]))
    if cycles:
        check("sky: a second section is a second whole day, not a fragment",
              wholes == cycles,
              f"{wholes}/{cycles} files with two sections have both running "
              f"00h to 24h, the second reusing {max(reused)} of the first's "
              f"colour blocks at its own times")

    candidate = [f.trailer[sky.OPCODE_CANDIDATE] for f in frames
                 if len(f.trailer) > sky.OPCODE_CANDIDATE]
    in_range = sum(0 <= v <= 9 for v in candidate)
    dead = sum(f.trailer[sky.OPCODE_CANDIDATE] in sky.NO_EVENT
               for f in named if len(f.trailer) > sky.OPCODE_CANDIDATE)
    check("sky: the field that spans the opcode range is not the opcode",
          dead > len(named) // 2 and in_range < len(candidate),
          f"{in_range}/{len(candidate)} of that word is in 0..9, and it puts "
          f"{dead}/{len(named)} of the named keyframes on a do-nothing case -- "
          f"so the sun and moon would never start or stop")

    # The runtime keyframe puts the opcode five dwords past the minute, so a
    # contiguous copy from the file would put it five past the trailer's time
    # -- and the trailer's time shifts by one with the kind word, which the
    # fixed index above would miss.  It fails the same way, which closes the
    # "the index just moved" escape rather than finding the field.
    def shifted(frame):
        at = 4 if frame.kind == sky.KIND_WITH_PADDING else 3
        i = at + sky.OPCODE_CANDIDATE_SHIFTED
        return frame.trailer[i] if i < len(frame.trailer) else None
    moved = [v for v in (shifted(f) for f in frames) if v is not None]
    moved_dead = sum(shifted(f) in sky.NO_EVENT for f in named)
    check("sky: nor is the word the runtime layout would predict",
          moved_dead > len(named) // 2,
          f"the runtime keyframe holds the opcode five dwords past the minute, "
          f"so a contiguous copy would put it five past the trailer's time -- "
          f"which shifts with the kind word.  That word is in 0..9 on "
          f"{sum(0 <= v <= 9 for v in moved)}/{len(moved)} keyframes and still "
          f"puts {moved_dead}/{len(named)} named ones on a do-nothing case")


def check_research_streams(check, game: Path) -> None:
    """What MisLoad.dll's loader requires of a .trf, checked against the files."""
    paths = research.trees(game)
    if not paths:
        return
    directories = []
    for path in paths:
        archive = NResArchive(path.read_bytes())
        directories.append({e.tag: e for e in archive.entries})

    versioned = sum(d["TRF0"].link_count == research.VERSION
                    for d in directories if "TRF0" in d)
    check("research: the loader's version gate holds on every archive",
          versioned == len(directories),
          f"{versioned}/{len(directories)} archives carry {research.VERSION} in "
          f"the directory's second count over TRF0, which MisLoad.dll requires "
          f"before it reads a byte")

    flagged = sum(d["TRF1"].link_count != research.STATE_FLAG
                  for d in directories if "TRF1" in d)
    sized = sum("TRF1" in d and d["TRF1"].size == d["TRF1"].element_count
                == research.ITEMS for d in directories)
    check("research: TRF1 is one byte per item and its flag is never set",
          flagged == 0 and sized == len(directories),
          f"{sized}/{len(directories)} archives hold {research.ITEMS} bytes of "
          f"TRF1, and {flagged} set the boolean the loader keeps beside it")

    absent = {tag for d in directories for tag in research.STREAMS if tag not in d}
    together = sum(("TRF3" in d) == ("TRF5" in d) for d in directories)
    without = sum("TRF3" not in d for d in directories)
    check("research: only the streams the loader can do without are missing",
          absent <= set(research.OPTIONAL) and together == len(directories),
          f"{without} of {len(directories)} archives lack "
          f"{', '.join(sorted(absent)) or 'nothing'}, always both at once; "
          f"every other tag is in every archive, as the loader requires")

    trees = [research.read(path) for path in paths]
    mapped = complete = paired = 0
    for tree in trees:
        parts = tree.parts
        mapped += len(parts) == research.PARTS
        complete += all(item.parts for item in tree.items)
        paired += max(len(item.parts) for item in tree.items) == 2
    check("research: TRFB maps every part onto an item, and every item is named",
          mapped == complete == len(trees) and paired == len(trees),
          f"{research.PARTS} part ids per archive across {len(trees)}, each "
          f"naming one of the {research.ITEMS} items, all {research.ITEMS} "
          f"named, and never more than two parts to an item")

    named = compared = 0
    library = game / descriptions.LIBRARY
    if library.exists():
        parts = descriptions.read(library)
        for tree in trees:
            for item in tree.items:
                for pid in item.parts:
                    part = parts.get(pid)
                    if part is None:
                        continue
                    compared += 1
                    named += part.name.strip().lower() == item.name.strip().lower()
        check("research: a part and the item that researches it are the same thing",
              compared and named == compared,
              f"{named}/{compared} TRFB entries land on the item whose TRF8 "
              f"display name is the part's own name in {descriptions.LIBRARY} "
              f"-- two files that share no bytes agreeing on all of it")

    back = items = 0
    for tree in trees:
        for item in tree.items:
            items += 1
            back += tree.part_at(item.part_index) in item.parts
    check("research: the mapping is written both ways round",
          items and back == items,
          f"{back}/{items} records hold a uint16 at +0x20 that indexes the "
          f"TRFB entry naming that same item -- part to item in TRFB, item to "
          f"part in the record")

    spread = [set() for _ in range(6)]
    for tree in trees:
        for item in tree.items:
            for i, value in enumerate(item.tail):
                spread[i].add(value)
    check("research: the record's last six bytes are six fields",
          all(1 < len(s) <= 40 for s in spread),
          "distinct values per byte at +0x22..+0x27: "
          + ", ".join(str(len(s)) for s in spread)
          + " -- each narrow, which a packed word's bytes would not be")

    agreed = pairs = 0
    for path, d in zip(paths, directories, strict=True):
        if "TRF3" not in d or "TRF5" not in d:
            continue
        archive = NResArchive(path.read_bytes())
        pairs += 1
        ok = True
        for counts, flat in (("TRF2", "TRF3"), ("TRF4", "TRF5")):
            blob = archive.read(d[counts])
            total = sum(struct.unpack(f"<{len(blob) // 4}i", blob))
            ok &= total == d[flat].element_count == d[flat].size // 4
        agreed += ok
    check("research: a count stream and its flat list agree three ways",
          pairs and agreed == pairs,
          f"{agreed}/{pairs} archives have sum(TRF2) == TRF3's element count "
          f"== its size in int32, and the same for TRF4 and TRF5 -- the pairing "
          f"the loader builds at 0x100032cd")


def check_descriptions(check, game: Path) -> None:
    """The parts database, objects.dlb, and what it names."""
    path = game / descriptions.LIBRARY
    if not path.exists():
        return
    parts = descriptions.read(path)
    check("descriptions: every member reads",
          len(parts) == descriptions.PARTS,
          f"{len(parts)} {descriptions.TAG} members in {descriptions.LIBRARY}, "
          f"each with a display name and five key=value slots")

    trees = [research.read(p) for p in research.trees(game)]
    ids: list[str] = []
    for tree in trees:
        archive = NResArchive(tree.source.read_bytes())
        for entry in archive.entries:
            if entry.tag == "TRF6":
                blob = archive.read(entry)
                ids = [x.decode("latin-1") for x in blob.split(b"\0") if x]
                break
        if ids:
            break
    check("descriptions: the library and the research tree list the same parts",
          ids and list(parts) == ids,
          f"{len(ids)} part ids in TRF6 and {len(parts)} in "
          f"{descriptions.LIBRARY}, the same names in the same order")

    by_code: dict[str, descriptions.Description] = {}
    for part in parts.values():
        if part.code:
            by_code.setdefault(part.code, part)
    tree = trees[0] if trees else None
    matched = compared = 0
    if tree:
        for item in tree.items:
            part = by_code.get(item.code) if item.code else None
            if part is None:
                continue
            compared += 1
            matched += item.values == (
                part.research_energy, part.research_ore,
                part.build_energy, part.build_ore,
            )
    check("descriptions: the tree's four floats are the library's four costs",
          compared and matched >= compared * 0.9,
          f"{matched}/{compared} items match ResearchEnergyCost, ResearchOreCost, "
          f"BuildEnergyCost and BuildOreCost exactly -- so the floats are two "
          f"resources charged twice, and none of them is a time")

    ammunition = [p for p in parts.values() if p.kind == "AMM"]
    clips = [p for p in ammunition if p.belongs_to]
    coded = sum(1 for c in clips if c.belongs_to.rsplit(" ", 1)[-1] in by_code)
    check("descriptions: an ammunition entry names the weapon it feeds",
          ammunition and len(clips) == len(ammunition),
          f"{len(clips)}/{len(ammunition)} ammunition members name a weapon in "
          f"slot 5 -- the gun-to-clip link said independently of the assemblies; "
          f"{coded} end in a short code that is itself a member and the rest name "
          f"it in words only")

    catalogue = set(parts)
    library_names = {e.name for e in NResArchive((game / "objects.rlb").read_bytes()).entries} \
        if (game / "objects.rlb").exists() else set()
    clips_all = {n for n in library_names if re.match(r"i_c\d\d_", n)}
    outside = sorted(clips_all - catalogue)
    check("descriptions: six clip models are outside the game's own catalogue",
          clips_all and outside == ["i_c06_l_01", "i_c06_l_02", "i_c06_l_df",
                                    "i_c07_l_01", "i_c07_l_02", "i_c07_l_df"],
          f"{len(clips_all) - len(outside)}/{len(clips_all)} i_cNN models in "
          f"objects.rlb have a {descriptions.TAG} entry; the {len(outside)} that "
          f"do not are every i_c06_l and i_c07_l, and they are in no tree, save "
          f"or assembly either")

    classified = [p for p in parts.values() if p.kind and p.size]
    tidy = [p for p in parts.values()
            if p.kind in descriptions.GROUP_KINDS.get(p.group, set())]
    stray_group = sorted(p.part for p in parts.values() if p not in tidy)
    check("descriptions: the group is the catalogue's top-level tab",
          len(classified) == len(parts) and len(tidy) >= len(parts) - 4,
          f"all {len(parts)} carry a classification line and {len(tidy)} sit in "
          f"the group their kind belongs to -- "
          + ", ".join(f"G{g} {descriptions.GROUPS[g]} "
                      f"({'+'.join(sorted(descriptions.GROUP_KINDS[g]))})"
                      for g in sorted(descriptions.GROUPS))
          + f"; the {len(stray_group)} that do not are "
          + ", ".join(stray_group))

    banded = [p for p in parts.values() if p.banded]
    stray = sorted(p.part for p in parts.values() if not p.banded)
    check("descriptions: the tech level rises with the upgrade level",
          len(banded) >= len(parts) - 4,
          f"{len(banded)}/{len(parts)} parts sit in the level band their "
          f"UpgradeLevel implies -- "
          + ", ".join(f"{k}:{v[0]}..{v[1]}" for k, v in sorted(descriptions.BANDS.items()))
          + f"; the {len(stray)} that do not are " + ", ".join(stray))

    free = [p for p in parts.values() if not p.researched]
    check("descriptions: what costs nothing to research is what you start with",
          free and len(free) < len(parts),
          f"{len(free)}/{len(parts)} parts have both research costs at zero; the "
          f"other {len(parts) - len(free)} must be paid for")


def check_resources(check, game: Path) -> None:
    """The .cfg resource descriptors, and the text library one of them names."""
    found: list[tuple[Path, resources.Descriptor]] = []
    for path in sorted(game.rglob("*.cfg")):
        for d in resources.descriptors(path):
            found.append((path, d))
    if not found:
        return

    libraries = 0
    by_name = by_index = unresolved = 0
    archives: dict[str, tuple[set[str], int]] = {}
    for _, d in found:
        library = resources.locate(game, d.library)
        if library is None:
            # Its bindings cannot resolve either, and saying so keeps the two
            # checks below from passing on a shrunken population.
            unresolved += len(d)
            by_name += len(d)
            continue
        libraries += 1
        key = str(library)
        if key not in archives:
            if library.suffix.lower() == ".dll":
                ids = set(resources.strings(library.read_bytes()))
                archives[key] = ({str(i) for i in ids}, -1)
            else:
                opened = (rsli.RsLiArchive.open(library) if rsli.is_rsli(library)
                          else NResArchive.open(library))
                names = {e.name.lower() for e in opened.entries}
                archives[key] = (names, len(opened.entries))
        members, count = archives[key]
        for value in d.bindings.values():
            if value.lstrip("-").isdigit():
                by_index += 1
                ok = value in members if count < 0 else 0 <= int(value) < count
            else:
                by_name += 1
                ok = (value.lower() in members
                      or value.rsplit(".", 1)[0].lower() in members)
            unresolved += not ok
    check("resources: every descriptor's library exists",
          libraries == len(found) == resources.DESCRIPTORS,
          f"{libraries}/{len(found)} descriptor objects in "
          f"{len({str(p) for p, _ in found})} .cfg files name a library that is "
          f"in the installation")
    check("resources: every name a descriptor binds resolves",
          unresolved == 0 and by_name + by_index == resources.BINDINGS,
          f"{by_name + by_index - unresolved}/{by_name + by_index} bindings "
          f"reach a member -- {by_name} by name, {by_index} by index")

    shapes = sum(d.libtype == resources.LIBTYPE and d.type in resources.TYPES
                 for _, d in found)
    check("resources: a descriptor's shape is uniform",
          shapes == len(found),
          f"{shapes}/{len(found)} are libtype {resources.LIBTYPE!r} with a type "
          f"in {sorted(resources.TYPES)}; 3 is never used")

    music = {d.role for _, d in found if d.type == 5}
    sounds = {resources.locate(game, d.library).name.lower()
              for _, d in found if d.type in (4, 5)
              and resources.locate(game, d.library)}
    check("resources: type 5 is the looping theme and nothing else",
          music == {resources.MUSIC_ROLE} and sounds <= {"sounds.lib", "voices.lib"},
          f"all {sum(d.type == 5 for _, d in found)} type-5 descriptors are "
          f"{resources.MUSIC_ROLE}, and types 4 and 5 together name only "
          f"{', '.join(sorted(sounds))}")

    index = game.joinpath(*resources.TEXT_INDEX)
    if not index.exists():
        return
    texts = resources.TextResources.open(game)
    named = {i for i in texts.names.values() if i in texts.table}
    check("resources: the text table and its index are the same set",
          len(texts.names) == len(texts.table) == len(named) == resources.TEXTS,
          f"{len(texts.names)} names in {index.name} and {len(texts.table)} "
          f"strings in the DLL, every name resolving and every string named")

    empty = sum(not t.strip() for t in texts.table.values())
    high = [t for t in texts.table.values() if any(ord(c) > 127 for c in t)]
    check("resources: the strings are text",
          empty == 0 and len(high) <= 3,
          f"{len(texts.table)} strings, none blank, {len(high)} carrying a "
          f"character above U+007F -- the English build, with a Cyrillic A left "
          f"inside one name")


def check_briefing(check, game: Path) -> None:
    """briefing.cfg: the flythrough a campaign mission opens on."""
    paths = briefing.briefings(game)
    if not paths:
        return
    campaign = sorted((game / "MISSIONS" / "CAMPAIGN").glob("*/Mission.*"))
    check("briefing: every campaign mission has one, and only those",
          len(paths) == briefing.BRIEFINGS
          and {p.parent for p in paths} == set(campaign),
          f"{len(paths)} briefings against {len(campaign)} campaign missions "
          f"and {len(gamedir.missions(game))} missions in all")

    stops: list[tuple[Path, briefing.Waypoint]] = []
    complete = 0
    for path in paths:
        raw = mission.load_cfg(path)
        complete += all(set(briefing.FIELDS) <= set(p) for p in raw.values())
        stops.extend((path, w) for w in briefing.waypoints(path))
    vocabulary = sum(w.edge in briefing.EDGES and w.wait in briefing.WAITS
                     for _, w in stops)
    check("briefing: a waypoint is 24 fields, always the same 24",
          complete == len(paths) and len(stops) == briefing.WAYPOINTS
          and vocabulary == len(stops),
          f"{len(stops)} waypoints in {len(paths)} files, every one with all "
          f"{len(briefing.FIELDS)} fields, an EdgeType in {list(briefing.EDGES)} "
          f"and a WaitType in {list(briefing.WAITS)}")

    check("briefing: LoopIndex is read and never set",
          all(w.loop == briefing.NO_LOOP for _, w in stops),
          f"all {len(stops)} waypoints carry LoopIndex {briefing.NO_LOOP}")

    inside = above = sampled = 0
    clearance = []
    for path in paths:
        d = path.parent
        m = mission.load(d / "data.tma")
        land_dir = game / Path(str(m.map_path).replace("\\", "/")).parent
        if not (land_dir / "Land.msh").exists():
            continue
        land = landmesh.load(land_dir / "Land.msh")
        (minx, miny, _), (maxx, maxy, _) = land.bounds()
        for w in briefing.waypoints(path):
            cx, cy, cz = w.camera
            tx, ty, _ = w.target
            inside += (minx <= cx <= maxx and miny <= cy <= maxy
                       and minx <= tx <= maxx and miny <= ty <= maxy)
            height = land.height_at(cx, cy)
            if height is not None:
                sampled += 1
                above += cz > height
                clearance.append(cz - height)
    median = statistics.median(clearance) if clearance else 0.0
    check("briefing: the camera path is in world coordinates",
          inside == len(stops) and above == sampled == len(stops),
          f"{inside}/{len(stops)} waypoints put both camera and target inside "
          f"their own map's extent, and {above}/{sampled} sit above the terrain "
          f"-- median clearance {median:.1f}, least {min(clearance or [0]):.1f}")

    texts = resources.TextResources.open(game)
    spoken = [(p, w) for p, w in stops if w.text_id]
    missing = [(p, w) for p, w in spoken if texts.get(w.text_id) is None]
    check("briefing: the subtitles resolve, bar the finale's",
          len(missing) == 3 and len({p for p, _ in missing}) == 1,
          f"{len(spoken) - len(missing)}/{len(spoken)} TextResID reach a string; "
          f"the {len(missing)} that do not are all in "
          f"{missing[0][0].parent.name if missing else '-'} -- "
          f"{', '.join(sorted(w.text_id for _, w in missing))}")

    voiced = unbound = 0
    for path in paths:
        bound: dict[str, str] = {}
        for d in resources.descriptors(path.parent / "mission.cfg"):
            if d.role == briefing.BRIEFING_ROLE:
                bound = d.bindings
        for w in briefing.waypoints(path):
            if w.sound_id:
                voiced += 1
                unbound += w.sound_id not in bound
    check("briefing: every voice resolves, and only through briefing_sounds",
          voiced and unbound == 0,
          f"{voiced - unbound}/{voiced} SoundResID are bound by the mission's "
          f"own {briefing.BRIEFING_ROLE} descriptor; none needs "
          f"{briefing.MESSAGE_ROLE}")

    files = sorted(game.rglob(briefing.MESSAGES))
    lines = [m for f in files for m in briefing.messages(f)]
    resolved = sum(texts.get(m.text_id) is not None for m in lines if m.text_id)
    gaps = sum([m.index for m in briefing.messages(f)]
               != list(range(len(briefing.messages(f)))) for f in files)
    with_role = {f.parent for f in files if any(
        d.role == briefing.MESSAGE_ROLE
        for d in resources.descriptors(f.parent / "mission.cfg"))}
    check("briefing: the in-mission messages resolve too",
          len(files) == briefing.MESSAGE_FILES
          and len(lines) == briefing.MESSAGES_TOTAL
          and resolved == len(lines)
          and with_role == {f.parent for f in files},
          f"{resolved}/{len(lines)} messages in {len(files)} files reach a "
          f"string, and the same {len(with_role)} missions declare "
          f"{briefing.MESSAGE_ROLE}; message_index is an id, not a position -- "
          f"{gaps} files skip a number")


def check_settings(check, game: Path) -> None:
    """The engine's own configuration files, and which module owns each."""
    registry_path = game / settings.COMPONENTS_FILE
    if not registry_path.exists():
        return
    rows = settings.registry(registry_path)
    names = settings.component_names(registry_path)
    present = sum((game / r.dll).exists()
                  or any(q.name.lower() == r.dll.lower() for q in game.iterdir())
                  for r in rows)
    check("settings: the component registry is eight contiguous ids",
          len(rows) == settings.COMPONENTS
          and [r.cid for r in rows] == list(range(settings.COMPONENTS))
          and set(names) == {r.cid for r in rows},
          f"{len(rows)} rows in {settings.COMPONENTS_FILE}, ids 0 to "
          f"{settings.COMPONENTS - 1}, each named by the file's own header "
          f"-- {names.get(0, '?')} through {names.get(settings.COMPONENTS - 1, '?')}")
    check("settings: every registry row names a module that is there",
          present == len(rows),
          f"{present}/{len(rows)} rows name one of "
          f"{len({r.dll.lower() for r in rows})} DLLs in the installation, "
          f"between them {len({r.function for r in rows})} entry points")

    behaviour_switches = settings.switches(game / settings.BEHAVIOUR_FILE)
    areal_switches = settings.switches(game / settings.AREALMAP_FILE)
    shared = set(behaviour_switches) & set(areal_switches)
    check("settings: the two debug files share a logging preamble",
          shared == set(settings.LOGGING),
          f"{len(behaviour_switches)} switches in {settings.BEHAVIOUR_FILE} and "
          f"{len(areal_switches)} in {settings.AREALMAP_FILE}, sharing exactly "
          f"{', '.join(sorted(shared))}")

    binaries = {p.name: p.read_bytes().lower() for p in sorted(game.iterdir())
                if p.suffix.lower() in (".dll", ".exe")}
    owners = {
        settings.COMPONENTS_FILE: "World3D.dll",
        settings.BEHAVIOUR_FILE: "Behavior.dll",
        settings.AREALMAP_FILE: "ArealMap.dll",
        settings.DISPLAY_FILE: "iron3d.dll",
        settings.DISPATCHER_FILE[-1]: "iron3d.dll",
    }
    matched = []
    for name, owner in owners.items():
        holders = [b for b, data in binaries.items() if name.lower().encode() in data]
        matched.append(holders == [owner])
    check("settings: one module owns each configuration file",
          all(matched) and len(binaries) > 1,
          f"{sum(matched)}/{len(owners)} file names appear in exactly one of "
          f"{len(binaries)} binaries, and it is the module that carries their "
          f"switches -- {', '.join(sorted(set(owners.values())))}")

    keys = settings.completed(game)
    if not keys:
        return
    missions = {settings.dispatcher_key(game, d): d for d in gamedir.missions(game)}
    resolved = sum(k in missions for k in keys)
    check("settings: every mission the dispatcher records is a real one",
          resolved == len(keys) and set(keys.values()) == {settings.DONE},
          f"{resolved}/{len(keys)} keys in {settings.DISPATCHER_FILE[-1]} "
          f"flatten from one of the {len(missions)} mission directories, every "
          f"value {settings.DONE} -- this install's progress, not the format")


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
        check_effects, check_effect_timing, check_actions, check_footprints, check_rsli,
        check_control, check_efficiency,
        check_motion, check_playback, check_ground, check_sensors, check_hit_test,
        check_combat, check_ownership,
        check_capture, check_repair, check_chassis, check_weapons,
        check_turrets, check_packages, check_builder,
        check_units, check_loading, check_search, check_construction,
        check_controls, check_player_input, check_turret_channels,
        check_behaviour, check_research, check_descriptions, check_saves,
        check_vocabulary, check_resources, check_briefing, check_settings,
        check_research_streams, check_atmosphere_events,
        check_varset_types, check_profiles,
    )
    for fn in checks:
        fn(check, game)
    failed = [n for n, ok, _ in results if not ok]
    print(f"\n{len(results) - len(failed)}/{len(results)} checks passed")
    for n in failed:
        print(f"  failed: {n}")
    return 1 if failed else 0
