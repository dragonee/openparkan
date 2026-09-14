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
    assembly,
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
    unread_by_map: Counter[str] = Counter()
    for folder in maps:
        flags = landmesh.load(folder / "Land.msh").draw_flags
        seen.update(flags)
        unread_by_map[folder.name] += sum(
            1 for v in flags if v & landmesh.DRAW_FLAGS_UNREAD)
    always = 0xFF
    for v in seen:
        always &= v
    check("Land.msh: the draw order's flags byte takes four values",
          len(seen) == 4 and always == 0x48,
          f"{dict(sorted(seen.items()))} over {sum(seen.values())} entries; "
          f"0x08 and 0x40 are set on every one and say nothing, 0x10 opens a "
          f"batch, and 0x80 is on {sum(n for v, n in seen.items() if v & 0x80)} "
          f"entries of two maps")
    # Terrain.dll's draw-order rebuilder (0x10060480 and two siblings) writes
    # bits 0-6 as 0x48 plus the batch start and leaves bit 7 alone; the draw
    # (0x1004399a) reads bit 4 and nothing reads bit 7.
    rebuilt = sum(n for v, n in seen.items()
                  if v & ~(landmesh.DRAW_BATCH_START | landmesh.DRAW_FLAGS_UNREAD)
                  == landmesh.DRAW_FLAGS_BUILT)
    carriers = {k: v for k, v in unread_by_map.items() if v}
    check("Land.msh: bit 7 aside, the draw-order byte is what the engine's rebuilder writes",
          rebuilt == sum(seen.values()) and len(carriers) == 2,
          f"{rebuilt}/{sum(seen.values())} entries are 0x{landmesh.DRAW_FLAGS_BUILT:02x} "
          f"plus the batch-start bit once bit 7 is set aside; bit 7 itself, which "
          f"no instruction in Terrain.dll reads or writes, sits on "
          f"{sum(carriers.values())} entries: {dict(sorted(carriers.items()))}")

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
          f"largest being {widest} -- three 2-bit edge codes, the winged-edge "
          f"link, which is also why its groups are neither spatial nor tied to "
          f"a material.  The surface word beside it uses two bits: "
          f"{dict(sorted(surface_values.items()))}")

    check("Land.msh: face field 13 is not a spatial patch id", tight < loose * 0.1,
          f"{tight}/{tight + loose} groups are tighter than a random subset of "
          f"the same size, so it cannot be used for culling")

    check("Land.msh: stream 14 is the weight of layer 1", dirty == 0,
          f"exactly 1.0 on all {clean} vertices no layer-2 face touches; "
          f"below it on {varying} of the {touched} that one does")

    # The landscape's draw asks the material manager for track 1 on every face
    # that is not water (Terrain.dll:0x1002b4b6) and binds its texture as the
    # second stage at render phase 9; a material with one track answers with
    # track 0.  What that reaches, over the fine level of every map:
    lib = materials.MaterialLibrary(game / "Material.lib")
    ground_twin = ground_all = water_twin = water_all = base_flag = faces_seen = 0
    layer2_twin = layer2_all = 0
    no_twin: set[str] = set()
    for folder in maps:
        m = landmesh.load(folder / "Land.msh")
        for f in m.lod_faces(0):
            faces_seen += 1
            # the draw's lighting path also wants face flag 0x400
            base_flag += bool(m.face_flags[f] & 0x400)
            name = m.texture_name(1, m.face_tex1[f])
            record = lib.get(name) if name else None
            twin = bool(record and record.track_count >= materials.TWIN_TRACKS)
            if m.is_water(f):
                water_all += 1
                water_twin += twin
                continue
            ground_all += 1
            ground_twin += twin
            if not twin and name:
                no_twin.add(name)
            if m.face_tex2[f] != landmesh.NO_TEXTURE:
                name2 = m.texture_name(2, m.face_tex2[f])
                record2 = lib.get(name2) if name2 else None
                layer2_all += 1
                layer2_twin += bool(record2 and record2.track_count >= materials.TWIN_TRACKS)
    check("Land.msh: the ground the draw asks for a second track has one",
          ground_twin > ground_all * 0.9 and water_all and water_twin == 0
          and base_flag == faces_seen,
          f"{ground_twin}/{ground_all} level-0 faces that are not water name a "
          f"layer-1 material with a second track (the rest: {sorted(no_twin)}), "
          f"and {layer2_twin}/{layer2_all} of their layer-2 materials do; none of "
          f"the {water_all} water faces', which the draw never asks.  Face flag "
          f"0x400, which the same path wants, is on {base_flag}/{faces_seen}")


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
          f"at a 14-byte header and a stride of 34 all four alpha bytes are "
          f"100 or below, over {slots} slots -- and the 16-byte "
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


#: The objects the buoy example draws: Mission 01's objective marker.
BUOY_RECORD = "s_tree_29"
BUOY_BEAM, BUOY_LAMP, BUOY_PLACE = "HLP_RAY_R", "HLP_LAMP_R", "HLP_PLACE_R"
#: The engine's scale for an object mesh's ``uint16`` UVs
#: (``Terrain.dll:0x10035070``) -- not the 256 ``mesh.py`` reads them over.
OBJECT_UV_FULL = 1024
MESH_ARCHIVES = (
    "static.rlb", "intsys.rlb", "turrets.rlb", "guns.rlb", "parts.rlb",
    "weapon.rlb", "animals.rlb", "bases.rlb", "fortif.rlb", "system.rlb",
)


def check_material_draw(check, game: Path) -> None:
    """How a material draws: the track fields, the glow, the cell, the buoy."""
    lib = materials.MaterialLibrary(game / "Material.lib")
    textures = NResArchive.open(game / "Textures.lib")

    # The track word: mode in the low three bits, the lerp mask above.
    modes, masks = Counter(), Counter()
    multi = ascending = starts_at_zero = 0
    unsorted = []
    for name, m in lib.materials.items():
        for t in m.tracks:
            modes[t.kind] += 1
            masks[t.param] += 1
            if len(t.keys) > 1:
                multi += 1
                times = [k.time for k in t.keys]
                if all(a < b for a, b in zip(times, times[1:], strict=False)):
                    ascending += 1
                else:
                    unsorted.append(name)
                starts_at_zero += times[0] == 0
    check("Material.lib: a track's mode is 0 to 3, its mask five bits",
          set(modes) <= {0, 1, 2, 3} and all(p < 0x20 for p in masks),
          f"modes {dict(sorted(modes.items()))} (loop, ping-pong, once, random) "
          f"and masks {dict(sorted(masks.items()))} over {sum(modes.values())} "
          f"tracks -- no mode past the four-way table at World3D.dll:0x10003668 "
          f"and no mask bit past 0x10, the ambient alpha")

    # The bracket search reads a key's time as the END of its interval: key
    # 0's entry holds from 0.  A start time would put some first key at 0.
    check("Material.lib: a key's time ends its interval",
          multi and starts_at_zero == 0 and ascending >= multi - 1,
          f"0 of {multi} multi-key tracks put their first key at time 0, and "
          f"{ascending} run strictly ascending -- what a search for "
          f"key[i-1].time <= t < key[i].time needs (the exception: "
          f"{', '.join(unsorted) or 'none'})")

    # The device never reads the entry's emissive; the ambient is the glow.
    glow = Counter()
    for m in lib.materials.values():
        if m.entries and m.entries[0].colour == (0, 0, 0) \
                and m.entries[0].ambient != (0, 0, 0):
            glow[m.blend] += 1
    entries = [e for m in lib.materials.values() for e in m.entries]
    emissive = sum(e.emissive != (0, 0, 0) for e in entries)
    check("Material.lib: the glow is a black diffuse under an ambient colour",
          glow[materials.BLEND_ADD] > 200 and emissive < len(entries) // 20,
          f"{dict(sorted(glow.items()))} materials by flags draw a black "
          f"diffuse with an ambient colour -- self-light, since the device's "
          f"emissive is scene + ambient -- while only {emissive} of "
          f"{len(entries)} entries carry the emissive nothing reads")

    # The buoy.
    beam, lamp, place = (lib.materials[n] for n in (BUOY_BEAM, BUOY_LAMP, BUOY_PLACE))
    track = [(k.entry, k.time) for k in beam.tracks[0].keys]
    same = all([(t.kind, t.param, [(k.entry, k.time) for k in t.keys])
                for t in m.tracks] == [(0, 1, track)] for m in (beam, lamp, place))
    pages = texm.parse_pages(textures.read_name(beam.entries[0].texture))
    cells = [e.cell for e in beam.entries]
    strips = [pages[c] for c in cells]
    check("Material.lib: the buoy beam steps SUN4.0's strips under a red glow",
          beam.blend == materials.BLEND_ALPHA and cells == [0, 1, 2] and same
          and track == [(0, 50), (1, 100), (2, 150), (1, 200)]
          and all(e.colour == (0, 0, 0) and e.ambient_alpha == 1.0 for e in beam.entries)
          and all(w == 64 and h == 128 for _, _, w, h in strips),
          f"{BUOY_BEAM} is flags {beam.blend} on {beam.entries[0].texture}, "
          f"cells {cells} = (x, y, w, h) {strips}, ambient "
          f"{['#{:02x}{:02x}{:02x}'.format(*e.ambient) for e in beam.entries]}; its track, "
          f"shared with {BUOY_LAMP} and {BUOY_PLACE}, is mode 0 mask 1 keys "
          f"{track} -- cells 0 1 2 1 every 50 ms, the colour gliding")

    # The beam's UVs fill exactly one cell at the engine's 1/1024.
    parts = assembly.Assembly(game)
    ref = parts.record_mesh(parts.library.get(BUOY_RECORD))
    mesh = parts.mesh(ref)
    wear = parts.wear(ref)
    beam_index = wear.materials.index(BUOY_BEAM)
    reach = [0, 0]
    for b in mesh.batches:
        if b.material != beam_index:
            continue
        first, count = b.triangles
        for t in range(first, first + count):
            for vi in mesh.triangles[t]:
                raw = [round(c * objmesh.UV_FIXED_POINT_SCALE) for c in mesh.uv[vi]]
                reach = [max(a, c) for a, c in zip(reach, raw, strict=True)]

    # And across every object batch that asks for a cell.
    inside = cell_batches = 0
    for name in MESH_ARCHIVES:
        ar = NResArchive.open(game / name)
        for e in ar:
            if e.tag != "MESH":
                continue
            try:
                palette = objmesh.parse_wear(
                    ar.read_name(e.name.rsplit(".", 1)[0] + ".wea")).materials
                m = objmesh.parse(ar.read(e), e.name, palette)
            except (KeyError, ValueError, struct.error):
                continue
            for b in m.batches:
                mat = lib.materials.get(palette[b.material].upper()) \
                    if b.material < len(palette) else None
                if mat is None or mat.whole_texture:
                    continue
                first, count = b.triangles
                hi = max((round(c * objmesh.UV_FIXED_POINT_SCALE)
                          for t in range(first, first + count)
                          for vi in m.triangles[t] for c in m.uv[vi]), default=0)
                cell_batches += 1
                inside += hi <= OBJECT_UV_FULL
    check("objects: a stream-5 UV is over 1024, so a cell stays in its page",
          reach[0] == OBJECT_UV_FULL and OBJECT_UV_FULL * 0.99 <= reach[1] <= OBJECT_UV_FULL
          and inside > cell_batches * 3 // 4,
          f"{BUOY_BEAM}'s batches on {BUOY_RECORD} reach raw {reach[0]} x "
          f"{reach[1]}: one cell at 1/1024, four cells by two at 1/256; "
          f"{inside} of {cell_batches} batches whose material names a cell "
          f"reach no further than 1024")


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


#: ``Terrain.dll``'s render settings by index, as ``0x1005eb10`` names them.
REFLECTION_SETTINGS = {25: ("UseEmbossBump", 2), 26: ("UseReflections", 2),
                       30: ("UseEMBMReflections", 2), 31: ("EMBMCoeff00", 0),
                       32: ("EMBMCoeff11", 0), 33: ("EMBMMaxVal", 1), 34: ("EMBMBumpTile", 0),
                       35: ("EMBMBumpMove", 1)}


def check_water_reflection(check, game: Path) -> None:
    """Water reflects: the settings, the reflection camera, the water box, phase 10."""
    paths = [game / name for name in ("Terrain.dll", "iron3d.dll", "Ngi32.dll")]
    if not all(p.exists() for p in paths):
        return
    terrain, iron, ngi = (p.read_bytes() for p in paths)
    t_at, i_at, n_at = _image_at(terrain), _image_at(iron), _image_at(ngi)

    def cstr(at, va: int) -> str:
        return at(va, 40).split(b"\0")[0].decode("latin-1")

    def f32(word: int) -> float:
        return struct.unpack("<f", struct.pack("<I", word))[0]

    # The names: from the 16th on, each record is `push type; push name; lea ecx;
    # call 0x1005f390; mov esi, eax; mov ecx, 9; mov edi, table + 0x24 * index`
    # (the first 15 are built inline); every one ends copying into its slot.
    table = 0x100A6798
    body = t_at(0x1005EB10, 2161)
    slots = {(v - table) // 0x24 for v in (struct.unpack("<I", m.group(1))[0]
                                           for m in re.finditer(rb"\xbf(....)", body, re.S))
             if table <= v < table + 36 * 0x24 and (v - table) % 0x24 == 0}
    named = {}
    record_code = rb"\x6a(.)\x68(....)\x8d\x8d....\xe8....\x8b\xf0\xb9\x09\x00\x00\x00\xbf(....)"
    for m in re.finditer(record_code, body, re.S):
        slot = (struct.unpack("<I", m.group(3))[0] - table) // 0x24
        named[slot] = (cstr(t_at, struct.unpack("<I", m.group(2))[0]), m.group(1)[0])
    # The defaults: `mov dword [reg + disp], imm32` on the settings object.
    defaults = {}
    body = t_at(0x1005FA80, 746)
    for m in re.finditer(rb"\xc7[\x80-\x87](....)(....)|\xc7[\x40-\x47](.)(....)", body, re.S):
        disp = struct.unpack("<I", m.group(1))[0] if m.group(1) else m.group(3)[0]
        defaults[(disp - 4) // 4] = struct.unpack("<I", m.group(2) or m.group(4))[0]
    got = {i: named.get(i) for i in REFLECTION_SETTINGS}
    values = {i: defaults.get(i) for i in REFLECTION_SETTINGS}
    want = {25: 1, 26: 1, 30: 0, 33: 64, 35: 10000}
    floats = {31: 0.01, 32: 0.01, 34: 100.0}
    check("Terrain.dll: the water's settings and their defaults",
          slots == set(range(36)) and got == REFLECTION_SETTINGS
          and all(values[i] == v for i, v in want.items())
          and all(values[i] is not None and abs(f32(values[i]) - v) < 1e-6
                  for i, v in floats.items()),
          f"{len(slots)} table slots filled at 0x1005eb10, {len(named)} through 0x1005f390; "
          + ", ".join(f"{i} {got[i][0] if got[i] else '?'} = "
                      f"{f32(values[i]) if i in floats and values[i] is not None else values[i]}"
                      for i in sorted(REFLECTION_SETTINGS)))

    # iron3d.dll hands the ini's keys to group 0x1e, Terrain.dll's page.
    sends = {
        "REFLECTIONS": (0x10061450, 0x10061493, b"\x0f\x95\x44\x24\x28", 0x1006171e,
                        b"\x8a\x4c\x24\x1c", 0x10061736, b"\x66\xc7\x44\x24\x12\x1a\x00"),
        "EMBOSS_BUMP": (0x100613BA, 0x100613FD, b"\x0f\x95\x44\x24\x2a", 0x10061747,
                        b"\x8a\x4c\x24\x1e", 0x1006177B, b"\x66\xc7\x44\x24\x16\x19\x00"),
        "EMBM": (0x100613E8, 0x10061432, b"\x0f\x95\x44\x24\x2b", 0x1006179A,
                 b"\x8a\x5c\x24\x1f", 0x10061795, b"\x66\x89\x5c\x24\x12"),
    }
    sent = {}
    for key, (push, flag_at, flag, use_at, use, id_at, id_code) in sends.items():
        pushed = (i_at(push, 1) == b"\x68"
                  and cstr(i_at, struct.unpack("<I", i_at(push + 1, 4))[0]) == key)
        sent[key] = pushed and i_at(flag_at, 5) == flag and i_at(use_at, 4) == use \
            and i_at(id_at, len(id_code)) == id_code
    group = i_at(0x1006172C, 5) == b"\xbb\x1e\x00\x00\x00"
    ini = settings.sections(game / "Iron_3D.ini").get("CS", {})
    check("iron3d.dll: REFLECTIONS, EMBOSS_BUMP and EMBM reach settings 26, 25 and 30",
          all(sent.values()) and group and ini.get("REFLECTIONS") == "1" and ini.get("EMBM") == "1",
          f"each key's value becomes a flag (setne) that is sent with group 0x1e and setting "
          f"0x1a, 0x19 and 0x1e at 0x10061736, 0x1006177b, 0x10061795: {sent}; the ini ships "
          f"REFLECTIONS={ini.get('REFLECTIONS')} EMBM={ini.get('EMBM')}, so the game runs "
          f"REFLECTION_SHIFTED")

    # The camera: its two names, the mirror and the clip plane.
    def word(va: int) -> int:
        return struct.unpack("<I", t_at(va, 4))[0]

    def pushes(va: int) -> str | None:
        return cstr(t_at, word(va + 1)) if t_at(va, 1) == b"\x68" else None

    def call_to(va: int) -> int:
        return (va + 5 + struct.unpack("<i", t_at(va + 1, 4))[0]) & 0xFFFFFFFF

    names = (pushes(0x1001FE75), pushes(0x1001FEC9))
    camera = call_to(0x1001FE7D) == call_to(0x1001FED1) == 0x100839C0
    guards = (t_at(0x1001FDE4, 5) == b"\xba\x1a\x00\x00\x00"
              and t_at(0x1001FE41, 5) == b"\xb8\x1e\x00\x00\x00")
    mirror = (t_at(0x10083E7D, 7) == b"\x81\x7d\xfc\x00\x00\x80\xbf"
              and t_at(0x1008435C, 10) == b"\xc7\x85\xe0\xfd\xff\xff\x00\x00\x80\xbf"
              and t_at(0x10084366, 6) == b"\xd9\x05\x04\xa2\x09\x10"
              and f32(word(0x1009A204)) == 2.0
              and t_at(0x100843F7, 6) == b"\xd9\x05\xfc\xa1\x09\x10"
              and f32(word(0x1009A1FC)) == 0.5)
    clip = (t_at(0x10084426, 7) == b"\x6a\x01\x68\x98\x00\x00\x00"
            and t_at(0x10084461, 7) == b"\x6a\x00\x68\x98\x00\x00\x00")
    check("Terrain.dll: the reflection camera, its mirror and its clip plane",
          names == ("REFLECTION_SHIFTED", "REFLECTION") and camera and guards and mirror and clip,
          f"CLandscape::Initialize, under setting 26, pushes {names[0]!r} under setting 30 and "
          f"{names[1]!r} otherwise into CCamera::CCamera 0x100839c0 ({camera}); the draw at "
          f"0x10083e20 skips a water level of -1, mirrors with -1 and 2 x h, and brackets the "
          f"draw with render state 152 (CLIPPLANEENABLE) 1 and 0 about plane z + 0.5 - h "
          f"({mirror and clip})")

    # The textures: the reflection capped at 256, the bump map 32 square; phase 10.
    sizes = (t_at(0x100423A0, 7) == b"\x81\x7d\xec\x00\x01\x00\x00"
             and t_at(0x100425CD, 6) == b"\x66\xc7\x45\xe2\x20\x00")
    surface = t_at(0x1002CDC3, 10) == b"\xc7\x82\xc8\x00\x00\x00\x0a\x00\x00\x00"
    bump = (t_at(0x100492C3, 5) == b"\xb9\x21\x00\x00\x00"
            and struct.unpack("<d", t_at(0x1009B2E8, 8))[0] == 4.0
            and abs(struct.unpack("<d", t_at(0x1009B2E0, 8))[0] - 3.14159) < 1e-6
            and call_to(0x100492F6) == 0x1008E160 and call_to(0x10049328) == 0x1008E820)
    record = struct.unpack_from("<11I", n_at(0x10036A30 + 44 * 16, 44))
    states = n_at(record[2], 12 * record[3])
    triples = {(s, k): v for s, k, v in (struct.unpack_from("<3I", states, 12 * i)
                                         for i in range(record[3]))}
    bumpenv, modulate, selectarg2, texture, diffuse, current = 22, 4, 3, 2, 0, 1
    phase = (record[0] == 10 and triples.get((0, 1)) == bumpenv and triples.get((0, 2)) == texture
             and triples.get((0, 3)) == current and triples.get((1, 1)) == modulate
             and triples.get((1, 2)) == texture and triples.get((1, 3)) == diffuse
             and triples.get((1, 4)) == selectarg2 and triples.get((1, 6)) == diffuse)
    check("Terrain.dll: the water surface is phase 10, BUMPENVMAP then reflection x diffuse",
          sizes and surface and bump and phase,
          f"CShade caps the reflection texture at 256 and makes a 32-square bump map "
          f"({sizes}) of EMBMMaxVal (setting 33) x cos and sin of 4 x 3.14159 (i + j)/32 "
          f"({bump}); "
          f"0x1002ca80 sets phase 10 ({surface}); Ngi32.dll's record 16 is phase {record[0]}: "
          f"stage 0 op {triples.get((0, 1))} (BUMPENVMAP), stage 1 op {triples.get((1, 1))} "
          f"(MODULATE) of texture and diffuse, alpha op {triples.get((1, 4))} of the diffuse")

    # The water box the texture covers, from Land.msh.
    boxes = {}
    for d in gamedir.maps(game):
        m = landmesh.load(d / "Land.msh")
        faces = [i for i in range(m.face_count) if m.is_water(i)]
        if not faces:
            continue
        vs = {v for i in faces for v in m.faces[i]}
        xs = [m.positions[v][0] for v in vs]
        ys = [m.positions[v][1] for v in vs]
        boxes[d.name] = (min(xs), min(ys), max(xs), max(ys), m.water_level())
    tut = boxes.get("Tut_1")
    texels = sorted(max(b[2] - b[0], b[3] - b[1]) / 256 for b in boxes.values())
    check("Land.msh: Tut_1's water box is 1095 x 1276, 4.3 x 5.0 units a reflection texel",
          tut is not None and len(boxes) == 11
          and [round(v, 1) for v in tut[:4]] == [385.5, 255.8, 1480.5, 1531.7]
          and round(tut[4], 4) == -1.7255,
          f"the bounding box of the water faces' vertices on {len(boxes)} maps; Tut_1 "
          f"{tut and [round(v, 1) for v in tut[:4]]} at h {tut and round(tut[4], 4)}; the "
          f"texture's larger texel runs from {texels[0]:.2f} to {texels[-1]:.2f} units across "
          f"the maps")


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

    # The words MisLoad.dll's object reader names (0x10003900), against the
    # data.  Two version words, then the record's own.
    versions = Counter((m.unknown_pre_objects, m.lode_version) for m in parsed)
    everything = [o for m in parsed for o in m.objects]
    headers = Counter(o.unknown[2][3] for o in everything)
    check("data.tma: the words before the objects and lodes are versions",
          versions == Counter({(mission.OBJECT_VERSION, 1): len(parsed)})
          and headers == Counter({1: len(everything)}),
          f"object record version {mission.OBJECT_VERSION} and lode table word 1 "
          f"on {len(parsed)}/{len(dirs)} missions; the property table's own "
          f"leading word, which the reader discards, is 1 on {headers[1]}/"
          f"{len(everything)} objects")

    owned_objects = [o for o in everything if o.clan_id is not None]
    indexed = sum(o.clan_index == o.clan_id for o in owned_objects)
    level = sum(o.angles[:2] == (0.0, 0.0) for o in everything)
    check("data.tma: the word after a path is the owning clan",
          owned_objects and indexed == len(owned_objects) and level == len(everything),
          f"{indexed}/{len(owned_objects)} owned objects carry their ClanID in "
          f"it, which iron3d.dll indexes its clan records with; and the two "
          f"'padding' words are the turns about x and y, 0 on "
          f"{level}/{len(everything)}")

    places = _building_places(game)
    inside = hosted = same_clan = heroes_in_bunkers = 0
    for m in parsed:
        by_id = {o.logical_id: o for o in m.objects}
        for o in m.objects:
            if o.host is None and o.vertex is None:
                continue
            inside += 1
            building = by_id.get(o.host)
            if (building is None or building.kind != mission.KIND_BUILDING
                    or o.kind != mission.KIND_UNIT or o.vertex is None):
                continue
            unit = objects.load_unit(game / building.path.replace("\\", "/"))
            place = places.get(unit.components[0].ref.member.lower())
            hosted += place is not None and 0 <= o.vertex < len(place[1])
            same_clan += building.clan_id == o.clan_id
            heroes_in_bunkers += (place is not None and place[0] == "BUNKER"
                                  and "\\HERO\\" in o.path.upper())
    check("data.tma: a unit can start inside a building, at a hall-way vertex",
          inside and hosted == inside and same_clan == heroes_in_bunkers == inside - 1,
          f"{hosted}/{inside} objects with the words set are units naming a "
          f"building's logical id and a vertex of its hall-way graph -- "
          f"{same_clan} their own clan's, {heroes_in_bunkers} of them heroes in "
          f"a bunker; the other "
          f"{len(everything) - inside} set both to -1")

    flagged = Counter(o.kind for o in everything if o.start_flag)
    pairs = halves = 0
    for m in parsed:
        bridges = [o for o in m.objects
                   if o.kind == mission.KIND_BUILDING and "BRIDGE" in o.path.upper()]
        for a in bridges:
            for b in bridges:
                turn = abs((b.rotation - a.rotation) % (2 * math.pi) - math.pi)
                if a.logical_id < b.logical_id and a.path == b.path and turn < 1e-3:
                    pairs += 1
                    later = b.start_flag and not a.start_flag
                    halves += bool(later) and abs(b.rotation - a.rotation - math.pi) < 1e-3
    check("data.tma: a building's start flag marks one half of each bridge",
          pairs and halves == pairs,
          f"on {halves}/{pairs} bridge pairs exactly one half sets it, the one "
          f"with the later logical id and the angle pi further on; "
          f"{flagged[mission.KIND_BUILDING]} buildings set it in all, and "
          f"{flagged[mission.KIND_UNIT]} units and "
          f"{flagged[mission.KIND_VEGETATION] + flagged[mission.KIND_ROCK]} "
          f"pieces of scenery whose flag is never passed on")

    uniform = sum(o.scale[0] == o.scale[1] == o.scale[2] for o in everything)
    scaled = Counter(o.kind for o in everything if o.scale != (1.0, 1.0, 1.0))
    check("data.tma: scale is uniform, and it is scenery that is scaled",
          uniform == len(everything)
          and scaled[mission.KIND_BUILDING] == 0 and scaled[mission.KIND_UNIT] == 2,
          f"x = y = z on {uniform}/{len(everything)}; "
          f"{scaled[mission.KIND_VEGETATION]} trees and "
          f"{scaled[mission.KIND_ROCK]} rocks are not 1, no building, and two "
          f"animals whose scale no creator is handed")

    lodes = [lode for m in parsed for lode in m.lodes]
    typed = sum(lode.object_type == mission.MINERAL_LODE for lode in lodes)
    per_clan = sum(len(m.lodes) == len(m.clans) for m in parsed)
    check("data.tma: the trailer's records are mineral lodes",
          lodes and typed >= len(lodes) - 3 and per_clan < len(parsed) // 2,
          f"{len(lodes)} records on {sum(1 for m in parsed if m.lodes)} missions, "
          f"{typed} typed {mission.MINERAL_LODE:#x} -- the minerals search's "
          f"type -- and {sum(lode.found for lode in lodes)} already found; the "
          f"count equals the clan count on only {per_clan} of {len(parsed)}, so "
          f"they are not a viewpoint per clan")


#: A placement whose lowest level-0 vertex stands this far above the ground floats.
FLOATING = 0.25


def check_scale(check, game: Path) -> None:
    """A placement's scale: read from version 10, uniform, applied to scenery only."""
    versions: Counter[int] = Counter()
    by_kind: Counter[tuple[int, bool]] = Counter()
    uniform = 0
    scaled_units = []
    first: Counter[tuple[str, float]] = Counter()
    parsed = []
    for d in gamedir.missions(game):
        m = mission.load(d / "data.tma")
        parsed.append(m)
        versions[m.unknown_pre_objects] += 1
        for o in m.objects:
            scaled = o.scale != (1.0, 1.0, 1.0)
            by_kind[(o.kind, scaled)] += 1
            uniform += o.scale[0] == o.scale[1] == o.scale[2]
            if scaled and o.kind not in mission.SCALED_KINDS:
                scaled_units.append(f"{d.name} {o.path.split(chr(92))[-1]} {o.scale[0]:g}")
            if scaled and d.as_posix().endswith("CAMPAIGN.00/Mission.01"):
                first[(o.path.lower(), o.scale[0])] += 1
    total = sum(by_kind.values())
    scaled = sum(n for (_k, s), n in by_kind.items() if s)
    check("data.tma: every object record is version 10, the one with a scale",
          set(versions) == {mission.SCALE_VERSION},
          f"the word before the object count is {dict(versions)} over "
          f"{sum(versions.values())} missions; MisLoad.dll:0x100039bf reads the three "
          f"floats only at {mission.SCALE_VERSION} or more")
    check("data.tma: 218 placements carry a uniform scale other than 1",
          scaled == 218 and uniform == total,
          f"{scaled} of {total} placements, uniform on {uniform}; by (kind, scaled): "
          f"{dict(sorted(by_kind.items()))}")
    check("data.tma: only scenery and two animals are scaled",
          by_kind[(mission.KIND_BUILDING, True)] == 0 and len(scaled_units) == 2
          and all("tushka" in u for u in scaled_units),
          f"no building; units {scaled_units}, built from their .dat with the matrix "
          f"alone (iron3d.dll:0x10033cdb), so theirs is not applied")
    check("Mission 01: seventeen scaled trees and stones",
          sum(first.values()) == 17 and all(p.startswith(("s_tree", "s_stone")) for p, _ in first),
          f"(member, scale): count {dict(sorted(first.items()))}")

    # At their scale, scaled scenery stands on the ground; at scale 1 about half floats.
    library = objects.ObjectLibrary(game / "objects.rlb")
    archives: dict[str, NResArchive] = {}
    lows: dict[str, float | None] = {}
    lands: dict[str, landmesh.LandMesh] = {}

    def lowest(name: str) -> float | None:
        if name not in lows:
            record = library.get(name)
            ref = record.mesh if record else None
            lows[name] = None
            if ref is not None:
                archive = archives.setdefault(
                    ref.library.lower(), NResArchive.open(game / ref.library.lower()))
                m = objmesh.parse(archive.read_name(ref.member), ref.member)
                posed = m.posed_positions()
                zs = [posed[v][2] for tri in m.select(0) for v in tri]
                lows[name] = min(zs) if zs else None
        return lows[name]

    tally: Counter[str] = Counter()
    for m in parsed:
        msh = game / "DATA" / "MAPS" / m.map_name / "Land.msh"
        if not msh.is_file():
            continue
        land = lands.setdefault(m.map_name, landmesh.load(msh))
        for o in m.objects:
            if o.kind not in mission.SCALED_KINDS:
                continue
            low = lowest(o.path)
            ground = land.height_at(o.position[0], o.position[1])
            if low is None or ground is None:
                continue
            tag = "tree" if o.kind == mission.KIND_VEGETATION else "stone"
            if o.placed_scale != 1.0:
                tally[f"{tag} scaled"] += 1
                tally[f"{tag} floats at its scale"] += (
                    o.position[2] + o.placed_scale * low - ground > FLOATING)
                tally[f"{tag} floats at scale 1"] += o.position[2] + low - ground > FLOATING
            else:
                tally[f"{tag} unscaled"] += 1
                tally[f"{tag} unscaled floats"] += o.position[2] + low - ground > FLOATING
    check("mission scenery: at its scale, a scaled tree or stone stands on the ground",
          all(tally[f"{t} floats at its scale"] * 10 < tally[f"{t} floats at scale 1"]
              and tally[f"{t} floats at its scale"] * 10 < tally[f"{t} scaled"]
              for t in ("tree", "stone")),
          f"{dict(tally)} (floating: lowest level-0 vertex more than {FLOATING} above "
          f"the ground); control: the placements at scale 1")


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

    # A point's two node slots: the control system places it by the first
    # (Control.dll:0x1000b22a); the ground contact lets a contact point live
    # and die with the second (0x1001a3aa, 0x1001ac0d).  On a chassis, where
    # the two differ, the second is a wheel or leg below the first.
    bases = NResArchive.open(game / "bases.rlb")
    differ = below = 0
    carried: Counter[str] = Counter()
    done: set[str] = set()
    for record in lib.records.values():
        mref, cref = record.mesh, record.slot_with_suffix("cpt")
        if not mref or not cref or cref.library.lower() != "bases.rlb" \
                or mref.library.lower() != "bases.rlb" or cref.member.lower() in done:
            continue
        done.add(cref.member.lower())
        try:
            points = objmesh.parse_control_points(bases.read_name(cref.member), cref.member)
            chassis = objmesh.parse(bases.read_name(mref.member), mref.member)
        except KeyError:
            continue
        for p in points:
            if p.placed_on == p.carrier:
                continue
            differ += 1
            k = p.carrier
            while 0 <= k < len(chassis.nodes):
                if k == p.placed_on:
                    below += 1
                    carried[re.sub(r"_?\d+$", "", p.name.split("_d")[0])] += 1
                    break
                parent = chassis.nodes[k].parent
                k = -1 if parent == objmesh.NO_PARENT else parent
    check("CTPT: where a chassis point's two nodes differ, the second carries it",
          differ and below > differ * 0.85,
          f"on {below}/{differ} chassis points whose two node slots differ, the "
          f"second is a node below the first -- {dict(carried.most_common(6))}; "
          f"the rest name -1 or a node past the mesh")

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
          f"-- the terrain's field 13 again; {class_seen - small_class} faces "
          f"carry leftovers above the six bits")

    # The trailing word's low six bits are the winged-edge link, checked by
    # geometry: the code names the neighbour's edge with the same two vertex
    # positions.  And flag 2 is a building's floor.
    named = coded = open_edges = open_three = 0
    floor_meshes: set[str] = set()
    graph_meshes: set[str] = set()
    floor_faces = floor_up = 0
    for name in ARCHIVES:
        ar = NResArchive.open(game / name)
        for e in ar:
            if e.tag != "MESH":
                continue
            m = objmesh.parse(ar.read(e), e.name)
            if not m.face_class:
                continue
            if objmesh.read_path_graph(NResArchive(ar.read(e), e.name)) is not None:
                graph_meshes.add(f"{name}/{e.name}")
            P = m.positions
            node_of = {}
            if any(f & objmesh.FACE_BUILDING_FLOOR for f in m.face_flags):
                floor_meshes.add(f"{name}/{e.name}")
                for k, node in enumerate(m.nodes):
                    for s in node.slot_index:
                        if s != objmesh.NO_SLOT and s < len(m.slots):
                            sl = m.slots[s]
                            stop = sl.first_triangle + sl.triangle_count
                            for t in range(sl.first_triangle, stop):
                                node_of[t] = k
            for i, tri in enumerate(m.triangles):
                if m.face_flags[i] & objmesh.FACE_BUILDING_FLOOR:
                    floor_faces += 1
                    k = node_of.get(i)
                    pose = m.world_pose(k) if k is not None else objmesh.IDENTITY_POSE
                    floor_up += objmesh.quaternion_rotate(pose[1], m.face_normal[i])[2] > 0.9
                for edge in range(3):
                    j = m.face_adjacency[i][edge]
                    back = m.edge_twin(i, edge)
                    if j == objmesh.NO_FACE:
                        open_edges += 1
                        open_three += back is None
                        continue
                    if j >= len(m.triangles):
                        continue
                    named += 1
                    mine = {P[tri[edge]], P[tri[(edge + 1) % 3]]}
                    other = m.triangles[j]
                    coded += back is not None and {P[other[back]], P[other[(back + 1) % 3]]} == mine
    check("MESH: a face record's last word is the winged-edge link",
          coded == named > 0 and open_three == open_edges,
          f"its low six bits name, for each edge, the neighbour's edge with the "
          f"same two vertex positions on {coded}/{named} in-range neighbours, "
          f"and read 3 on all {open_three}/{open_edges} open edges -- the "
          f"terrain's field 13, checked here by geometry")
    check("MESH: face flag 2 is the floor of a building with a path graph",
          floor_meshes == graph_meshes and floor_up > floor_faces * 0.85,
          f"it is on {len(floor_meshes)} meshes, exactly the {len(graph_meshes)} "
          f"that carry a path graph, and {floor_up}/{floor_faces} of its faces "
          f"point straight up once posed")

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

    # Mounting an external part drops its root node and hangs the root's
    # children on the socket (AniMesh.dll:0x1000a7cc), so the root's own pose
    # never reaches the picture -- nor would geometry, and none has any.
    part_meshes: dict[str, objmesh.ObjectMesh | None] = {}
    mounted = empty_root = 0
    for f in dats:
        try:
            unit = objects.load_unit(f)
        except objects.ObjectFormatError:
            continue
        for c in unit.components[1:]:
            part = lib.get(c.ref.member)
            if part is None or part.tag != "EXTO" or not part.mesh:
                continue
            key = f"{part.mesh.library}/{part.mesh.member}".lower()
            if key not in part_meshes:
                try:
                    blob = NResArchive.open(game / part.mesh.library).read_name(part.mesh.member)
                    part_meshes[key] = objmesh.parse(blob, part.mesh.member)
                except KeyError:
                    part_meshes[key] = None
            m = part_meshes[key]
            if m is None or not m.nodes:
                continue
            mounted += 1
            empty_root += all(s == objmesh.NO_SLOT for s in m.nodes[0].slot_index)
    check("UNITS/*.dat: a mounted part's root node carries no geometry",
          mounted and empty_root == mounted,
          f"{empty_root}/{mounted} external parts mounted in the shipped "
          f"assemblies have a root node with no slot at all -- the node the "
          f"engine drops when it hangs the part on its socket, which is why a "
          f"socket's rotation wins wherever it disagrees with the root's")

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
    ordered = sections_seen = 0
    failures = []
    for path in files:
        try:
            atmosphere = sky.load(path)
        except sky.SkyFormatError as exc:
            failures.append(str(exc))
            continue
        parsed += 1
        frames += len(atmosphere)
        dated += all(0 <= k.hour <= 24 and 0 <= k.minute < 60 for k in atmosphere.keyframes)
        # Terrain.dll:0x10067500 bubble-sorts each section after loading; the
        # files never need it.
        for index in range(atmosphere.section_count):
            stored = [k.minutes for k in atmosphere.keyframes if k.section == index]
            sections_seen += 1
            ordered += stored == sorted(stored)
    check("sky.ske: parses to the byte", parsed == len(files),
          f"{parsed}/{len(files)} files, {frames} keyframes, read as Terrain.dll:0x100672d0 "
          f"reads them: per section a version, a count and two times, per keyframe a "
          f"version, a time and the opcode ahead of the slots, and a closing time"
          + ("" if not failures else f" -- {failures[0]}"))
    check("sky.ske: keyframes carry a time of day", dated == parsed,
          f"{dated}/{parsed} files hold an hour 0-24 and a minute 0-59 in every keyframe "
          f"of every section")
    check("sky.ske: keyframes run in time order", ordered == sections_seen,
          f"{ordered}/{sections_seen} sections are stored sorted by time, which the "
          f"engine's sort after loading (0x10067500) leaves alone")

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

    # A starting event takes what it needs from its keyframe's effect list:
    # GetEvents stops with "Rain background sound not specified" or "Lightning
    # effect not specified" when the first entry is empty.  Snow needs nothing.
    starts: Counter[str] = Counter()
    supplied: Counter[str] = Counter()
    with_weather = 0
    for path in files:
        try:
            atmosphere = sky.load(path)
        except sky.SkyFormatError:
            continue
        kinds = atmosphere.weather()
        with_weather += bool(kinds)
        for kind, frames_ in kinds.items():
            for frame in frames_:
                starts[kind] += 1
                first = frame.effects[0] if frame.effects else ""
                supplied[kind] += first == {"rain": sky.RAIN_MARKER,
                                            "lightning": sky.LIGHTNING_MARKER}.get(kind, first)
    check("sky.ske: a starting event carries the effect the engine asks for",
          starts["rain"] and starts["lightning"] and starts["snow"]
          and supplied == starts,
          f"{with_weather}/{parsed} missions start weather: {starts['rain']} rain starts all "
          f"name {sky.RAIN_MARKER} first, {starts['lightning']} lightning starts all name "
          f"{sky.LIGHTNING_MARKER} (Terrain.dll:0x1006e3bb, 0x1006e5af); "
          f"{starts['snow']} snow starts need and name nothing")

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

    # The third float is the sun object's light (0x1006ac9a).  A body starts
    # and stops with it at its lowest, so the light fades in and out with the
    # body -- which only reads true with each opcode on its own keyframe.  The
    # control attributes every keyframe's time and opcode to the keyframe
    # before it, the way this reader once did.
    faded = windows_seen = shifted_faded = 0
    for path in files:
        try:
            atmosphere = sky.load(path)
        except sky.SkyFormatError:
            continue
        for index in range(atmosphere.section_count):
            ordered_frames = atmosphere.section_keyframes(index)
            for kind, start, stop in atmosphere.windows(index):
                if kind not in sky.BODY_ANGLES or stop is None:
                    continue
                i, j = ordered_frames.index(start), ordered_frames.index(stop)
                inside = ordered_frames[i + 1:j]
                if not inside:
                    continue
                windows_seen += 1
                faded += max(start.light, stop.light) <= min(k.light for k in inside)
                before = ordered_frames[i - 1:j] if i else []
                if len(before) >= 3:
                    shifted_faded += (max(before[0].light, before[-1].light)
                                      <= min(k.light for k in before[1:-1]))
    check("sky.ske: a body starts and stops with the light at its lowest",
          windows_seen and faded == windows_seen and shifted_faded < windows_seen // 2,
          f"on {faded}/{windows_seen} sun and moon windows with keyframes inside, the "
          f"third float at the start and the stop is no higher than anywhere between "
          f"(0.0-0.2 against up to 5.0); control: moved one keyframe back, as an "
          f"earlier reading of the file attributed them, it holds on {shifted_faded}")

    # Slot 18 is the cloud layer's material colour (Terrain.dll:0x1007a4de).
    # Clouds are brightest when the light is: red at a body's rise, near white
    # at the brightest keyframe.  Control: slot 16, which nothing reads.
    def luminance(c):
        return 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]
    lit_clouds = lit_control = varying_files = 0
    for path in files:
        try:
            atmosphere = sky.load(path)
        except sky.SkyFormatError:
            continue
        group = atmosphere.section_keyframes(0)
        peak = max(group, key=lambda k: k.light)
        if all(k.light == peak.light for k in group):
            continue
        varying_files += 1
        lit_clouds += luminance(peak.cloud_colour) >= max(
            luminance(k.cloud_colour) for k in group)
        lit_control += len({k.slots[16] for k in group}) > 1 and luminance(
            peak.colour(16)) >= max(luminance(k.colour(16)) for k in group)
    check("sky.ske: the clouds are brightest at the brightest keyframe",
          varying_files and lit_clouds >= varying_files * 0.8 and lit_control < varying_files // 2,
          f"on {lit_clouds}/{varying_files} files whose light varies, slot 18 -- the cloud "
          f"layer's colour -- is at its brightest where the third float is (#f0f5ff on "
          f"Mission 01, #ac2800 as the sun rises); control: slot 16 does so on {lit_control}")

    # Where the sun stands is not in the file -- CSun takes two whole-degree
    # angles from a block Terrain.dll fills with constants, picked by whether
    # the starting keyframe's name is exactly "sun".  Nothing in the data can
    # confirm a constant directly, but three things it implies are checkable.

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
            if k.body:
                named[k.name] = named.get(k.name, 0) + 1
        for section in range(atmosphere.section_count):
            sections += 1
            window: dict[str, list[tuple[int, int]]] = {}
            for kind, start, stop in atmosphere.windows(section):
                if kind in sky.BODY_ANGLES and stop is not None:
                    window.setdefault(kind, []).append((start.minutes, stop.minutes))
            if all(len(window.get(name, [])) == 1 for name in sky.BODY_ANGLES):
                paired += 1
                (rise, set_), (moonrise, moonset) = window["sun"][0], window["moon"][0]
                overlaps += not (set_ <= moonrise or moonset <= rise)
            else:
                odd.append(f"{path.parent.name}/{section}")
    bodies = {n: named.get(n, 0) for n in sky.BODY_ANGLES}
    check("sky.ske: the only bodies a starting keyframe names are the sun and the moon",
          set(named) == set(sky.BODY_ANGLES),
          f"{bodies} starts against the engine's single test, name == 'sun'")
    check("sky.ske: the sun and the moon each run from a start to a stop",
          paired >= sections - 2,
          f"{paired}/{sections} sections hold exactly one sun and one moon window, opcode "
          f"0 to opcode 1; the other {len(odd)} are the two 24-hour skies, which run the "
          f"sun twice and never the moon")
    check("sky.ske: the sun and the moon are never up together", overlaps == 0,
          f"{overlaps} of {paired} such sections overlap -- the sun runs about 00:30 to "
          f"14:30 and the moon 15:30 to 23:30, which is what makes two fixed positions a "
          f"quarter turn apart coherent")

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


#: The one header +0x14 bit shipped textures set, on 81 ARGB8888 textures.
TEXTURE_BIT_26 = 0x04000000
MISSION_01_SKY = "MISSIONS/CAMPAIGN/CAMPAIGN.00/Mission.01/sky.ske"


def check_render_state(check, game: Path) -> None:
    """What the renderer does: texture alpha, the sky's fog, dome and settings."""
    archive = NResArchive.open(game / "Textures.lib")
    word14: Counter[int] = Counter()
    carriers: set[int] = set()
    later = 0
    formats: Counter[int] = Counter()
    keyed = palettised = 0
    for entry in archive:
        data = archive.read(entry)
        if data[:4] != b"Texm":
            continue
        _m, _w, _h, _mips, _flags, u14, u18, fmt = texm.HEADER.unpack_from(data, 0)
        word14[u14] += 1
        if u14:
            carriers.add(fmt)
        later += u18 != 0
        formats[fmt] += 1
        if fmt == texm.FMT_PALETTE8:
            palettised += 1
            tex = texm.decode(data)
            body = data[texm.HEADER_SIZE + texm.PALETTE_SIZE:]
            keyed += 0 in body[: tex.width * tex.height]
    total = sum(formats.values())
    alpha = sum(n for f, n in formats.items() if texm.uploads_with_alpha(f))
    check("Textures.lib: nothing is colour-keyed; only 4444 and 8888 have alpha",
          set(word14) == {0, TEXTURE_BIT_26} and carriers == {texm.FMT_ARGB8888}
          and later and palettised and keyed == 0
          and alpha == formats[texm.FMT_ARGB4444] + formats[texm.FMT_ARGB8888],
          f"header +0x14 is 0 on {word14[0]} and {TEXTURE_BIT_26:#x} on "
          f"{word14[TEXTURE_BIT_26]} (all 8888) of {total}, so neither alpha bit "
          f"Ngi32.dll's upload tests (0x1000fdf6) is set: {alpha} textures, the 4444 "
          f"and 8888 ones, get an alpha surface and the rest an opaque one.  No "
          f"palettised texture draws index 0, the one an alpha surface would clear "
          f"({keyed} of {palettised}).  Control: +0x18 is non-zero on {later}")

    # Which textures World3D.dll loads opaque: a lit skin's (directory flags
    # bit 1), unless EMBOSS_BUMP is on.  Who names the alpha textures, the
    # cutouts and the 81 carrying header bit 26.
    mats = materials.MaterialLibrary(game / "Material.lib")
    named_by: dict[str, set[int]] = defaultdict(set)
    for m in mats.materials.values():
        for t in m.textures:
            named_by[t.upper()].add(m.blend)
    alpha_names = []
    bit26_names = []
    cutouts = []
    for entry in archive:
        data = archive.read(entry)
        if data[:4] != b"Texm":
            continue
        _m, _w, _h, _mips, _flags, u14, _u18, fmt = texm.HEADER.unpack_from(data, 0)
        name = entry.name.upper()
        if u14 & texm.HEADER_BIT_26:
            bit26_names.append(name)
        if texm.uploads_with_alpha(fmt, u14):
            alpha_names.append(name)
            if texm.is_cutout(texm.decode(data)):
                cutouts.append(name)

    def opaque(name: str) -> bool:
        return any(texm.material_load_flags(b) & texm.LOAD_OPAQUE for b in named_by[name])
    skins = [n for n in alpha_names if opaque(n)]
    skin_cutouts = sorted(n for n in cutouts if opaque(n))
    trees = [n for n in cutouts if "TREE" in n]
    bit26_skins = [n for n in bit26_names if opaque(n)]
    check("Material.lib: a lit skin's texture loads opaque, whatever its format",
          len(skins) == 171 and skin_cutouts == ["AIM_02.0", "PI_CSPG3.0", "S7.0"]
          and trees and not any(opaque(n) for n in trees)
          and len(bit26_skins) == 78 and len(bit26_names) == 81,
          f"World3D.dll:0x10004441 ORs load flag 0x80000 (Ngi32.dll:0x1000fe18, opaque "
          f"surface) into the texture of every material with directory flags bit 1 "
          f"unless EMBOSS_BUMP is on: that is {len(skins)} of the {len(alpha_names)} "
          f"4444/8888 textures, {len(skin_cutouts)} of the {len(cutouts)} cut-outs "
          f"({', '.join(skin_cutouts)}) and none of the trees ({', '.join(trees)}); "
          f"{len(bit26_skins)} of the {len(bit26_names)} textures with header bit 26 "
          f"are lit skins' and the other {len(bit26_names) - len(bit26_skins)} are named "
          f"by no material")

    starts: Counter[float] = Counter()
    ends: Counter[int] = Counter()
    off = keyframes = 0
    ordered = backwards = files = sides = 0
    for path in sorted(game.glob("MISSIONS/**/sky.ske")):
        atmosphere = sky.load(path)
        for k in atmosphere.keyframes:
            keyframes += 1
            starts[k.fog_start] += 1
            ends[round(k.fog_end)] += 1
            off += 0 < k.number(sky.RING3_SLOTS[0]) <= 1
            sides += k.slots[sky.HORIZON_SLOTS[1]] == k.slots[sky.HORIZON_SLOTS[3]]
        k = atmosphere.brightest()
        files += 1

        def lum(c):
            return 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]
        apex, ring2, ring3 = (sum(lum(c) for c in ring) / len(ring) for ring in k.rings[:3])
        ordered += apex <= ring2 <= ring3
        backwards += ring3 <= ring2 <= apex
    check("sky.ske: slots 5 and 6 are the fog's start and end over 700",
          set(starts) == {0.0} and min(ends) >= 70 and max(ends) <= 700 and off == 0,
          f"the start is 0 on all {starts[0.0]} keyframes and the end {min(ends)} to "
          f"{max(ends)} units, 700 on {ends[700]} (Terrain.dll:0x1007bbc5); control: "
          f"slot 7 read as a float lands in (0, 1] on {off}")
    first = sky.load(game / MISSION_01_SKY)
    first_ends = Counter(round(k.fog_end) for k in first.keyframes)
    check("Mission 01: the fog ends between 420 and 700 units",
          sorted(first_ends) == [420, 490, 525, 560, 700],
          f"{dict(sorted(first_ends.items()))} over {len(first.keyframes)} keyframes of "
          f"a {first.day_seconds:g}-second day")
    check("sky.ske: the dome darkens from ring 3 up to the apex",
          files and ordered == files and backwards < files and sides > 0.9 * keyframes,
          f"at the brightest keyframe of {ordered} of {files} files the apex (slot 15) is "
          f"no brighter than ring 2 (11-14), and ring 2 no brighter than ring 3 (7-10); "
          f"control: the reverse holds on {backwards}.  The horizon on +x and -x "
          f"(slots 3 and 4) match on {sides} of {keyframes}")

    found = [p for p in game.rglob("*") if p.name.lower() == "shade.cfg"]
    cfgs = list(game.rglob("*.cfg"))
    check("install: no shade.cfg ships, so the renderer's compiled defaults apply",
          not found and cfgs,
          f"{len(found)} shade.cfg among {len(cfgs)} .cfg files; Terrain.dll reads it "
          f"at 0x1005f652 and falls back to ForceSWFog 1, AtmSkyDetail 4 "
          f"({2 ** 4} dome segments) at 0x1005fa80")


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

    # Sub-object flag bit 5 marks the cockpit, and the name says the same
    # thing: the two agree exactly.
    marked = named = drawable = agree = hull_triangles = 0
    for _name, m in meshes:
        for node in m.nodes:
            hull = node.name.split("_")[0] in ("CP", "BTCP")
            triangles = sum(
                m.slots[i].triangle_count
                for i in node.slots_for_lod(0)
                if i < len(m.slots)
            )
            marked += node.is_cockpit
            named += hull
            if hull and triangles:
                drawable += 1
                agree += node.is_cockpit
                hull_triangles += triangles
    # The fifth slot of a variant is what the unit's own view draws: the nodes
    # carrying only that slot are exactly the cockpit nodes, and no ordinary
    # node shares its fifth slot with one of its levels.
    only_fifth, hulls, shared, both = set(), set(), 0, 0
    for mesh_name, m in meshes:
        for k, node in enumerate(m.nodes):
            levels = [node.slot_index[i] for i in range(objmesh.LOD_COUNT)
                      if node.slot_index[i] != objmesh.NO_SLOT]
            fifth = node.cockpit_slot()
            if node.is_cockpit:
                hulls.add((mesh_name, k))
            if fifth is None:
                continue
            if not levels:
                only_fifth.add((mesh_name, k))
            else:
                both += 1
                shared += fifth in levels
    check("MESH: the nodes with only a fifth slot are the flag-0x20 nodes",
          only_fifth == hulls and hulls and shared == 0,
          f"the {len(only_fifth)} nodes carrying only a fifth slot are exactly "
          f"the {len(hulls)} flagged 0x20; on the {both} ordinary nodes that "
          f"carry both, the fifth is a separate slot on all {both - shared}")

    check("MESH: flag bit 0x20 marks exactly the CP_* nodes",
          marked == agree == drawable > 0 and marked < named,
          f"{marked} nodes carry the bit and every one is named CP_* or BTCP_*; "
          f"{agree}/{drawable} of those names that have geometry carry it, and "
          f"the {named - drawable} that do not are empty. {hull_triangles} "
          f"triangles that only the unit's own view draws")

    # Those nodes are where the first-person view sits.  The turret's camera
    # component registers its view with the unit's mesh (Control.dll:0x1002399a),
    # the mesh draws its fifth slots to a registered view
    # (AniMesh.dll:0x10014be5), and the view's eye is at CameraCenter.
    library = objects.ObjectLibrary(game / "objects.rlb")
    by_member = {name.lower(): m for name, m in meshes}
    cockpit_archives: Counter[str] = Counter()
    for name in archives:
        path = game / name
        if not path.exists():
            continue
        for entry in NResArchive.open(path):
            found = by_member.get(entry.name.lower())
            if found is not None and any(n.is_cockpit for n in found.nodes):
                cockpit_archives[name] += 1
    eyes = eyes_in = without = 0
    opened_cpt: dict[str, NResArchive] = {}
    for record in library.records.values():
        mref, cref = record.mesh, record.slot_with_suffix("cpt")
        if not mref or not cref:
            continue
        found = by_member.get(mref.member.lower())
        if found is None:
            continue
        lib_name = cref.library.lower()
        if lib_name not in opened_cpt:
            opened_cpt[lib_name] = NResArchive.open(game / lib_name)
        try:
            points = objmesh.parse_control_points(
                opened_cpt[lib_name].read_name(cref.member), cref.member)
        except KeyError:
            continue
        has_cockpit = any(n.is_cockpit for n in found.nodes)
        for point in points:
            if point.name.lower() != "cameracenter":
                continue
            if not has_cockpit:
                without += 1
                continue
            eyes += 1
            node = point.placed_on
            eyes_in += 0 <= node < len(found.nodes) and found.nodes[node].is_cockpit
    check("MESH: the flag-0x20 nodes are the cockpit the turret's camera sits in",
          set(cockpit_archives) == {"turrets.rlb"} and eyes and eyes_in == eyes,
          f"the meshes that carry them are all in turrets.rlb "
          f"({dict(cockpit_archives)}), and on {eyes_in}/{eyes} records whose "
          f"mesh has one the CameraCenter control point sits on it; "
          f"{without} camera points belong to meshes without one")

    # And dropping them is what makes every model fit the box it states.
    def within(m: objmesh.ObjectMesh, skip_hulls: bool) -> bool:
        lo = [math.inf] * 3
        hi = [-math.inf] * 3
        for i, node in enumerate(m.nodes):
            if skip_hulls and node.is_cockpit:
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
    check("MESH: level 0 fits inside the authored box once cockpits are dropped",
          without == len(boxed) > with_hulls,
          f"{without}/{len(boxed)} models fit inside their own box against "
          f"{with_hulls}/{len(boxed)} while the cockpit nodes are drawn")

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


#: A mesh node flag the life loader turns into "never hidden" (``Control.dll:0x1000f9bd``).
NODE_NEVER_HIDDEN = 0x100
#: A mesh node flag the life loader turns into "vital" (``Control.dll:0x1000f9aa``).
NODE_VITAL = 0x200

#: Mission 01's dummies, by chassis: each node's name, parent, stage count, hit points
#: and explosion, and the controller's +92, the ms a dead unit lasts.
MISSION_01_DUMMIES = {
    "r_h_01": ([("ASbs_m1o1", None, 1, 500.0), ("ASd1_m1o1", 0, 2, 800.0),
                ("ASd2_m1o1", 0, 2, 800.0), ("ASd3_m1o1", 2, 2, 600.0)],
               "explode_aim_S.exp", 3000),
    "r_h_03": ([("ALbs_m1o1", None, 1, 1400.0)]
               + [(f"ALd{k}_m1o1", 0, 2, 1000.0) for k in range(1, 6)],
               "explode_aim_L.exp", 5000),
}


def _stage_count(node) -> int:
    """How many of blocks 0, 1 and 2 in a row have a level-0 slot (``AniMesh.dll:0x10005840``)."""
    n = 0
    for variant in range(objmesh.VARIANT_COUNT):
        if node.slot_index[variant * objmesh.SLOTS_PER_VARIANT] == objmesh.NO_SLOT:
            break
        n += 1
    return n


def check_node_stages(check, game: Path) -> None:
    """What a damaged node draws, which nodes are never hidden, and how long the dead last."""
    library = objects.ObjectLibrary(game / "objects.rlb")
    opened: dict[str, NResArchive] = {}

    def member(ref) -> bytes | None:
        if ref is None or not ref.member:
            return None
        try:
            if ref.library not in opened:
                opened[ref.library] = NResArchive.open(game / ref.library)
            return opened[ref.library].read_name(ref.member)
        except (KeyError, ValueError, FileNotFoundError):
            return None

    stages: Counter[int] = Counter()
    never_hidden: Counter[str] = Counter()
    chassis_delay: Counter[int] = Counter()
    for record in library.records.values():
        blob = member(record.mesh)
        if blob is not None:
            try:
                model = objmesh.parse(blob, record.mesh.member)
            except (ValueError, struct.error):
                model = None
            for node in model.nodes if model else ():
                stages[_stage_count(node)] += 1
                if node.flags & NODE_NEVER_HIDDEN:
                    never_hidden[record.name] += 1
        if record.name.lower().startswith("r_"):
            raw = member(record.slot_with_suffix("ctl"))
            if raw is not None:
                chassis_delay[control.parse(raw).scale] += 1

    check("MESH: a node's stages are its blocks with a level-0 slot in a row",
          stages[1] > stages[2] > stages[3] > 0 and stages[0] > 0,
          f"1 on {stages[1]} nodes, 2 on {stages[2]}, 3 on {stages[3]}, and no "
          f"level-0 slot on {stages[0]} (the life loader counts those as 1, "
          f"Control.dll:0x1000fa0b)")
    check("MESH: only building shells are never hidden (node flag 0x100)",
          sum(never_hidden.values()) > 0
          and all(name.lower().startswith("bu_") for name in never_hidden),
          f"{sum(never_hidden.values())} nodes over {len(never_hidden)} records, all "
          f"bu_* buildings; every other node is hidden at its last stage "
          f"(Control.dll:0x100118cd)")
    check("controller +92: a dead chassis lasts one to five seconds",
          set(chassis_delay) <= {1000, 2000, 3000, 5000} and chassis_delay[2000] > 0,
          f"{dict(sorted(chassis_delay.items()))} over the r_* chassis: ms added to the "
          f"clock at death before KillGameObject (Control.dll:0x100110b1)")

    seen = {}
    for name in MISSION_01_DUMMIES:
        record = library.get(name)
        model = objmesh.parse(member(record.mesh), record.mesh.member)
        table = objects.parse_damage(member(record.damage), record.damage.member)
        rows = [(node.name, None if node.parent == objmesh.NO_PARENT else node.parent,
                 _stage_count(node), row.durability)
                for node, row in zip(model.nodes, table, strict=True)]
        explosions = {row.explosion.member for row in table}
        flagged = any(node.flags & (NODE_NEVER_HIDDEN | NODE_VITAL) for node in model.nodes)
        delay = control.parse(member(record.slot_with_suffix("ctl"))).scale
        seen[name] = (rows, explosions, flagged, delay)
    check("Mission 01's dummies: parts of two stages on a base of one",
          all(seen[n][0] == rows and seen[n][1] == {exp} and not seen[n][2]
              and seen[n][3] == delay
              for n, (rows, exp, delay) in MISSION_01_DUMMIES.items()),
          "; ".join(f"{n}: " + ", ".join(f"{r[0].split('_')[0]} {r[2]}x{r[3]:g}" for r in rows)
                    + f", {'/'.join(sorted(exp))}, +92 {delay}, "
                    f"{'a vital or kept node' if flagged else 'no vital or kept node'}"
                    for n, (rows, exp, flagged, delay) in seen.items()))

    # The knock-off switch and its flight time are set once, by a static initialiser.
    image = (game / "Control.dll").read_bytes()
    sections, _ = resources._sections(image)
    at = resources._offset(sections, 0x100063B4 - 0x10000000)
    stored = image[at:at + 17]
    switch = stored[:7] == bytes.fromhex("c605a024041001")
    flight = (stored[7:13] == bytes.fromhex("c705a4240410")
              and struct.unpack_from("<f", stored, 13)[0])
    check("Control.dll: a destroyed part is knocked off and flies for 3000 ms",
          switch and flight == 3000.0 and b"KillGameObject" in image,
          f"0x100063b4 stores 1 in the switch at 0x100424a0 and {flight} in 0x100424a4; "
          f"KillGameObject imported {b'KillGameObject' in image}")


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
    check("Effect.dll: a class loads under half its block straight off the pointer",
          recorded < slots / 2,
          f"{recorded} of the {slots} four-byte slots across the ten block "
          f"types are loaded with fld straight off the block pointer; the map is a "
          f"lower bound -- exponent triples read through a pointer into the block "
          f"(types 3, 4, 8 and 9 at +64 and +124, Effect.dll:0x100106f6) and fields "
          f"copied as dwords first (type 1's light) are live too")

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

    check_ctl_fields(check, game, blobs, parsed)


#: The component classes the factory builds as its generic device
#: (``Control.dll:0x1002d6ec``, ``0x10020800``).
GENERIC_DEVICE_TYPES = frozenset({3, 6, 7, 11, 12, 13, 14, 16, 18, 20, 21, 22, 23, 24, 25, 28, 29})


def check_ctl_fields(check, game: Path, blobs, parsed) -> None:
    """.ctl: component entries, section 1's contacts, the lean, triple 2, device inputs."""
    entries = [(x, c.counts[2]) for c in parsed for k in c.components for x in k.entries]
    channels = sum(c.counts[2] for c in parsed)
    named = sum(len({x for k in c.components for x in k.entries}) for c in parsed)
    check(".ctl: a component's entries are its controller's channels",
          entries and all(0 <= x < n for x, n in entries),
          f"all {len(entries)} entries on "
          f"{sum(1 for c in parsed for k in c.components if k.entries)} components index "
          f"section 2 (Control.dll:0x10021de7 rebases them by the part's first channel); "
          f"{named} of the {channels} channels are driven by some component")

    points: dict[tuple[str, str], list] = {}
    ndp_flags: dict[tuple[str, str], list[int]] = {}
    for path in all_archives(game):
        archive = NResArchive.open(path)
        for entry in archive:
            stem = entry.name.lower().rsplit(".", 1)[0]
            if entry.name.lower().endswith(".cpt"):
                points[(path.name.lower(), stem)] = objmesh.parse_control_points(
                    archive.read(entry))
            elif entry.name.lower().endswith(".ndp"):
                raw = archive.read(entry)
                ndp_flags[(path.name.lower(), stem)] = [
                    struct.unpack_from("<i", raw, 4 + 76 * i)[0]
                    for i in range(struct.unpack_from("<i", raw, 0)[0])]

    states = same = gaps = resolved = with_cpt = contacts = good_groups = 0
    contact_groups: dict[tuple[str, str], set[int]] = {}
    reached: dict[tuple[str, str], set[int]] = {}
    limping: Counter[str] = Counter()
    gear: Counter[int] = Counter()
    names: Counter[str] = Counter()
    for (lib, name, blob), c in zip(blobs, parsed, strict=True):
        key = (lib.lower(), name.lower().rsplit(".", 1)[0])
        reached[key] = ({g for g in c.groups if g != control.NO_GROUP}
                        | {s.actions for s in c.states} | {k.group for k in c.components})
        stride = control.SECTION1_RECORD + control.SECTION1_PER_B * c.counts[1]
        for i, s in enumerate(c.states):
            if not s.contacts:
                continue
            states += 1
            same += [k.point for k in s.contacts] == [k.point for k in c.states[0].contacts]
            pts = points.get(key)
            for j, k in enumerate(s.contacts):
                contacts += 1
                at = control.HEADER_SIZE + i * stride + control.SECTION1_RECORD + 16 * j
                gaps += struct.unpack_from("<i", blob, at + 12)[0] == 0
                good_groups += k.group == control.NO_GROUP or 0 <= k.group < c.counts[4]
                if k.group != control.NO_GROUP:
                    contact_groups.setdefault(key, set()).add(k.group)
                if pts is not None:
                    with_cpt += 1
                    if 0 <= k.point < len(pts):
                        resolved += 1
                        names[re.sub(r"[_\d].*|(Left|Right)", "", pts[k.point].name)] += 1
            if any(k.flags & control.NEEDS_DESTROYED for k in s.contacts):
                limping[name.lower()] += 1
                for k in s.contacts:
                    node = pts[k.point].nodes[1] if pts and 0 <= k.point < len(pts) else -1
                    flags = ndp_flags.get(key, [])
                    gear[flags[node] & 0x60 if 0 <= node < len(flags) else -1] += 1
    only = sum(len(g - reached[key]) for key, g in contact_groups.items())
    check(".ctl: a state's contacts are control points, the same in every state",
          states and same == states and gaps == contacts and resolved == with_cpt > 0
          and good_groups == contacts and only == 22,
          f"{contacts} contacts on {states} states; the points agree with state 0's on "
          f"{same}; +12 is zero on {gaps}; {resolved}/{with_cpt} index the same-stem .cpt "
          f"where there is one ({dict(names.most_common(5))}); +8 is -1 or a group on "
          f"{good_groups}, and {only} groups are reached only from a contact -- footsteps")

    check(".ctl: the states that need a destroyed contact are walkers limping",
          sum(limping.values()) == 102 and set(gear) <= {0x20, 0x40} and gear[0x20] == gear[0x40],
          f"{dict(limping)} states carry 0x200 (Control.dll:0x10001107 wants that contact's "
          f"node destroyed); every contact on them sits on a running-gear node, "
          f"left {gear[0x20]} and right {gear[0x40]}")

    selecting: dict[str, set[int]] = defaultdict(set)
    lean_states = 0
    sources = set()
    for (_lib, name, _blob), c in zip(blobs, parsed, strict=True):
        for s in c.states:
            if any(s.lean):
                lean_states += 1
            for axis, sel in enumerate(s.lean):
                if sel:
                    selecting[name.lower()].add(axis)
                    sources.add(sel & ~control.LEAN_NEGATE)
    known = set(control.LEAN_TURN + control.LEAN_VELOCITY + control.LEAN_ACCELERATION)
    by_name = {(n.lower()): c for (_l, n, _b), c in zip(blobs, parsed, strict=True)}
    authored = sum(1 for n, axes in selecting.items() for a in axes
                   if by_name[n].triples[control.TRIPLE_LEAN][a] < 6.0)
    selected = sum(len(a) for a in selecting.values())
    z_default = sum(1 for n in selecting
                    if abs(by_name[n].triples[control.TRIPLE_LEAN][2] - control.FULL_TURN) < 1e-4)
    check(".ctl: a state's lean selects a source, and triple 6 bounds it",
          lean_states == 28 and len(selecting) == 16 and sources <= known
          and authored == selected and z_default == len(selecting),
          f"{lean_states} states on {len(selecting)} controllers pick sources "
          f"{sorted(sources)} (Control.dll:0x10014e8d); every axis they lean carries an "
          f"authored triple-6 limit below a turn, {authored} of {selected}, and z, which "
          f"none leans, keeps 6.28 on all {z_default}")

    unread = sum(1 for c in parsed if not any(c.triples[control.TRIPLE_UNREAD]))
    check(".ctl: triple 2 is mostly left zero",
          unread == 518,
          f"{unread} of {len(parsed)} leave +32..+40 at zero; nothing in Control.dll reads "
          f"them in either copy of the block")

    devices = [k for c in parsed for k in c.components if k.type_id in GENERIC_DEVICE_TYPES]
    selectors = Counter(s for k in devices for s in k.inputs if s)
    pairs = Counter((k.inputs[1:], k.weights) for k in devices if any(k.inputs[1:]))
    inputs = sorted({control.device_input(s) for s in selectors} - {None})
    check(".ctl: a generic device's selectors name motion inputs",
          devices and all(s == 1 or control.device_input(s) for s in selectors)
          and set(pairs) == {((12, 7), (1.0, 0.5)), ((12, 4), (1.0, 0.5))},
          f"{len(devices)} generic devices use selectors {dict(selectors)} "
          f"(Control.dll:0x10020d90): {inputs}; "
          f"the wheels' two inputs are the forward speed and half the yaw rate either "
          f"way, {dict(pairs)}")


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

    # The lean: state +0x08 picks a source per axis, triple 6 is its limit.
    leaning = [(name, s) for _lib, name, c in every for s in c.states if s.lean_word]
    words = Counter(s.lean_word for _, s in leaning)
    valid = all(((s.lean_word >> (8 * axis)) & 0x7F) in set(control.LEAN_SOURCES) | {0}
                for _, s in leaning for axis in (0, 1))
    no_yaw = all(not s.lean_word & 0xFFFF0000 for s in states)
    wheels = {name for name, s in leaning if s.lean_word == 0x8389}
    hero_leans = any(s.lean_word for lib, name, c in every
                     if (lib, name) == ("bases.rlb", "r_h_02.ctl") for s in c.states)
    check(".ctl: a state's +0x08 picks what leans the hull on pitch and roll",
          valid and no_yaw and set(words) == {0x0386, 0x8389, 0x8405}
          and wheels == {f"r_{s}_0{k}.ctl" for s in "lmb" for k in (3, 4)} and not hero_leans,
          f"{len(leaning)} of {len(states)} states in {len({n for n, _ in leaning})} "
          f"controllers set it: " + ", ".join(f"{w:#06x} on {n}" for w, n in sorted(words.items()))
          + "; every selector names a source the spin integrator knows "
          "(Control.dll:0x1001538c), byte 2 (yaw) is 0 on all, and 0x8389 is on exactly "
          "the six wheeled and tracked chassis.  The lean is the source's fraction x "
          "triple 6 (0x10014f80)")

    righting = Counter(s.mode & (control.STATE_RIGHT_UP | control.STATE_RIGHT_TO_VECTOR
                                 | control.STATE_NO_RIGHTING) for s in states)
    mixed = sum(1 for s in states if s.mode & control.STATE_RIGHT_UP
                and s.mode & control.STATE_RIGHT_TO_VECTOR)
    by_name = defaultdict(set)
    for _lib, name, c in every:
        for s in c.states:
            by_name[name].add(s.mode & (control.STATE_RIGHT_UP | control.STATE_RIGHT_TO_VECTOR))
    ground = {f"r_{s}_0{k}.ctl" for s in "lmb" for k in (3, 4)}
    check(".ctl: a state's +0x04 bits right the hull toward up or toward the ground",
          not mixed and all(by_name[n] == {control.STATE_RIGHT_TO_VECTOR} for n in ground)
          and by_name["r_h_02.ctl"] == {control.STATE_RIGHT_UP}
          and righting[control.STATE_RIGHT_UP] and righting[control.STATE_RIGHT_TO_VECTOR],
          f"of {len(states)} states {righting[control.STATE_RIGHT_UP]} set 0xC0 (up), "
          f"{righting[control.STATE_RIGHT_TO_VECTOR]} set 0x30 (the vector at +0x348), "
          f"{righting[control.STATE_NO_RIGHTING]} set 0x400000 (skip) and "
          f"{righting[0]} none; none mixes the two.  All six wheeled and tracked "
          f"chassis's states take 0x30, every hero state 0xC0 (Control.dll:0x1000c3a2)")

    idle = {k: round(c.triples[control.TRIPLE_IDLE][1], 3) for k, c in joined.items()}
    floors = Counter(idle.values())
    check(".ctl: triple 2's forward component, the AI walker's floor, is under 2 m/s",
          idle and set(floors) <= {0.0, 0.1, 0.49, 0.6}
          and all(idle[p] == 0.6 for p in ("R_L_01", "R_M_01", "R_B_01", "R_B_05", "R_H_02"))
          and idle["R_T_01"] == 0.49,
          ", ".join(f"{v:g} on {n}" for v, n in sorted(floors.items()))
          + f" of {len(idle)} chassis; Behavior.dll:0x1003bed0 holds an order's speed "
          f"above it x Movement_MinSpeedPercent, and 0x1003bf05 above 2")


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
    walked: Counter[str] = Counter()
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
        _tally_walks(land, d.name.lower() == "tut_1", walked)
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
          f"{landmesh.WALKABLE_NORMAL_Z} (cos 80 degrees), which neither of the ground "
          f"contact's searches accepts (Control.dll:0x1001a6fd) and its walk leaves at "
          f"once (Terrain.dll:0x10026630)")
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
    # How a state is weighed and planned, and which states follow the ground.
    edges = scaled = crossed = gap_agrees = 0
    widest = 1.0
    blending = all_axes = spanned = still = 0
    unbounded: Counter[str] = Counter()
    contact_states = contact_marked = marked = 0
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

            n = len(c.states)
            for to in range(n):
                for frm in range(n):
                    raw = c.cost(to, frm)
                    if raw >= control.NO_EDGE:
                        continue
                    factor = c.transition_factor(to, frm)
                    edges += 1
                    widest = max(widest, factor)
                    scaled += raw > 0 and factor > 1
                    crossed += raw * factor >= control.NO_EDGE
                    # Control: the gap between the two velocity boxes.
                    gap = max((max(0.0, c.states[frm].velocity[0][a] - c.states[to].velocity[1][a],
                                   c.states[to].velocity[0][a] - c.states[frm].velocity[1][a])
                               for a in range(3) if c.states[frm].flags >> a & 1), default=0.0)
                    gap_agrees += abs(1 + gap - factor) < 1e-4
            for s in c.states:
                if s.mode & control.STATE_UNBOUNDED:
                    unbounded[path.name.lower()] += 1
                marked += bool(s.mode & control.STATE_GROUND_CONTACTS)
                if c.counts[1]:
                    contact_states += 1
                    contact_marked += bool(s.mode & control.STATE_GROUND_CONTACTS)
                if not s.blends:
                    continue
                blending += 1
                all_axes += s.flags & 7 == 7
                lo, hi = s.velocity
                if s.blend_divisor > 0:
                    spanned += abs(s.blend_divisor - max(h - m for m, h in zip(lo, hi,
                                                                              strict=True))) < 1e-4
                else:
                    still += s.blend == 1.0

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

    check(".ctl: the loader scales a transition by the centre-to-minimum reach, not the box gap",
          edges and scaled and crossed == 0 and gap_agrees < edges,
          f"{edges} edges under 1,000,000; the factor 1 + largest |centre(to) - min(from)| "
          f"over velocity axes + the same over spin axes (Control.dll:0x10001790) changes "
          f"{scaled} non-zero costs, at most x{widest:.1f}, and lifts none to 1,000,000.  "
          f"Control: one plus the gap between the velocity boxes gives the same factor on "
          f"only {gap_agrees}")
    check(".ctl: every blended state switches on all three velocity axes, and D is its span",
          blending and all_axes == blending and spanned + still == blending and still,
          f"{blending} states weigh their pairs by speed (Control.dll:0x1000555b); all switch "
          f"on x, y and z, so lo = min(largest |min|, largest |max|) is finite.  "
          f"D = largest ||max| - |min|| (0x100055d4) equals the box's largest span on "
          f"{spanned}; on the other {still} D is 0, the weight stays 1, and their blend base "
          f"is 1 anyway")
    check(".ctl: state bit 0x40000, unbounded motion, is the rounds', trees' and stones'",
          set(unbounded) == {"weapon.rlb", "static.rlb", "system.rlb"},
          f"{dict(sorted(unbounded.items()))}: it skips the top-speed clamp, the slope brake "
          f"and the turn-rate clamp (Control.dll:0x1001461e, 0x10015681, 0x10014c02); no "
          f"chassis or animal state carries it")
    check(".ctl: state bit 0x4 is set exactly on the states of controllers with contact points",
          marked and contact_marked == contact_states == marked,
          f"{marked} states carry it, all {contact_states} states of the controllers whose "
          f"counts[1] declares contact points; with it the body falls under the world's "
          f"gravity until a flagged contact reaches its ground (Control.dll:0x10015d60), "
          f"without it the sphere is only lifted out of the ground (0x1001b3c3)")

    # The mesh walk behind FindWorldFace: edges, winding and the crossing rule.
    check("Land.msh: adjacency slot e is the edge from vertex e to e+1, and faces wind CCW",
          walked["shared"] and walked["by_edge"] == walked["shared"]
          and walked["ccw"] == walked["up"],
          f"{walked['by_edge']} of {walked['shared']} neighbours share the positions of "
          f"vertices e and e + 1; {walked['ccw']} of the {walked['up']} walkable faces wind "
          f"counter-clockwise seen from above.  FindWorldFace's inside test and its "
          f"crossing rule (Terrain.dll:0x10026340) both assume this")
    check("Tut_1: the FindWorldFace rule walks to the face under the target",
          walked["walks"] and walked["arrived"] == walked["walks"],
          f"{walked['arrived']} of {walked['walks']} walks from a level-0 face's centroid to "
          f"a point 7.6 or 10.3 away end on the one walkable level-0 face under it, "
          f"crossing the edge whose first vertex lies right of the line and whose second "
          f"lies left")


def _tally_walks(land, sample: bool, out: Counter) -> None:
    """Edge order and winding on every map; sampled walks on one (check_ground)."""
    pos = land.positions

    def key(v):
        return tuple(round(x, 3) for x in pos[v])

    for f, vs in enumerate(land.faces):
        for e, nb in enumerate(land.adjacency[f]):
            if nb == landmesh.NO_NEIGHBOUR:
                continue
            out["shared"] += 1
            out["by_edge"] += {key(vs[e]), key(vs[(e + 1) % 3])} <= {key(v) for v in land.faces[nb]}
        if land.face_normal[f][2] > landmesh.WALKABLE_NORMAL_Z:
            out["up"] += 1
            a, b, c = (pos[v] for v in vs)
            out["ccw"] += (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]) > 0
    if not sample:
        return
    faces0 = land.lod_faces(0)
    boxes = {g: (min(pos[v][0] for v in land.faces[g]), max(pos[v][0] for v in land.faces[g]),
                 min(pos[v][1] for v in land.faces[g]), max(pos[v][1] for v in land.faces[g]))
             for g in faces0}
    for f in faces0[::97]:
        if land.face_normal[f][2] <= landmesh.WALKABLE_NORMAL_Z:
            continue
        cx, cy = (sum(pos[v][i] for v in land.faces[f]) / 3 for i in range(2))
        for ox, oy in ((7.0, 3.0), (-5.0, 9.0)):
            tx, ty = cx + ox, cy + oy
            under = [g for g, (x0, x1, y0, y1) in boxes.items()
                     if x0 <= tx <= x1 and y0 <= ty <= y1 and land.contains_xy(g, tx, ty)]
            if len(under) != 1 or land.face_normal[under[0]][2] <= landmesh.WALKABLE_NORMAL_Z:
                continue
            out["walks"] += 1
            out["arrived"] += land.walk(f, (cx, cy), (tx, ty)) == under[0]


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

    def turn(frame: float) -> tuple[float, float, float]:
        q = body.pose_at(0, frame)[1]
        ahead = objmesh.quaternion_rotate(q, (0.0, 1.0, 0.0))
        up = objmesh.quaternion_rotate(q, (0.0, 0.0, 1.0))
        return math.atan2(-ahead[0], ahead[1]), abs(ahead[2]), abs(up[0])

    sway = {}
    for gait, first, last in (("walk", 5, 13), ("run", 18, 34)):
        frames = [first + (last - first) * i / 240 for i in range(241)]
        turns = [turn(f) for f in frames]
        yaws = [y for y, _, _ in turns]
        travel = [body.pose_at(0, f)[0] for f in frames]
        sway[gait] = (max(abs(y) for y in yaws), statistics.fmean(yaws),
                      max(max(p, r) for _, p, r in turns),
                      max(t[1] for t in travel) - min(t[1] for t in travel),
                      max(abs(t[0]) for t in travel))
    ten = math.radians(10)
    peaks = (turn(22)[0], turn(30)[0])
    hero_unit = objects.load_unit(game / "UNITS" / "UNITS" / "HERO" / "tut1_p.dat")
    parents = hero_unit.parents()
    socket = next((c.attach_node for i, c in enumerate(hero_unit.components)
                   if parents[i] == 0 and c.is_external), -1)
    carried = (0 <= socket < len(body.nodes) and body.nodes[socket].name == "Base_TL"
               and body.nodes[socket].parent == 0)
    check("r_h_02: the body node yaws 10° each way a cycle, and carries the turret",
          all(abs(a - ten) < 1e-3 and abs(m) < 1e-3 and tilt < 1e-4
              for a, m, tilt, _, _ in sway.values())
          and abs(peaks[0] + ten) < 1e-3 and abs(peaks[1] - ten) < 1e-3
          and sway["run"][3] > 5.5 and 0.12 < sway["run"][4] < 0.14 and carried,
          f"node 0 (B_Dn) turns about z at most {math.degrees(sway['walk'][0]):.2f}° "
          f"over walk frames 5-13 and {math.degrees(sway['run'][0]):.2f}° over run "
          f"frames 18-34, {math.degrees(peaks[0]):+.1f}° at 22 and "
          f"{math.degrees(peaks[1]):+.1f}° at 30, mean {sway['run'][1]:+.4f} rad, "
          f"tilting {max(s[2] for s in sway.values()):.1e}; tut1_p's turret hangs on "
          f"node {socket}, Base_TL, a child of node 0.  The pose walk clears only the "
          f"root's translation (AniMesh.dll:0x10008d88) and the camera's points take "
          f"the node's model matrix (IAnimation slot 4 through Control.dll:0x1001b4f0), "
          f"so the eye swings 10° each way once a run cycle, {run / 14:.3f} s at 14 "
          f"m/s, while the root's {sway['run'][3]:.2f} of run travel and "
          f"±{sway['run'][4]:.2f} of side sway never reach the picture")

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
    def direction(s):
        lo, hi = s.velocity[0][1], s.velocity[1][1]
        return 0 if lo <= 0 <= hi else (1 if lo > 0 else -1)

    both_ways = file_keeps = live_keeps = 0
    for frm, s in enumerate(hero.states):
        if not (s.anchor and s.by_velocity):
            continue
        exits = [to for to in range(len(hero.states)) if to != frm
                 and hero.cost(to, frm) < control.NO_EDGE and direction(hero.states[to])]
        if len({direction(hero.states[to]) for to in exits}) < 2:
            continue
        both_ways += 1
        for weight, tally in ((hero.cost, "file"), (hero.live_cost, "live")):
            best = min(weight(to, frm) for to in exits)
            kept = all(direction(hero.states[to]) == direction(s)
                       for to in exits if weight(to, frm) == best)
            if tally == "file":
                file_keeps += kept
            else:
                live_keeps += kept
    check("r_h_02: scaled at load, the table keeps a gait going",
          both_ways == 8 and live_keeps == both_ways and file_keeps == 0
          and hero.live_cost(85, 78) == 5.0 and hero.live_cost(79, 78) == 17.0,
          f"on {live_keeps} of the {both_ways} velocity-driven anchors whose exits run "
          f"both ways, the cheapest exits keep the direction once each cost is times "
          f"1 + the gaps from the destination box's centre to the source's minimum "
          f"(Control.dll:0x10001790); control: by the file's costs alone {file_keeps} "
          f"do.  The forward run's 78 goes on to 85 at {hero.live_cost(85, 78):g}, "
          f"not to 79 at {hero.live_cost(79, 78):g}")
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

    # Header +0x14 is a settings id out of Effect.dll's own string table.
    settings = Counter(fx.gate for fx in library)
    named = sum(n for s, n in settings.items() if s in effects.EFFECT_SETTINGS)
    grouped = sum(n for s, n in settings.items() if s >> 8 in (0, 1, 2, 3))
    hero = library.get("hero_cannon")
    listed = ", ".join(f"{s:#x} {effects.EFFECT_SETTINGS.get(s)} {n}"
                       for s, n in sorted(settings.items()))
    check("FXID: header +0x14 is one of Effect.dll's twenty settings switches",
          named == len(library) and grouped == len(library) and hero is not None
          and hero.setting == "Gun fire"
          and all(effects.setting_enabled(s, 1) for s in settings),
          f"all {named} of {len(library)} effects name a switch "
          f"({len(settings)} of the 20 ids: {listed}); "
          f"the gate is table[id & 0xff] (Effect.dll:0x1000ec40) and preset 1, "
          f"RENDER_QUALITY=2, leaves every one on; the hero's guns are group 3's "
          f"'Gun fire'")

    # The test point and bit 8.
    points = Counter(fx.test_point for fx in library)
    flagged_kinds: Counter[int] = Counter()
    with_flag = with_flag_point = 0
    for fx in library:
        kinds = {e.kind for e in fx.emitters if e.flagged}
        flagged_kinds.update(e.kind for e in fx.emitters if e.flagged)
        if kinds:
            with_flag += 1
            with_flag_point += fx.test_point != (0.0, 0.0, 0.0)
    on_axis = sum(n for p, n in points.items() if p[1] == p[2] == 0.0 and p[0] >= 0.0)
    check("FXID: the tested point lies on +x, and bit 8 is only on drawn emitters",
          on_axis == len(library) and set(flagged_kinds) <= {3, 4, 7, 8, 9}
          and with_flag_point > with_flag // 2,
          f"header +0x24 is {', '.join(f'{p} on {n}' for p, n in points.most_common())} "
          f"-- the point Effect.dll:0x10007eb5 carries into the frame and rays from the "
          f"camera; bit 8 by type {dict(sorted(flagged_kinds.items()))}, never on a "
          f"light, a sound, a bolt or type 10; {with_flag_point} of the {with_flag} "
          f"effects with a flagged emitter lift the point off their origin")

    flags = {bit: [fx for fx in library if fx.flags & bit]
             for bit in (effects.FX_DETACH, effects.FX_TARGET_POINT,
                         effects.FX_HIDE_OCCLUDED, effects.FX_SECOND_PASS,
                         effects.FX_SHADE_FLAG)}
    detach = flags[effects.FX_DETACH]
    target = flags[effects.FX_TARGET_POINT]
    bolts = [fx for fx in library if any(e.kind == 5 for e in fx.emitters)]
    once = sum(fx.mode == effects.TIME_ONCE for fx in detach)
    check("FXID: flag 4 anchors once-through effects, flag 0x1000 every bolt",
          len(detach) > 400 and once > 0.95 * len(detach)
          and {fx.name for fx in target} == {fx.name for fx in bolts}
          and all({e.kind for e in fx.emitters} == {5} for fx in target),
          f"flag 4 (let go of the attach point, Effect.dll:0x10006324) on {len(detach)}, "
          f"{sum(fx.mode == effects.TIME_ONCE for fx in detach)} of them time mode 1; "
          f"flag 0x1000 (the manager's target point each tick, 0x10006349) on "
          f"{len(target)}, exactly the {len(bolts)} effects with a type-5 bolt, all "
          f"bolt-only; flag 0x400 on {len(flags[effects.FX_HIDE_OCCLUDED])}, 0x800 on "
          f"{len(flags[effects.FX_SECOND_PASS])}, 0x2000 on "
          f"{[fx.name for fx in flags[effects.FX_SHADE_FLAG]]}")

    # Type 1 is a light: its kind word and its attenuation terms.
    kinds = Counter()
    attenuation = Counter()
    ranges = []
    for fx in library:
        for e in fx.emitters:
            light = e.light
            if light is None:
                continue
            kinds[struct.unpack_from("<I", e.body, effects.LIGHT_KIND_AT)[0]] += 1
            attenuation[tuple(round(v, 3) for v in light.attenuation)] += 1
            ranges.append(light.range)
    cannon = next(e.light for e in hero.emitters if e.kind == 1) if hero else None
    check("FXID: a type-1 block is a Direct3D light's parameters",
          set(kinds) <= set(effects.LIGHT_KINDS) and sum(kinds.values()) == 618
          and all(a[0] == 0.0 for a in attenuation) and min(min(r) for r in ranges) >= 0.0
          and cannon is not None and cannon.range == (30.0, 3.0),
          f"+4, the light kind (Effect.dll:0x1000f649), is {dict(sorted(kinds.items()))}; "
          f"the attenuation terms +124..+132 are {dict(attenuation)} -- never a constant "
          f"term; every range is >= 0; the hero cannon's flash runs 30 to 3 in "
          f"colour {cannon.colour[0][:3] if cannon else None} to "
          f"{tuple(round(v, 2) for v in cannon.colour[1][:3]) if cannon else None}")

    # A bolt's segment and count, a stream's interval, a particle's fade.
    segments = Counter()
    counts = Counter()
    intervals = []
    lives = []
    fades = Counter()
    stream_fades = Counter()
    for fx in library:
        for e in fx.emitters:
            if e.kind == 5:
                counts[struct.unpack_from("<I", e.body, effects.BOLT_COUNT_AT)[0]] += 1
                segments[struct.unpack_from("<f", e.body, effects.BOLT_SEGMENT_AT)[0]] += 1
            elif e.kind == 8:
                intervals.append(e.emission_interval)
                lives.append(e.particle_lifetime)
                start, end, _power = e.fade
                stream_fades["down" if start > end else "up" if start < end else "flat"] += 1
            elif e.kind in (7, 10):
                start, end, _power = e.fade
                fades["down" if start > end else "up" if start < end else "flat"] += 1
    check("FXID: bolts run in segments, streams emit on an interval, particles fade",
          set(counts) == {20} and min(segments) >= 50.0
          and all(0.0 < a <= 1.0 and 0.0 < b <= 1.0 for a, b in intervals)
          and fades["down"] > 0.9 * sum(fades.values())
          and stream_fades["down"] > 0.9 * len(intervals),
          f"all {sum(counts.values())} bolts draw at most {sorted(counts)} sprites, one "
          f"per {dict(sorted(segments.items()))} units of their length "
          f"(Effect.dll:0x10002c53); the {len(intervals)} streams emit every "
          f"{min(min(i) for i in intervals):g} to {max(max(i) for i in intervals):g} s "
          f"(0x10011a6c) and their particles live {min(min(v) for v in lives):.2g} to "
          f"{max(max(v) for v in lives):.2g} s, a ring's worth (0x1001209e); fade values "
          f"run {dict(fades)} on bursts (+8/+12/+16) and {dict(stream_fades)} on "
          f"streams (+4/+8/+12)")

    # Time mode 4 reads a node's value, and the hero turret names the nodes its
    # guns' barrels and arms animate.
    turrets = NResArchive.open(game / "turrets.rlb")
    tur = control.parse(turrets.read_name("o_tur_ht_02.ctl"), names)
    tmesh = objmesh.parse(turrets.read_name("o_tur_ha_02.msh"), "o_tur_ha_02.msh")
    ids = {r.args[3]: r.resource.member.lower() for r in tur.group(control.ENTRY_LOAD)
           if r.action == control.ACT_EFFECT_POINTS}
    timed = {ids[r.args[0]]: r.args[1] for r in tur.group(control.ENTRY_LOAD)
             if r.action == control.ACT_EFFECT_TIME_POINT}
    animated = {ch.node: i for i, ch in enumerate(tur.channels)}
    guns = {i for c in tur.components if c.type_id == control.GUN_TYPE for i in c.entries}
    arms = {i for c in tur.components if c.type_id == control.ARM_TYPE for i in c.entries}
    by_gun = {name: tmesh.nodes[node].name for name, node in timed.items()
              if animated.get(node) in guns}
    by_arm = sorted(name for name, node in timed.items() if animated.get(node) in arms)
    as_channels = sum(1 for n in timed.values()
                      if n < len(tur.channels) and tur.channels[n].node == n)
    check("o_tur_ht_02: time mode 4 reads the nodes the barrels and arms animate",
          len(timed) == 7 and all(node in animated for node in timed.values())
          and by_gun == {"hero_cannon": "Gun02_m1o1", "hero_prifle": "Plz02_m1o1",
                         "hero_redlaser": "Lz02_m1o1"}
          and by_arm == ["hero_cannon_sfx", "hero_missile_sfx", "hero_prifle_sfx",
                         "hero_redlaser_sfx"]
          and all(library.get(n).mode == effects.TIME_POINT for n in timed),
          f"action 14's v5 is a mesh node (AniMesh.dll:0x10005600 returns node +0x114, "
          f"the value the channel update sets, Control.dll:0x10021c97): {timed}; the gun "
          f"effects follow their gun component's barrel channel on {by_gun}, the four "
          f"sounds an arm channel's node; control: read as channel numbers, "
          f"{as_channels} of 7 name a channel on that node")

    # A unit answers interface 0xd with its own material manager, so a round
    # that strikes it plays the slot of the struck batch's material class.
    mats = materials.MaterialLibrary(game / "Material.lib")
    things = objects.ObjectLibrary(game / "objects.rlb")
    opened: dict[str, NResArchive] = {}
    classes: Counter[int] = Counter()
    units = 0
    for record in things.by_tag("BTLU"):
        ref = record.textures
        if ref is None or not ref.library:
            continue
        opened.setdefault(ref.library, NResArchive.open(game / ref.library))
        units += 1
        for name in objmesh.read_wea(opened[ref.library].read_name(ref.member)):
            m = mats.get(name)
            if m is not None:
                classes[m.surface] += 1
    mission = {}
    for stem in ("r_h_01", "r_h_02", "r_h_03"):
        ref = things.get(stem).textures
        opened.setdefault(ref.library, NResArchive.open(game / ref.library))
        mission[stem] = sorted({mats.get(n).surface for n in
                                objmesh.read_wea(opened[ref.library].read_name(ref.member))})
    total = sum(classes.values())
    check("objects.rlb: a unit's skins are the machine surface, so a hit plays 'mt'",
          units == 63 and classes[5] > 0.8 * total
          and all(v == [5] for v in mission.values()),
          f"the {total} wear materials of the {units} BTLU units are class "
          f"{dict(classes.most_common())} (255 unset: slot 0); Mission 01's dummies "
          f"and hero {mission}.  A unit's QueryInterface answers 0xd from its own "
          f"table (AniMesh.dll:0x1000358f, LoadMatManager), CWorld::GetWorldFace takes "
          f"the struck batch's material (AniMesh.dll:0x100135c8), and Control.dll:"
          f"0x100114fd reads its class, so slot 6, {effects.SURFACE_TAGS[5]!r}, plays")


def _planted_steps(c: control.Controller, m, points, order: list[int]) -> tuple[list, float]:
    """The footsteps a state sequence sounds, and the closest a foot comes to 0.1.

    A contact is planted in a state when its point, at pair B's last frame, lies
    within ``PLANTED_WITHIN`` of its height in the rest pose (``0x1001a2d5``); a
    step is a planted state after one that was not (``0x1001b08f``).
    """
    def height(point: int, frame: float) -> float:
        p = points[point]
        node = p.nodes[0]
        pose = m.blended_pose(node, frame, frame, 1.0)
        parent = m.nodes[node].parent
        while parent != objmesh.NO_PARENT and parent < len(m.nodes):
            pose = objmesh.compose(m.blended_pose(parent, frame, frame, 1.0), pose)
            parent = m.nodes[parent].parent
        return objmesh.apply(pose, p.position)[2]

    def rest_height(point: int) -> float:
        return objmesh.apply(m.world_pose(points[point].nodes[0]), points[point].position)[2]

    rest = {k.point: rest_height(k.point) for k in c.states[order[0]].contacts}
    was: dict[int, bool | None] = dict.fromkeys(rest)
    steps, nearest = [], math.inf
    for s in order + order[:1]:
        for k in c.states[s].contacts:
            gap = abs(height(k.point, c.states[s].pair_b[1]) - rest[k.point])
            nearest = min(nearest, abs(gap - control.PLANTED_WITHIN))
            planted = gap < control.PLANTED_WITHIN
            if planted and was[k.point] is False:
                steps.append((s, k.point))
            was[k.point] = planted
    return steps, nearest


def check_sounds(check, game: Path) -> None:
    """Sound emitters, the gun arms' sounds and the hero's footsteps."""
    library = effects.EffectLibrary(game / "effects.rlb")
    modes: Counter[int] = Counter()
    for fx in library:
        for e in fx.emitters:
            if e.is_sound:
                modes[struct.unpack_from("<I", e.body, effects.SOUND_MODE_AT)[0]] += 1
    sfx = [library.get(f"hero_{g}_sfx") for g in ("cannon", "prifle", "redlaser", "missile")]
    breath = library.get("hero_breath")
    breath_sound = next((e for e in breath.emitters if e.is_sound), None) if breath else None
    steps = [fx for fx in library if fx.name.lower().startswith("step_")]
    step_shape = Counter((fx.mode, fx.duration, len(fx.emitters), fx.emitters[0].is_sound,
                          fx.emitters[0].sound_loops, fx.emitters[0].window) for fx in steps)
    arms_one_shot = all(fx and fx.mode == effects.TIME_POINT and len(fx.emitters) == 1
                        and fx.emitters[0].is_sound and not fx.emitters[0].sound_loops
                        and abs(fx.emitters[0].window[0] - 0.15) < 1e-6 for fx in sfx)
    check("FXID: a sound is a one-shot at +8, or a loop over its window when +4 is 2 or 3",
          sum(modes.values()) == 517 and modes[0] == 411 and modes[2] == 106
          and set(modes) == {0, 2} and arms_one_shot and len(steps) == 9
          and all(not fx.emitters[0].sound_loops for fx in steps)
          and breath_sound is not None and breath_sound.sound_loops
          and breath_sound.audible_range == (1.0, 4.0),
          f"+4 over the {sum(modes.values())} sound emitters: {dict(sorted(modes.items()))} "
          f"(Effect.dll:0x10012d3e keeps 2 or 3 as the loop byte); the four hero arm sounds "
          f"are time-mode-4 one-shots at 0.15: {arms_one_shot}; all {len(steps)} step_* "
          f"effects are one-shots; hero_breath's H_breath.wav loops, audible "
          f"{breath_sound.audible_range if breath_sound else None}")
    check("FXID: every step_* effect is 0.5 s of one one-shot sound with its trigger at 0.1",
          len(step_shape) == 1 and next(iter(step_shape))[:5] == (effects.TIME_MANUAL, 0.5, 1, True,
                                                                  False)
          and all(abs(a - b) < 1e-6
                  for a, b in zip(next(iter(step_shape))[5], (0.1, 0.9), strict=True)),
          f"{dict(step_shape)} (mode, duration, emitters, sound, loops, window) over "
          f"{sorted(fx.name for fx in steps)}")

    names = frozenset(p.name.lower() for p in all_archives(game))
    turrets = NResArchive.open(game / "turrets.rlb")
    turret = control.parse(turrets.read_name("o_tur_ht_02.ctl"), names)
    arm_nodes = {11, 7, 19, 15}
    arm_channels = [ch for ch in turret.channels if ch.node in arm_nodes and ch.first == 42.0]
    check(".ctl: the hero's arm channels carry no flag, so their nodes' values run 0 to 1",
          len(arm_channels) == 4 and all(ch.flags == 0 and ch.last == 48.0 for ch in arm_channels),
          f"channels on nodes {sorted(ch.node for ch in arm_channels)}, frames 42-48, flags "
          f"{[ch.flags for ch in arm_channels]}: the nodes the four _sfx effects take their "
          f"time from (docs/29)")

    cfg = game / "ui" / "game_resources.cfg"
    bound = [k for d in resources.descriptors(cfg) for k in d.bindings] if cfg.is_file() else []
    weapon_sel = [k for k in bound if "WEAP" in k.upper() and "SEL" in k.upper()]
    check("ui/game_resources.cfg binds no weapon-selection sound",
          bound and not weapon_sel and "TARGET_SELECTED" in bound,
          f"{len(bound)} bindings; weapon and select together: {weapon_sel}; weapon voices "
          f"{[k for k in bound if 'WEAP' in k.upper()]}")

    # Action 3's v4 is a node; the hero's step groups.
    node_ok = past_cpt = total = 0
    contact_groups: Counter[tuple] = Counter()
    for arcname in ("bases.rlb", "animals.rlb", "static.rlb", "turrets.rlb", "fortif.rlb",
                    "parts.rlb"):
        path = game / arcname
        if not path.is_file():
            continue
        arc = NResArchive.open(path)
        entries = {e.name.lower(): e for e in arc}
        for name, entry in entries.items():
            if not name.endswith(".ctl") or name[:-4] + ".msh" not in entries:
                continue
            try:
                c = control.parse(arc.read(entry), names)
            except control.ControlFormatError:
                continue
            for s in c.states:
                for k in s.contacts:
                    if k.group >= 0:
                        contact_groups[tuple(sorted({(r.action, r.args[1]) for r in c.references
                                                     if r.group == k.group}))] += 1
            points_of = [r for r in c.references if r.action == control.ACT_EFFECT_POINT]
            if not points_of:
                continue
            m = objmesh.parse(arc.read(entries[name[:-4] + ".msh"]), name)
            cpt = entries.get(name[:-4] + ".cpt")
            points = objmesh.parse_control_points(arc.read(cpt)) if cpt else []
            for r in points_of:
                total += 1
                node_ok += 0 <= r.args[0] < len(m.nodes)
                past_cpt += not 0 <= r.args[0] < len(points)
    check(".ctl: action 3 places its effect on a node, not a control point",
          total == 183 and node_ok == total and past_cpt == 18
          and contact_groups[((control.ACT_EFFECT_START, 1),)] == 800
          and contact_groups[((control.ACT_EFFECT_START, 0), (control.ACT_EFFECT_START, 1),
                              (control.ACT_EFFECT_RESTART, 0))] == 368,
          f"{node_ok} of {total} action-3 records on controllers with a same-stem mesh name one "
          f"of its nodes; {past_cpt} lie past the same-stem .cpt (Control.dll:0x100029bb rebases "
          f"v4 by the part's first node, as action 14 does); contact groups: "
          f"{contact_groups.most_common(3)}")

    bases = NResArchive.open(game / "bases.rlb")
    hero = control.parse(bases.read_name("r_h_02.ctl"), names)
    mesh = objmesh.parse(bases.read_name("R_H_02.msh"), "r_h_02")
    points = objmesh.parse_control_points(bases.read_name("R_H_02.CPT"))
    load = [r for r in hero.references if r.group == hero.groups[control.ENTRY_LOAD]
            and r.action == control.ACT_EFFECT_POINT
            and r.resource.member.lower().startswith("step_")]
    feet = {points[k.point].nodes[0]: points[k.point].name for k in hero.states[0].contacts}
    placed = {r.args[0] for r in load}
    run, run_near = _planted_steps(hero, mesh, points, list(range(85, 91)) + list(range(73, 79)))
    walk, walk_near = _planted_steps(hero, mesh, points, list(range(33, 45)) + list(range(9, 21)))
    check("r_h_02: the steps sit on the feet's nodes, and land three times a run and a walk cycle",
          len(load) == 10 and placed == set(feet) == {4, 8}
          and sorted(run) == [(78, 0), (78, 2), (90, 0)]
          and sorted(walk) == [(9, 2), (12, 2), (18, 0)],
          f"the load group's {len(load)} step effects sit on nodes {sorted(placed)}, which "
          f"{feet} sit on; planted against the rest pose's height, the run (states 85-90, "
          f"73-78) steps {run} and the walk (33-44, 9-20) {walk}, (state, contact); the "
          f"nearest a foot comes to the 0.1 line: {run_near:.4f} at a run, {walk_near:.4f} "
          f"at a walk")


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

    weapon = NResArchive.open(game / "weapon.rlb")
    library = effects.EffectLibrary(game / "effects.rlb")
    ends = {}
    for name in ("bb_h_01", "bp_h_01", "bl_h_01", "bm_h_01"):
        c = rounds[f"{name}.ctl"]
        end = next(r for r in c.group(control.ENTRY_RANGE) if r.action == control.ACT_EXPLODE_NODE)
        blast = effects.parse_explosion(weapon.read_name(end.resource.member), end.resource.member)
        played = library.get(blast.effect.member) if blast.effect else None
        ends[name] = (c.triples[control.TRIPLE_TOP_SPEED][1], c.bounds[0],
                      end.resource.member.lower(), blast.kind, blast.damage, blast.radius,
                      blast.placement, played)
    puffs = [ends[n][7] for n in ("bb_h_01", "bp_h_01", "bl_h_01")]
    missile = ends["bm_h_01"][7]
    check("the hero's rounds: 500, 150, 1000 and 350 m, then a puff or a blast",
          [e[:2] for e in ends.values()] == [(350, 500), (150, 150), (10000, 1000), (70, 350)]
          and [e[2] for e in ends.values()] == ["bb_h_01_end.exp", "bp_h_01_end.exp",
                                                 "bl_h_01_end.exp", "bm_h_01r.exp"]
          and all(e[3:7] == (effects.HIT_DIRECT, 1.0, 1.0, 0) for e in list(ends.values())[:3])
          and ends["bm_h_01"][3:7] == (effects.HIT_AREA, 200.0, 10.0, 0)
          and all(p is not None and p.duration == 1.5 and p.setting == "Smoke"
                  and [m.kind for m in p.emitters] == [7, 7, 4, 4]
                  and p.materials == ["smoke_g", "smoke_g_add", "glow_eng", "glow_eng"]
                  for p in puffs)
          and missile is not None and missile.duration == 3.0 and missile.sounds,
          "cannon, plasma, laser, missile: " + "; ".join(
              f"{n} {e[0]:g} m/s for {e[1]:g} m, then {e[2]} (kind {e[3]}, {e[4]:g} in "
              f"{e[5]:g} m) playing {e[7].name if e[7] else None}"
              for n, e in ends.items())
          + f".  The three direct rounds' puffs are {puffs[0].duration:g} s of "
          f"{', '.join(puffs[0].materials)} under the {puffs[0].setting} switch.  A range "
          f"is the path flown: each tick the length moved comes off +0x4c8 "
          f"(Control.dll:0x1000cfc6, 0x1000d070) and a longer move is cut to what is "
          f"left; the hit's damage has no distance in it")

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

    check_action_conditions(check, game)


def check_action_conditions(check, game: Path) -> None:
    """Section 5's conditions and runs, the hero's footsteps, the critical-damage entries."""
    names = frozenset(p.name.lower() for p in all_archives(game))
    modes: Counter[str] = Counter()
    bits: set[int] = set()
    inverted_inside = True
    opened = closed = stray = 0
    hero = None
    entries: dict[int, Counter[tuple[int, ...]]] = {control.ENTRY_CRITICAL: Counter(),
                                                    control.ENTRY_RECOVERED: Counter()}
    paired = set_six = unused = 0
    effects_on: Counter[str] = Counter()
    for path in all_archives(game):
        archive = NResArchive.open(path)
        for entry in archive:
            if not entry.tag.upper().startswith("CTL"):
                continue
            try:
                c = control.parse(archive.read(entry), names)
            except control.ControlFormatError:
                continue
            if entry.name.lower() == "r_h_02.ctl":
                hero = c
            for r in c.references:
                if r.flags & control.REF_ANY:
                    modes["any"] += 1
                if r.flags & control.REF_ALL:
                    modes["all"] += 1
                if r.flags & (control.REF_ANY | control.REF_ALL):
                    bits |= {i for i in range(control.CONDITIONS) if r.mask >> i & 1}
                    inverted_inside &= r.inverted & ~r.mask == 0
            for g in range(c.counts[4]):
                inside = False
                for r in (r for r in c.references if r.group == g):
                    if r.flags & control.REF_OPEN and not inside:
                        inside, opened = True, opened + 1
                    if r.flags & control.REF_ELSE:
                        closed += inside
                        stray += not inside
                        inside = False
                stray += inside
            if not c.groups:
                continue
            unused += any(c.groups[e] != control.NO_GROUP for e in (1, 5, 8, 9))
            six = c.groups[control.ENTRY_CRITICAL] != control.NO_GROUP
            seven = c.groups[control.ENTRY_RECOVERED] != control.NO_GROUP
            set_six += six
            paired += six and seven
            ids = {r.args[3]: r.resource.member for r in c.references
                   if r.action in (control.ACT_EFFECT_POINT, control.ACT_EFFECT_POINTS)}
            for e in entries:
                if c.groups[e] != control.NO_GROUP:
                    grp = c.group(e)
                    entries[e][tuple(sorted({r.action for r in grp}))] += 1
                    if e == control.ENTRY_CRITICAL:
                        on = [r.args[0] for r in grp]
                        off = [r.args[0] for r in c.group(control.ENTRY_RECOVERED)]
                        paired -= six and seven and sorted(on) != sorted(off)
                        effects_on.update(ids.get(i, "?") for i in on)
    check(".ctl: a section-5 record's ints 1 and 2 are a mask over the condition bytes",
          modes == Counter(any=37) and bits <= set(range(11)) | {14, 15} and inverted_inside
          and opened == closed == 8 and stray == 0,
          f"{dict(modes)} records test bytes {sorted(bits)} (surface ids 0-10, 7 the liquid "
          f"bed; Control.dll:0x100022c0), every inversion inside its mask; {opened} runs "
          f"open with 0x80000000 and {closed} close with 0x10000000, none left open")

    steps = []
    if hero is not None:
        ids = {r.args[3]: r.resource.member.lower() for r in hero.references
               if r.action in (control.ACT_EFFECT_POINT, control.ACT_EFFECT_POINTS)}
        feet = {k.group for s in hero.states for k in s.contacts} - {control.NO_GROUP}
        for r in hero.references:
            if r.group in feet and r.mask and not r.flags & control.REF_ELSE:
                steps.append((r.mask.bit_length() - 1, ids.get(r.args[0], "")))
    tags = effects.SURFACE_TAGS

    def matching(shift: int) -> int:
        return sum(1 for bit, fx in steps if 0 <= bit + shift < len(tags)
                   and fx.startswith("step_h") and fx[6:7] == tags[bit + shift][0])
    control_hits = max(matching(-1), matching(1))
    check("r_h_02: the hero's footstep group picks its effect by surface id",
          len(steps) == 12 and matching(0) == 8 and control_hits <= 2,
          f"each foot's group (a contact's +8) starts {sorted(set(steps))}; "
          f"on {matching(0)} of {len(steps)} the effect's letter is the surface's tag "
          f"({' '.join(tags)}): metal, stone, grass, al.  Control: one surface either way, "
          f"{control_hits}")

    check(".ctl: block entries 6 and 7 switch an effect on and off at critical damage",
          set_six == 32 and paired == 32 and unused == 0
          and set(entries[control.ENTRY_CRITICAL]) == {(control.ACT_EFFECT_ON,)}
          and set(entries[control.ENTRY_RECOVERED]) == {(control.ACT_EFFECT_OFF,)},
          f"{set_six} controllers set entry 6 and {paired} of them turn the same effects "
          f"off in entry 7 (Control.dll:0x10012bfb, 0x10012b12): {dict(effects_on)}; "
          f"{unused} set entry 1, 5, 8 or 9")


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

    # -- clan relations: the file's words are the SuperAI's ------------------
    words: Counter[tuple[str, int]] = Counter()
    pairs: Counter[tuple[int, int]] = Counter()
    unmatched = overridden = 0
    missions = [mission.load(p) for p in sorted(game.glob("MISSIONS/**/data.tma"))]
    for m in missions:
        by_name = {c.name.lower(): c for c in m.clans}
        for clan in m.clans:
            for other, word in clan.relations.items():
                target = by_name.get(other.lower())
                unmatched += target is None
                if target is clan:
                    words[("self", word)] += 1
                elif target is not None:
                    words[("other", word)] += 1
                    # MisLoad.dll forces 1 to and from a neutral clan
                    overridden += (mission.CLAN_NEUTRAL in (clan.type, target.type)
                                   and word != mission.RELATION_NEUTRAL)
        matrix = m.relations()
        for i in range(len(m.clans)):
            for j in range(i + 1, len(m.clans)):
                pairs[(matrix[i][j], matrix[j][i])] += 1
    others = {w: n for (k, w), n in words.items() if k == "other"}
    selves = {w for (k, w) in words if k == "self"}
    check("data.tma: a relation word is 0, 1 or 2, and the clans agree pairwise",
          set(others) == {0, 1, 2} and selves == {1}
          and not unmatched and not overridden
          and all(a == b for a, b in pairs) and len(pairs) == 3,
          f"towards other clans {dict(sorted(others.items()))}, towards itself 1 on "
          f"{words[('self', 1)]} (the loader makes it 2, MisLoad.dll:0x100015b0); "
          f"{unmatched} name no clan and {overridden} words to or from a neutral "
          f"clan differ from the 1 the loader forces.  Across {len(missions)} "
          f"missions every pair of clans holds the same word each way: "
          + ", ".join(f"{a} on {n}" for (a, _b), n in sorted(pairs.items()))
          + " (0 hostile, 1 neutral, 2 allied; ai.dll:0x10005e80)")


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

    # A round's swept sphere is its mesh header's sphere (AniMesh.dll:0x1000a891,
    # 0x10009510): one part, scale 1.
    radii = {}
    for record in library.by_tag("BULL"):
        m = mesh_of(record)
        if m is not None:
            radii[record.name.lower()] = m.collision_radius()
    positive = [r for r in radii.values() if r > 0]
    hero_radii = ", ".join(f"{n} {radii[n]:.3g}" for n in HERO_ROUNDS if n in radii)
    check("weapon.rlb: every round's mesh header carries the sphere it sweeps",
          radii and len(positive) == len(radii),
          f"{len(positive)}/{len(radii)} BULL rounds with a mesh have a header sphere "
          f"of radius {min(positive):.3g}-{max(positive):.3g}; the hero's: {hero_radii}")


#: Mission 01, the two halves of its bridge, and what the ground contact takes as ground.
MISSION_01_DATA = "MISSIONS/CAMPAIGN/CAMPAIGN.00/Mission.01/data.tma"
BRIDGE_HALVES = (23, 24)
WALKABLE_Z = 0.173648


def _world_faces(asm, obj):
    """An object's level-0 triangles in the world: (corners, face normal, flags)."""
    half = obj.rotation / 2.0
    place = (tuple(obj.position), (math.cos(half), 0.0, 0.0, math.sin(half)))
    out = []
    for part in asm.parts(obj.kind, obj.path):
        mesh = asm.mesh(part.ref)
        if mesh is None:
            continue
        for n, node in enumerate(mesh.nodes):
            s = node.hit_slot(0)
            if s is None:
                continue
            pose = objmesh.compose(place, objmesh.compose(part.pose, mesh.world_pose(n)))
            turn = ((0.0, 0.0, 0.0), pose[1])
            slot = mesh.slots[s]
            for f in range(slot.first_triangle, slot.first_triangle + slot.triangle_count):
                out.append(([objmesh.apply(pose, mesh.positions[i]) for i in mesh.triangles[f]],
                            objmesh.apply(turn, mesh.face_normal[f]), mesh.face_flags[f]))
    return out


def _deck_z(faces, x, y):
    """The highest walkable face holding (x, y), or None."""
    best = None
    for (a, b, c), normal, _ in faces:
        if normal[2] <= WALKABLE_Z:
            continue
        d = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])
        if abs(d) < 1e-9:
            continue
        u = ((b[0] - x) * (c[1] - y) - (c[0] - x) * (b[1] - y)) / d
        v = ((c[0] - x) * (a[1] - y) - (a[0] - x) * (c[1] - y)) / d
        if min(u, v, 1 - u - v) >= -1e-6:
            z = u * a[2] + v * b[2] + (1 - u - v) * c[2]
            best = z if best is None else max(best, z)
    return best


def _land_z(land, x, y):
    """Land.msh under (x, y): the highest dry face and the highest water face."""
    if getattr(land, "_grid", None) is None:
        land._build_index()
    (sx, sy), (ox, oy) = land._grid_step, land._grid_origin
    dry = wet = None
    for fi in land._grid.get((int((x - ox) / sx), int((y - oy) / sy)), ()):
        a, b, c = (land.positions[i] for i in land.faces[fi])
        den = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1])
        if abs(den) < 1e-12:
            continue
        l1 = ((b[1] - c[1]) * (x - c[0]) + (c[0] - b[0]) * (y - c[1])) / den
        l2 = ((c[1] - a[1]) * (x - c[0]) + (a[0] - c[0]) * (y - c[1])) / den
        if min(l1, l2, 1 - l1 - l2) < -1e-6:
            continue
        z = l1 * a[2] + l2 * b[2] + (1 - l1 - l2) * c[2]
        if land.is_water(fi):
            wet = z if wet is None else max(wet, z)
        else:
            dry = z if dry is None else max(dry, z)
    return dry, wet


def check_lake_and_buoys(check, game: Path) -> None:
    """Mission 01: how much life the hero has for a lake bed, and the buoys' faces."""
    m = mission.load(game / MISSION_01_DATA)
    asm = assembly.Assembly(game)
    hero = next(o for o in m.objects if "\\hero\\" in o.path.lower())
    lives = {}
    for component in objects.load_unit(asm.unit_file(hero.path)).components:
        record = asm.library.get(component.ref.member)
        ref = record.damage if record else None
        if ref is None:
            continue
        rows = objects.parse_damage(asm.archive(ref.library).read_name(ref.member), ref.member)
        lives[component.ref.member.lower()] = (len(rows), sum(r.durability for r in rows))
    models = {k: v for k, v in lives.items() if v[0] > 1}
    fitted = [v for k, v in lives.items() if v[0] == 1]
    total = sum(life for _, life in lives.values())
    bed = materials.MaterialLibrary(game / "Material.lib").get("WATER_BOT")
    updates = math.ceil(total / (bed.damage_rate * 0.25))
    check("Mission 01: a lake bed takes the hero's life on the third life update",
          models == {"r_h_02": (10, 2881), "e_tur_ht_02": (36, 4474)}
          and len(fitted) == 7 and all(v == (1, 1) for v in fitted) and total == 7362
          and bed.damage_rate == 10000 and bed.surface == 1 and updates == 3
          and math.ceil((total - 7) / (bed.damage_rate * 0.25)) == 3,
          f"hero models {models}, {len(fitted)} fitted parts of 1 hit point; {total:g} in all; "
          f"WATER_BOT surface {bed.surface}, rate {bed.damage_rate:g} a second, so "
          f"{bed.damage_rate * 0.25:g} per 250 ms update and {updates} updates to die, "
          f"with or without the fitted parts")

    buoys = [(i, o.kind, o.placed_scale) for i, o in enumerate(m.objects)
             if o.path.lower() == "s_tree_29"]
    normals, count = Counter(), 0
    for part in asm.parts(buoys[0][1], "s_tree_29"):
        mesh = asm.mesh(part.ref)
        for node in mesh.nodes:
            s = node.hit_slot(0)
            if s is None:
                continue
            slot = mesh.slots[s]
            for t in range(slot.first_triangle, slot.first_triangle + slot.triangle_count):
                count += 1
                normals[round(mesh.face_normal[t][2], 2)] += 1
    check("Mission 01: five buoys, 36 vertical faces and 12 at normal z 0.49",
          [i for i, _, _ in buoys] == [25, 26, 27, 29, 30]
          and all(kind == 2 and scale == 1 for _, kind, scale in buoys)
          and count == 178 and normals[0.49] == 12 and normals[0.0] == 36,
          f"s_tree_29 at objects {[i for i, _, _ in buoys]}, kinds and scales "
          f"{sorted({(k, s) for _, k, s in buoys})}; {count} level-0 triangles, "
          f"normal z {sorted(normals.items())}; control: the cone's horizontal part "
          f"{math.sqrt(1 - 0.49 ** 2):.2f} leaves a flattened push "
          f"{1 - math.sqrt(1 - 0.49 ** 2):.0%} of a step short")


def check_collision(check, game: Path) -> None:
    """Collision and the ground on Mission 01: the bridge's deck, obstacles' spheres and flags."""
    m = mission.load(game / MISSION_01_DATA)
    land = landmesh.load(game / "DATA/MAPS/Tut_1/Land.msh")
    asm = assembly.Assembly(game)

    profiles = {}
    for i in BRIDGE_HALVES:
        obj = m.objects[i]
        faces = _world_faces(asm, obj)
        ang = obj.rotation + math.pi / 2
        rows = []
        for t in range(-44, 80, 4):
            x, y = obj.position[0] + math.cos(ang) * t, obj.position[1] + math.sin(ang) * t
            rows.append((t, _deck_z(faces, x, y), *_land_z(land, x, y), y))
        profiles[i] = (obj, faces, rows)
    ok_halves, notes = True, []
    for i, (obj, faces, rows) in profiles.items():
        on = [r for r in rows if r[1] is not None]
        first, origin = on[0], next(r for r in rows if r[0] == 0)
        top = [r for r in rows if r[0] >= 24]
        walk = sum(normal[2] > WALKABLE_Z for _, normal, _ in faces)
        dry_min = min(r[2] for r in rows if r[2] is not None)
        waters = {round(r[3], 2) for r in rows if r[3] is not None}
        ok_halves &= (obj.path.lower().endswith("m_bridge.dat")
                      and obj.properties["Type"].value & 0xFFFFFFFF == 0x80001000
                      and len(faces) == 230 and walk == 130
                      and first[0] == -36 and abs(first[1] - 0.32) < 0.01 and first[2] == 0.0
                      and abs(origin[1] - 9.39) < 0.01
                      and all(abs(r[1] - 11.52) < 0.01 for r in top)
                      and dry_min < -19 and waters == {-1.73})
        notes.append(f"object {i}: deck from {first[1]:.2f} over dry {first[2]:.2f} "
                     f"at t {first[0]}, "
                     f"{origin[1]:.2f} at the origin, {top[0][1]:.2f} from t 24; "
                     f"{walk} of {len(faces)} faces walkable; bed down to {dry_min:.2f}, "
                     f"water {waters}")
    meet = [profiles[i][2][-1] for i in BRIDGE_HALVES]
    samples = [r for _, _, rows in profiles.values() for r in rows]
    wet = [r for r in samples if r[3] is not None]
    bed = min(r[2] for r in samples if r[2] is not None)
    check("Mission 01: the bridge's deck meets the bank and the halves meet level",
          ok_halves and all(abs(r[1] - 11.52) < 0.01 for r in meet)
          and abs(meet[0][4] - meet[1][4]) < 5 and abs(bed + 28.72) < 0.05
          and wet and all(r[2] < r[3] for r in wet),
          "; ".join(notes) + f"; the halves' ends at y {meet[0][4]:.1f} and {meet[1][4]:.1f}, "
          f"both 11.52; the bed's lowest {bed:.2f}; control: on all {len(wet)} samples over "
          f"water the landscape's dry face lies below it, so only the deck crosses")

    flags, radii, triangles = {}, {}, {}
    for obj in m.objects:
        spheres, count = [], 0
        for part in asm.parts(obj.kind, obj.path):
            mesh = asm.mesh(part.ref)
            if mesh is None:
                continue
            per = Counter()
            for node in mesh.nodes:
                s = node.hit_slot(0)
                if s is None:
                    continue
                slot = mesh.slots[s]
                count += slot.triangle_count
                end = slot.first_triangle + slot.triangle_count
                per.update(mesh.face_flags[slot.first_triangle:end])
            flags[mesh.name] = dict(sorted(per.items()))
            if mesh.volume:
                spheres.append((objmesh.apply(part.pose, mesh.volume.centre), mesh.volume.radius))
        if not spheres:
            continue
        weight = sum(r for _, r in spheres)
        centre = tuple(sum(c[k] * r for c, r in spheres) / weight for k in range(3))
        radius = max(math.dist(c, centre) + r for c, r in spheres) * obj.placed_scale
        stem = obj.path.split("\\")[-1].lower()
        radii.setdefault(stem, set()).add(round(radius, 2))
        triangles[stem] = count
    flagged = {k: v for k, v in flags.items() if set(v) != {0}}
    check("Mission 01: only the big tree's leaves and the bridge carry level-0 face flags",
          flagged == {"s_tree_0_04.msh": {0: 212, 4: 192},
                      "fr_m_brige.msh": {0: 196, 2: 16, 4: 18}},
          f"{flagged}; every other level-0 triangle of the mission's {len(flags)} meshes is 0, "
          f"so a leaf, flagged 4, is the only face a walker's segment passes "
          f"(Control.dll:0x1001db5f)")
    hero = radii.get("tut1_p.dat")
    others = [r for k, v in radii.items() if k != "tut1_p.dat" for r in v]
    want = {"tut1_p.dat": {2.18}, "l_targ.dat": {5.22}, "m_targ.dat": {15.45},
            "tut1_e1.dat": {2.72}, "helic.dat": {2.45}, "tut1_mf1.dat": {5.98},
            "s_tree_29": {3.43}, "m_bridge.dat": {61.92}}
    check("Mission 01: the hero's collision sphere is the smallest, so it is always the mover",
          all(radii.get(k) == v for k, v in want.items()) and hero and min(others) > max(hero)
          and triangles.get("tut1_p.dat") == 688
          and {r for k in ("s_stone_05", "s_stone_06", "s_stone_07", "s_stone_10")
               for r in radii[k]}
          >= {41.17, 78.89},
          f"radii {dict(sorted((k, sorted(v)) for k, v in radii.items()))}; the parts' header "
          f"spheres joined as AniMesh.dll:0x10009510 joins them, times the placement's scale; "
          f"the larger sphere is the obstacle (Control.dll:0x1001d647)")


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

    # ---- which sector is the front: a model's +y.  The running gear's names put
    # a front (F) or back (B) letter beside their side letter.
    front = back = wrong = 0
    for libname in ("bases.rlb", "animals.rlb"):
        archive = NResArchive.open(game / libname)
        meshes = {e.name.lower()[:-4]: e for e in archive if e.tag.upper().startswith("MESH")}
        for entry in archive:
            stem = entry.name.lower()[:-4]
            if not entry.name.lower().endswith(".ndp") or stem not in meshes:
                continue
            rows = objects.parse_damage(archive.read(entry), entry.name)
            gear = [(i, r.flags) for i, r in enumerate(rows)
                    if r.flags in (objects.LEFT_GEAR, objects.RIGHT_GEAR)]
            if not gear:
                continue
            model = objmesh.parse(archive.read(meshes[stem]), meshes[stem].name)
            for i, flags in gear:
                word = model.nodes[i].name.split("_")[0]
                side = "L" if flags == objects.LEFT_GEAR else "R"
                at = word.rfind(side)
                beside = {word[at - 1] if at > 0 else "", word[at + 1: at + 2]}
                y = model.world_pose(i)[0][1]
                if beside & {"F"} and not beside & {"B"}:
                    front += y > 0
                    wrong += y <= 0
                elif beside & {"B"} and not beside & {"F"}:
                    back += y < 0
                    wrong += y >= 0
    check("bases.rlb: a machine's front is +y, so shield sector 0 is the front",
          front and back and not wrong,
          f"of the running-gear nodes whose name puts F or B beside the side letter "
          f"(LFdd, WFRa, TBL, WMLB ...), all {front} front ones rest at y > 0 and all "
          f"{back} back ones at y < 0 ({wrong} disagree); the sector test turns the "
          f"hit into the object's frame first (Control.dll:0x1002c666) and gives +y "
          f"sector 0, -x 2 (the left gear's side) and +x 3")

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
    fortif_meshes = {e.name.lower()[:-4]: e for e in fortif if e.tag.upper().startswith("MESH")}
    frail = total_b = frail_bare = sturdy_bare = 0
    for entry in fortif:
        name = entry.name.lower()
        if not name.endswith(".ndp"):
            continue
        table = objects.parse_damage(fortif.read(entry), name)
        total_b += 1
        stub = table[0].durability <= 1 and len(table) > 1
        frail += stub
        if name[:-4] in fortif_meshes:
            model = objmesh.parse(fortif.read(fortif_meshes[name[:-4]]), name)
            bare = model.nodes[0].hit_slot() is None
            frail_bare += stub and bare
            sturdy_bare += not stub and bare
    check("fortif.rlb: a building's first node can be a 1-hit-point stub",
          frail > 0 and frail_bare == frail and not sturdy_bare,
          f"{frail}/{total_b} building tables give node 0 one hit point among "
          f"sturdier parts, and on all {frail_bare} of them node 0 has no level-0 "
          f"geometry for a round to strike, where it has on every other table "
          f"({sturdy_bare} bare).  A building is agent kind 3 (Terrain.dll:0x10057da0), "
          f"which Control.dll:0x100110ab only marks when node 0 goes, where a unit dies")

    # ---- the .exp's two floats after the radius: 1.0 everywhere, and unread
    pair_values: Counter[tuple[float, float]] = Counter()
    for path in sorted(game.glob("*.rlb")):
        try:
            archive = NResArchive.open(path)
        except NotAnNResArchive:
            continue
        for entry in archive:
            if entry.name.lower().endswith(".exp"):
                pair_values[effects.parse_explosion(archive.read(entry), entry.name).values] += 1
    check("*.exp: the two floats after the radius are 1.0 on every explosion",
          list(pair_values) == [(1.0, 1.0)],
          f"{dict(pair_values)}; the one place an .exp record is fetched "
          f"(Control.dll:0x100113db) reads its +4, +8 and +0x14 and its names, and "
          f"the hits it builds read only its kind -- +0xc and +0x10 are not read")


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

    # Computer 0 is the first class-13 record: CBuilding files items in load order
    # (Terrain.dll:0x100583a2), and the loader appends components in file order.
    two: dict[str, tuple[float, float]] = {}
    for name, entry in entries.items():
        if not entry.tag.upper().startswith("CTL"):
            continue
        c = control.parse(fortif.read(entry), names)
        rates = [min(c.channels[k].rate for k in p.entries) for p in c.components
                 if p.type_id == control.COMPUTER_TYPE and p.entries]
        if len(rates) == 2:
            two[name[:-4]] = (rates[0], rates[1])
    differ = {n: r for n, r in two.items() if not math.isclose(r[0], r[1])}
    check("fortif.rlb: where a building has two computers, the first is the pod at 0.5",
          len(two) == 18 and len(differ) == 8
          and all(math.isclose(a, 0.5, abs_tol=1e-6) and math.isclose(b, 0.2, abs_tol=1e-6)
                  for a, b in differ.values()),
          f"{len(two)} buildings carry two class-13 parts; they differ on {len(differ)} "
          f"({', '.join(sorted(differ))}): first 0.5, second a wrapping 0.2.  Computer 0 "
          f"is the first, so these pods fire the capture at 0.9 / 0.5 = 1.8 s, not 4.5 s")

    # A unit stands in a place only while its world speed is at most 2 m/s -- 1000 at
    # the teleport places (Behavior.dll:0x100184f0).
    teleport: Counter[str] = Counter()
    other = 0
    for name, entry in entries.items():
        if not name.endswith(".msh"):
            continue
        graph = objmesh.read_path_graph(NResArchive(fortif.read(entry), entry.name))
        for n in graph.nodes if graph else ():
            if n.flags & objmesh.PLACE_TELEPORT:
                teleport[name[:-4]] += 1
            elif n.flags & (objmesh.PLACE_POD | objmesh.PLACE_DOCK | objmesh.PLACE_MINE
                            | objmesh.PLACE_STORE):
                other += 1
    check("fortif.rlb: only the main teleport's places let a moving unit count",
          dict(teleport) == {"fr_m_mtp": 4, "fr_b_ruin": 1} and other > 40,
          f"places with a bit of {objmesh.PLACE_TELEPORT:#x}, where the speed bound is "
          f"{objmesh.PLACE_TELEPORT_SPEED:g}: {dict(teleport)}; the other {other} pods, "
          f"docks and ore places count a unit only at {objmesh.PLACE_SPEED:g} m/s or less "
          f"(property 0x27, the world velocity)")

    charge: Counter[float] = Counter()
    for path in sorted(game.glob("MISSIONS/**/data.tma")):
        for obj in mission.load(path).objects:
            prop = obj.properties.get("ChargeRadius")
            if prop is not None:
                charge[float(prop.value)] += 1
    check("data.tma: ChargeRadius is always the constant its getter returns",
          list(charge) == [mission.CHARGE_RADIUS] and sum(charge.values()) == 463,
          f"{dict(charge)}: Behavior.dll's property getter returns 10000 for kind 6 "
          f"(0x1000b688) and its setter stores nothing (0x1000b575), so no mission "
          f"could change it")


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

    # 4b. the row's field, regener, is query 0x77: a class-15 device's value 0 x
    # condition (a class-9 device would give value 1).  Only repair parts use it,
    # and each part's first device is its repair system.
    regener = Counter(k.lower()[:5] for k, d in lib.items()
                      for s in d.stats if s.field == "regener")
    first_is_repair = 0
    intsys = NResArchive.open(game / "intsys.rlb")
    for entry in intsys:
        if entry.tag.upper().startswith("CTL") and entry.name.lower().startswith("o_rps_"):
            parsed = control.parse(intsys.read(entry), names)
            first_is_repair += parsed.components[0].type_id == control.REPAIR_TYPE
    check("objects.dlb: the Regeneration row reads the part's repair rate",
          dict(regener) == {"i_rps": 16} and first_is_repair == 16,
          f"field 'regener' on {dict(regener)}; the first device of {first_is_repair}/16 "
          f"o_rps controllers is class 15, whose value 0 x condition the field's "
          f"query answers (iron3d.dll:0x1006f62c id 0x77 -> Control.dll:0x1002c1b6)")

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

    # 8. what each part brings, in load order: records, nodes, and what a fitted
    #    part keeps of its slot
    arm = weapons.Armoury(game)
    assemblies = sorted(game.glob("UNITS/**/*.dat"))
    doubled = []
    one_record = with_entries = into = 0
    slot_entries: Counter[int] = Counter()
    external_parts: set[str] = set()
    for path in assemblies:
        unit = objects.load_unit(path)
        parents = unit.parents()
        classes: Counter[int] = Counter()
        firsts: dict[int, list[control.Component]] = {}
        for i, c in enumerate(unit.components):
            member = c.ref.member.lower()
            record, parsed = library.get(member), arm.controller(member)
            if record is None or parsed is None:
                continue
            if i == 0 or record.tag == objects.EXTERNAL_TAG:
                firsts[i] = list(parsed.components)
                classes.update(p.type_id for p in parsed.components)
                if i:
                    external_parts.add(member)
            elif record.tag == objects.INTERNAL_TAG:
                into += 1
                one_record += len(parsed.components) == 1
                with_entries += any(p.entries for p in parsed.components)
                slots = firsts.get(parents[i], [])
                if 0 <= c.attach_node < len(slots):
                    slot = slots[c.attach_node]
                    if slot.entries:
                        slot_entries[slot.type_id] += 1
                    classes[slot.type_id] -= 1
                    classes[parsed.components[0].type_id] += 1
        if any(classes[k] > 1 for k in control.SINGLE_POINTER_CLASSES):
            doubled.append(path.name)
    check("UNITS: no assembly loads two records of a single-pointer class",
          assemblies and not doubled,
          f"over {len(assemblies)} assemblies, counting the root's and every external "
          f"part's records with each internal part in its slot, none has two of class "
          f"1, 8, 9, 17, 21 or 27 -- the factory keeps the last (Control.dll:0x1002d56e)"
          + (f"; doubled: {doubled[:4]}" if doubled else ""))
    check("UNITS: a fitted part is one record with no entries, so a slot's entries stay",
          into and one_record == into and not with_entries
          and set(slot_entries) == {control.GUN_TYPE, control.RADAR_TYPE, control.DEFLECTOR_TYPE},
          f"all {one_record}/{into} internal parts and clips carry exactly one record and "
          f"none has entries; the slots they fill that do are "
          + ", ".join(f"{n} of class {k}" for k, n in sorted(slot_entries.items()))
          + " (guns, radars, deflectors).  The re-parse appends entries "
          "(Control.dll:0x10021df3) and replaces the rest")

    in_step = 0
    for member in sorted(external_parts):
        record = library.get(member)
        if record.mesh and record.damage:
            model = objmesh.parse(arm.read(record.mesh), record.mesh.member)
            rows = objects.parse_damage(arm.read(record.damage), record.damage.member)
            in_step += len(model.nodes) == len(rows)
    check("objects.rlb: an external part's mesh and .ndp list the same nodes",
          external_parts and in_step == len(external_parts),
          f"{in_step}/{len(external_parts)} turrets and guns in the assemblies; both "
          f"loaders drop node 0 of a part together (AniMesh.dll:0x1000a79d, "
          f"Control.dll:0x10008c6a), so the two stay in step")

    # 9. class 3, CICLS_SIMPLE: the wheels, tracks and rotors
    simple = []
    for path in all_archives(game):
        archive = NResArchive.open(path)
        for entry in archive:
            if entry.tag.upper().startswith("CTL"):
                blob = archive.read(entry)
                try:
                    parsed = control.parse(blob)
                except control.ControlFormatError:
                    continue
                for p in parsed.components:
                    if p.type_id == control.SIMPLE_TYPE:
                        gains = struct.unpack_from("<2f", blob, p.offset + control.SIMPLE_GAINS_AT)
                        simple.append((path.name.lower(), entry.name.lower(), p, gains))
    on_chassis = {name for lib, name, _, _ in simple if lib == "bases.rlb"}
    drive = {name for _, name, p, g in simple if p.flags in (0x01070C00, 0x02040C00)
             and g == (1.0, 0.5)}
    quiet = all(not any(p.values[1:]) and p.values[0] in (0.0, 0.5) and p.power == 0
                for _, _, p, _ in simple)
    half = sum(p.values[0] == 0.5 for _, _, p, _ in simple)
    picks = all(((p.flags >> shift) & 0xFF) in set(control.SIMPLE_SOURCES) | {0, 1}
                for _, _, p, _ in simple for shift in (0, 8, 16))
    wheeled_tracked = {f"r_{s}_0{k}.ctl" for s in "lmb" for k in (3, 4)}
    check(".ctl: the class-3 records are simple devices driven by the motion",
          simple and quiet and picks and drive == wheeled_tracked,
          f"{len(simple)} records, {sum(lib == 'bases.rlb' for lib, *_ in simple)} on "
          f"{len(on_chassis)} chassis; zero power, values zero but a 0.5 first value on "
          f"{half} flyer records, every flag byte "
          f"0, 1 or a source 0x10020d90 knows; the forward-speed +/- half-turn drive "
          f"(0x01070C00/0x02040C00 at gains 1 and 0.5) is on exactly the six wheeled "
          f"and tracked chassis")

    # 10. TRF1 is state bits: available, researched, in the tree
    opened = locked = opened_ok = locked_ok = 0
    for tree in trees:
        for item in tree.items:
            done = all(tree[r].researched for r in item.requires)
            if item.category == research.STATE_IN_TREE | research.STATE_AVAILABLE:
                opened += 1
                opened_ok += done
            elif item.category == research.STATE_IN_TREE:
                locked += 1
                locked_ok += not done
    check(".trf: TRF1 is state bits -- an item opens once its prerequisites are researched",
          opened and locked and opened_ok == opened and locked_ok == locked,
          f"{opened_ok}/{opened} items in the tree and open (5) have every prerequisite "
          f"researched (bit 2), and {locked_ok}/{locked} in the tree and locked (4) have "
          f"one that is not -- MisLoad.dll slot 30's rule (0x10002c10); 0 is out of the tree")

    # 11. the chassis's own body goes back to its payload
    shop = units.Workshop(game)
    loads = [shop.weigh(path) for path in assemblies
             if objects.load_unit(path).components[0].ref.member.lower().startswith("r_")]
    loads = [w for w in loads if w is not None]
    fits = sum(not w.over for w in loads)
    without = sum(w.total > w.payload for w in loads)
    check("UNITS: with the chassis's own body given back, nearly every robot fits its payload",
          loads and fits >= len(loads) - 10 and without > len(loads) // 3,
          f"{fits} of {len(loads)} robots weigh no more than payload + body "
          f"(units.Workshop.weigh; Control.dll:0x1000fb6f gives part 0's nodes back); "
          f"counting the body against the payload, {without} would not")

    # 12. the S-6f is named by the catalogue and the trees only
    naming = set()
    for path in sorted(game.rglob("*")):
        if path.is_file() and path.suffix.lower() != ".trf" \
                and b"r_l_06" in path.read_bytes().lower():
            naming.add(path.name.lower())
    check("install: only objects.rlb, objects.dlb and bases.rlb name r_l_06, besides the trees",
          naming == {"objects.rlb", "objects.dlb", "bases.rlb"},
          f"outside MISSIONS/SCRIPTS/*.trf the S-6f is named by {sorted(naming)}: no "
          f"assembly, mission or binary")


#: ``Control.dll``'s item: the base component's vtable, the radar's, their shared
#: update, and where the constructor, the step and the step length keep their numbers.
ITEM_VTABLE = 0x1003C448
RADAR_VTABLE = 0x1003C800
ITEM_UPDATE = 0x10020900
ITEM_CONSTRUCTOR = 0x10020800
RADAR_CONSTRUCTOR = 0x10024310
ITEM_STATE_SITE = 0x10020832    # mov dword ptr [esi+0x50], 5
ITEM_STEP_AT = 0x1003C488       # 0.45, at 0x10020a3a and 0x10020a48
ITEM_IDLE_AT = 0x1003B934       # 100.0, at 0x10020d72
STEP_MS_AT = 0x1003C504         # 1000.0, at 0x1002216d
#: The nodes of Mission 01's placed objects that move without a state, by assembly:
#: the class-3 and radar channels that play more than one frame.
MISSION_01_MOVING = {
    "helic.dat": {"Tup_m1o1", "Tdn_m1o1", "TTrad_m1o1"},
    "tut1_mf1.dat": {"engnL_m1o1", "engnR_m1o1", "LTwng_m1o1", "LBwng_m1o1",
                     "RTwng_m1o1", "RBwng_m1o1", "TMrad_m1o1"},
    "tut1_e1.dat": {"TTrad_m1o1"},
    "tut1_p.dat": set(), "l_targ.dat": set(), "m_targ.dat": set(), "m_bridge.dat": set(),
}


def _turns(item: control.Item, seconds: float, fps: float, sources=None) -> float:
    """How many turns an item's first channel makes a second, stepped at ``fps``."""
    total, last, t = 0.0, None, 0.0
    while t <= seconds * 1000.0:
        item.tick(t, sources)
        if last is not None:
            gap = item.now[0] - last
            total += gap - round(gap)
        last, t = item.now[0], t + 1000.0 / fps
    return total / seconds


def check_moving_parts(check, game: Path) -> None:
    """Rotors, dishes and wings: the item update that turns them, and what it turns."""
    path = game / "Control.dll"
    if path.exists():
        at = _image_at(path.read_bytes())

        def slots(va: int) -> tuple[int, ...]:
            return struct.unpack("<16I", at(va, 64))

        item, radar = slots(ITEM_VTABLE), slots(RADAR_VTABLE)
        calls = [t for _, t in _calls(at, RADAR_CONSTRUCTOR, 16)]
        numbers = (struct.unpack("<f", at(ITEM_STEP_AT, 4))[0],
                   struct.unpack("<f", at(ITEM_IDLE_AT, 4))[0],
                   struct.unpack("<f", at(STEP_MS_AT, 4))[0])
        check("Control.dll: a radar is an item and keeps the item's update",
              item[11] == radar[11] == ITEM_UPDATE and item[1:] == radar[1:]
              and item[0] != radar[0] and calls[:1] == [ITEM_CONSTRUCTOR]
              and at(ITEM_STATE_SITE, 7) == bytes.fromhex("c7465005000000")
              and abs(numbers[0] - control.ITEM_STEP) < 1e-6
              and numbers[1:] == (control.ITEM_IDLE_MS, 1000.0),
              f"vtables {ITEM_VTABLE:#x} and {RADAR_VTABLE:#x} share slots 1-15 (update "
              f"{item[11]:#x}) and differ in slot 0; the radar's constructor calls "
              f"{calls[0]:#x} first; the item starts in state 5; the step is "
              f"{numbers[0]:.2f}, an idle step {numbers[1]:g} ms, lengths in "
              f"{numbers[2]:g}ths of a second")

    # every class-3 and radar record keeps the item's defaults
    records = []
    for archive_path in all_archives(game):
        archive = NResArchive.open(archive_path)
        for entry in archive:
            if not entry.tag.upper().startswith("CTL"):
                continue
            blob = archive.read(entry)
            try:
                parsed = control.parse(blob)
            except control.ControlFormatError:
                continue
            for p in parsed.components:
                if p.type_id in (control.SIMPLE_TYPE, control.RADAR_TYPE):
                    groups = struct.unpack_from("<2i", blob, p.offset + control.ITEM_ON_GROUP_AT)
                    records.append((archive_path.name.lower(), entry.name.lower(), p, groups,
                                    parsed.channels))
    radars = [r for r in records if r[2].type_id == control.RADAR_TYPE]
    simple = [r for r in records if r[2].type_id == control.SIMPLE_TYPE]
    defaults = all(p.state is None and groups == (-1, -1) for _, _, p, groups, _ in records)
    plain = all(p.flags == 0 and p.weights == (1.0, 1.0) for _, _, p, _, _ in radars)
    check(".ctl: every class-3 and radar record keeps the item's starting word 5",
          records and defaults and plain and len(radars) == 76 and len(simple) == 72,
          f"{len(simple)} class-3 and {len(radars)} radar records: state -1 and no switch "
          f"groups at +0x10/+0x14 on all; every radar's flags 0 and weights 1 and 1, so a "
          f"rate of 1")

    # a radar's entries are its dish: a wrapping 2 pi channel of four quarter turns
    library = objects.ObjectLibrary(game / "objects.rlb")
    meshes: dict[tuple[str, str], tuple[str, str]] = {}
    for record in library.records.values():
        ctl, msh = record.slot_with_suffix("ctl"), record.mesh
        if ctl and msh:
            meshes.setdefault((ctl.library.lower(), ctl.member.lower()), (msh.library, msh.member))
    parts = assembly.Assembly(game)
    dishes = quarter = 0
    rates: Counter[float] = Counter()
    holders = set()
    for lib, name, p, _, channels in radars:
        for e in p.entries:
            ch = channels[e]
            dishes += 1
            holders.add((lib, name))
            rates[round(ch.rate, 2)] += 1
            ref = meshes.get((lib, name))
            model = parts.mesh(objects.ResourceRef(*ref)) if ref else None
            if model is None or not (ch.flags & control.CHANNEL_WRAP and ch.last - ch.first == 4
                                     and abs(ch.span - 6.28) < 1e-3):
                continue
            yaws = [math.degrees(2 * math.atan2(model.pose_at(ch.node, float(f))[1][3],
                                                model.pose_at(ch.node, float(f))[1][0]))
                    for f in range(int(ch.first), int(ch.last) + 1)]
            steps = [(b - a + 180) % 360 - 180 for a, b in zip(yaws, yaws[1:], strict=False)]
            quarter += len({round(s) for s in steps} - {89, -89}) == 1 and all(
                abs(abs(s) - 90) < 1.5 for s in steps)
    hero = next(r for r in radars if r[1] == HERO_TURRET + ".ctl")
    check(".ctl: a radar's channel is its dish, one turn a unit of value",
          dishes == quarter == 60 and len(holders) == 58 and not hero[2].entries
          and rates == Counter({0.5: 56, 0.3: 2, 1.3: 2}),
          f"{dishes} channels on {len(holders)} radar records wrap, span 2 pi and play four "
          f"frames a quarter turn apart about the node's z on {quarter}; rates {dict(rates)}; "
          f"the hero's turret's radar slot has {len(hero[2].entries)} entries")

    # stepped as read: the rotors, the dishes and the M-2f's switches
    bases = NResArchive.open(game / "bases.rlb")
    turrets = NResArchive.open(game / "turrets.rlb")
    t2 = control.parse(bases.read_name("r_t_02.ctl"))
    rotors = [_turns(control.Item(p, t2.channels), 10.0, 60.0)
              for p in t2.components if p.type_id == control.SIMPLE_TYPE]
    dishes_spin = []
    for name in ("o_tur_tb_01.ctl", "o_tur_mb_01.ctl", "o_tur_tt_01.ctl"):
        c = control.parse(turrets.read_name(name))
        dishes_spin += [_turns(control.Item(p, c.channels), 10.0, 60.0)
                        for p in c.components if p.type_id == control.RADAR_TYPE and p.entries]
    m2 = control.parse(bases.read_name("r_m_02.ctl"))
    devices = [p for p in m2.components if p.type_id == control.SIMPLE_TYPE]
    wings = control.Item(devices[3], m2.channels)
    engine = control.Item(devices[0], m2.channels)
    frames = []
    for t in range(0, 8001, 50):
        speed = {12: 0.6 if t >= 1000 else 0.0}
        wings.tick(float(t), speed)
        engine.tick(float(t), speed)
        frames.append((t, wings.frame(0), engine.frame(0)))
    rest = all(w == 2.0 and e == 2.0 for t, w, e in frames if t <= 1000)
    swept = min(t for t, w, _ in frames if w == 1.0)
    out = min(t for t, _, e in frames if e == 1.0)
    top = m2.triples[2][1] * 3.6
    check("Control.dll item, stepped: a T-2 rotor turns 7.3 times a second, a dish 0.5",
          [round(r, 3) for r in rotors] == [7.3, 7.3]
          and [round(d, 3) for d in dishes_spin] == [0.5, 0.5, 0.5]
          and rest and 2000 <= swept <= 2150 and 6000 <= out <= 6150 and round(top) == 125,
          f"r_t_02's two constant devices {', '.join(f'{r:.3f}' for r in rotors)} turns a "
          f"second; the tiny and medium turrets' dishes "
          f"{', '.join(f'{d:.3f}' for d in dishes_spin)}; the M-2f's wings and side engines "
          f"hold frame 2 at rest and, from 0.6 of its {top:.0f} km/h top forward speed at "
          f"1 s, reach frame 1 by {swept} and {out} ms")

    # Mission 01: what moves without a state
    d01 = game / MISSION_01
    if not (d01 / "data.tma").exists():
        return
    m01 = mission.load(d01 / "data.tma")
    moving: dict[str, set[str]] = {}
    for o in m01.objects:
        if o.kind not in (mission.KIND_UNIT, mission.KIND_BUILDING):
            continue
        f = parts.unit_file(o.path)
        if f is None:
            continue
        unit = objects.load_unit(f)
        names: set[str] = set()
        for comp in unit.components:
            record = parts.library.get(comp.ref.member)
            ctl = record and record.slot_with_suffix("ctl")
            if not ctl or record.tag == "INTO":
                continue
            c = control.parse(parts.archive(ctl.library).read_name(ctl.member))
            model = parts.mesh(record.mesh) if record.mesh else None
            for p in c.components:
                if p.type_id not in (control.SIMPLE_TYPE, control.RADAR_TYPE):
                    continue
                for e in p.entries:
                    ch = c.channels[e]
                    if model and ch.last != ch.first:
                        names.add(model.nodes[ch.node].name)
        moving[f.name.lower()] = names
    check("Mission 01: what moves by itself is two rotors, a flyer's wings and three dishes",
          moving == MISSION_01_MOVING,
          "; ".join(f"{k}: {', '.join(sorted(v)) or 'none'}" for k, v in sorted(moving.items())))


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

    # What the AI's fight module holds back, and whose mount aims above the target.
    heavy = sorted({(e.code, g.round.damage) for e, g in guns.values()
                    if g.round and g.round.damage >= weapons.AI_HEAVY_DAMAGE})
    lobbed = Counter((e.sub, g.round.member.lower()) for e, g in guns.values()
                     if g.round and g.round.lobbed)
    flamers = sum(1 for e, g in guns.values() if e.sub == "FLM" and g.round)
    check("weapon.rlb: the AI holds back only the winged SSMs; only flame rounds fall",
          [c for c, _ in heavy] == sorted(["MWML1M", "LWML1L", "LWML2M"])
          and {s for s, _ in lobbed} == {"FLM"} and sum(lobbed.values()) == flamers,
          f"rounds of {weapons.AI_HEAVY_DAMAGE:g} damage or more: {heavy} -- the fight "
          f"module fires those only at a target whose id nibble is 3 (Behavior.dll:"
          f"0x10024d30); mode-3 rounds, which a mount lobs (Control.dll:0x10028401): "
          f"{dict(lobbed)}, on all {flamers} flamers")


#: A research item's role, for the roles a turret gives a unit.
TURRET_ROLE_TYPE = {research.ROLE_BATTLE: objects.TYPE_WARRIOR,
                    research.ROLE_TRANSPORT: objects.TYPE_TRANSPORT,
                    research.ROLE_BUILDER: objects.TYPE_BUILDER,
                    research.ROLE_HQ: objects.TYPE_HQ, research.ROLE_HERO: objects.TYPE_HERO}


def check_firing(check, game: Path) -> None:
    """Firing a gun: the button, the selected guns, the barrel stroke, the sight."""
    rows = controls.table(game / "hero.tbl")
    guns_rows = [r for r in rows if r.target == "CICLS_MULTIGUN"]
    fire = sorted((r.key, r.pressed, r.index, r.state) for r in guns_rows
                  if r.command == "MCMD_STATE")
    select = sorted((r.index, r.pressed) for r in guns_rows if r.command == "MCMD_SELECT")
    check("hero.tbl: the left button fires every selected gun; 0 selects all, 1..8 toggle one",
          fire == [("SCAN_LMOUSE", False, -1, "CIS_SWITCHOFF"),
                   ("SCAN_LMOUSE", True, -1, "CIS_CONTINUEFIGHT")]
          and select == [(-1, True)] + [(i, True) for i in range(1, 9)],
          f"button rows {fire}: index -1 reaches every gun whose selected byte is set "
          f"(World3D.dll:0x100105ed); select rows by index {[i for i, _ in select]}")

    names = frozenset(p.name.lower() for p in all_archives(game))
    arms_where = []
    shot_groups: Counter[bool] = Counter()
    group_actions: Counter[tuple[int, ...]] = Counter()
    barrels = rateless = sights_total = 0
    sights: Counter[tuple[str, str]] = Counter()
    for path in all_archives(game):
        archive = NResArchive.open(path)
        members = {e.name.lower() for e in archive}
        for entry in archive:
            if not entry.tag.upper().startswith("CTL"):
                continue
            try:
                c = control.parse(archive.read(entry), names)
            except control.ControlFormatError:
                continue
            if any(p.type_id == control.ARM_TYPE for p in c.components):
                arms_where.append(f"{path.name.lower()}/{entry.name.lower()}")
            points = None
            stem = entry.name.lower()[:-4]
            for p in c.components:
                if p.type_id in (control.GUN_TYPE, control.BUILDER_TYPE):
                    shot_groups[p.group != control.NO_GROUP] += 1
                    if p.group != control.NO_GROUP:
                        group_actions[tuple(sorted({r.action for r in c.references
                                                    if r.group == p.group}))] += 1
                    for e in p.entries:
                        barrels += 1
                        rateless += c.channels[e].rate <= 0 or c.channels[e].origin != -1
                elif p.type_id == control.TURRET_TYPE and len(p.entries) >= 2:
                    if points is None and stem + ".cpt" in members:
                        points = [q.name for q in objmesh.parse_control_points(
                            archive.read_name(stem + ".cpt"))]
                    if not points:
                        continue
                    yaw, pitch = (c.channels[i] for i in p.entries[:2])
                    sights_total += 1
                    sights[(points[yaw.origin], points[pitch.point])] += 1
    check(".ctl: a turret's guns aim along a ray from TurretCenter through TargetDirect",
          list(sights) == [("TurretCenter", "TargetDirect")] and barrels and rateless == 0,
          f"the yaw channel's second point (+0x10) and the pitch channel's point (+0x14) "
          f"on {sights_total} turret components: {dict(sights)}; a gun aims its round at "
          f"the first thing that ray meets, no nearer than 100 m (Control.dll:0x1002a610, "
          f"0x1002a7be).  All {barrels} barrel channels have a rate and no second point, "
          f"so every stroke ends and the round leaves from the one point")
    check(".ctl: only the hero turret has class-24 components; 34 guns run a shot group",
          arms_where == ["turrets.rlb/o_tur_ht_02.ctl"] and shot_groups[True] == 34
          and set(group_actions) <= {(control.ACT_EFFECT_START,),
                                     (control.ACT_EFFECT_POINTS,),
                                     (control.ACT_EFFECT_START, control.ACT_EFFECT_RESTART)},
          f"arms on {arms_where}; record +0xc names a group on {shot_groups[True]} of "
          f"{sum(shot_groups.values())} guns, holding only actions {dict(group_actions)}")

    turrets = NResArchive.open(game / "turrets.rlb")
    tur = control.parse(turrets.read_name("o_tur_ht_02.ctl"), names)
    points = [q.name for q in objmesh.parse_control_points(turrets.read_name("o_tur_ht_02.cpt"))]
    guns = [p for p in tur.components if p.type_id == control.GUN_TYPE]
    arms = [p for p in tur.components if p.type_id == control.ARM_TYPE]
    arm_frames = {(tur.channels[e].first, tur.channels[e].last) for p in arms for e in p.entries}
    shots = [(points[tur.channels[e].point], tur.channels[e].rate,
              round(tur.channels[e].stroke_ms + p.values[control.GUN_INTERVAL]))
             for p in guns for e in p.entries]
    cannon = guns[0].values[control.GUN_INTERVAL] if guns else 0.0
    check("o_tur_ht_02: a hero shot is its barrel's stroke and then the gun's interval",
          [p.resource.member.lower() for p in guns] == ["bb_h_01", "bp_h_01", "bl_h_01",
                                                         "bm_h_01"]
          and len(arms) == len(guns) and arm_frames == {(42.0, 48.0)}
          and [s[0] for s in shots] == ["Mgun_d", "Plaz_d", "Laz_d", "Roc_d1", "Roc_d2"]
          and [s[2] for s in shots] == [250, 750, 450, 2250, 1650]
          and all(p.group == control.NO_GROUP for p in guns),
          f"barrel point, rate and ms a shot: {shots} (1000 / rate of stroke, "
          f"Control.dll:0x1002a190, then value 3), so the cannon fires "
          f"{1000 / shots[0][2]:g} a second where its panel's 1000 / max(1, value 3) "
          f"says {1000 / max(1.0, cannon):g}.  Four arms play frames {arm_frames}; no "
          f"hero gun names a shot group")

    # A gun is ready once its arm is out: the turret pairs each follower with a gun
    # and an arm (Control.dll:0x10027170), and an arm is an item stepping 0.45.
    comps = tur.components
    mounts = tur.gun_mounts()
    arm_rates = {tur.channels[e].rate for p in arms for e in p.entries}
    unfold_s = 0.0
    if len(arm_rates) == 1:
        rate = next(iter(arm_rates))
        left = 1.0
        while left > 1e-9:
            step = min(control.ITEM_STEP, left)
            unfold_s += step / rate
            left -= step
    check("o_tur_ht_02: each gun's mount pairs with its arm, which unfolds in 0.5 s",
          [(comps[m.gun].resource.member.lower(), comps[m.arm].type_id) for m in mounts
           if m.gun is not None and m.arm is not None]
          == [("bb_h_01", 24), ("bp_h_01", 24), ("bl_h_01", 24), ("bm_h_01", 24)]
          and len(mounts) == 4 and {p.state for p in arms} == {0x21}
          and math.isclose(unfold_s, 0.5),
          f"followers {[m.channel for m in mounts]} take guns {[m.gun for m in mounts]} and "
          f"arms {[m.arm for m in mounts]} in order; every arm starts in state "
          f"{sorted({p.state for p in arms})} (record +0x18) and its channels run at "
          f"{sorted(arm_rates)} a second, so 0.45, 0.9, 1 take {unfold_s:g} s "
          f"(Control.dll:0x10020900).  The gun is not ready until then (0x10028200)")

    unpaired: list[str] = []
    paired = flag_2000 = 0
    for path in all_archives(game):
        archive = NResArchive.open(path)
        for entry in archive:
            if not entry.tag.upper().startswith("CTL"):
                continue
            try:
                c = control.parse(archive.read(entry), names)
            except control.ControlFormatError:
                continue
            flag_2000 += sum(1 for ch in c.channels if ch.flags & 0x2000)
            ms = c.gun_mounts()
            if any(m.gun is None for m in ms):
                unpaired.append(entry.name.lower())
            elif ms:
                paired += 1
    check(".ctl: every follower channel has a gun to aim, but on two gun parts",
          sorted(unpaired) == ["o_gun_ba_03.ctl", "o_gun_ta_02.ctl"] and paired > 60
          and flag_2000 == 0,
          f"on {paired} controllers every channel flagged 8 (and not 0x40) meets a class-2 "
          f"or class-30 part to pair with; {unpaired} (e_gun_bl_03, e_gun_tl_02) carry a "
          f"follower and no gun.  No channel carries the flag 0x2000 the turret's takt "
          f"treats apart (Control.dll:0x10027eec)")

    fx = effects.EffectLibrary(game / "effects.rlb")
    load = [r for r in tur.references if r.group == tur.load_group]
    made = {r.values[7]: r.resource.member.lower() for r in load
            if r.action == control.ACT_EFFECT_POINTS}
    bound = {made.get(r.args[0]): r.args[1] for r in load
             if r.action == control.ACT_EFFECT_TIME_POINT}
    barrel_nodes = [tur.channels[p.entries[0]].node for p in guns if p.entries]
    arm_nodes = [tur.channels[p.entries[0]].node for p in arms if p.entries]
    guns_fx = {k: v for k, v in bound.items() if k and not k.endswith("_sfx")}
    sfx = {k: v for k, v in bound.items() if k and k.endswith("_sfx")}
    modes = {k: fx.get(k).mode for k in bound if k and fx.get(k)}
    sound_windows = {e.window for k in sfx if fx.get(k)
                     for e in fx.get(k).emitters if e.is_sound}
    check("o_tur_ht_02: the gun effects follow the barrels, the _sfx the arms",
          sorted(guns_fx.values()) == sorted(barrel_nodes[:3])
          and sorted(sfx.values()) == sorted(arm_nodes)
          and set(modes.values()) == {4} and len(sound_windows) == 1
          and math.isclose(next(iter(sound_windows))[0], 0.15, abs_tol=1e-6),
          f"action 14 binds {guns_fx} to the barrel nodes {barrel_nodes} and {sfx} to the "
          f"arm nodes {arm_nodes}; all {len(modes)} are time mode 4, the node's phase "
          f"(Effect.dll:0x10005d3f, AniMesh.dll:0x10005600), so a gun's effect plays "
          f"through its stroke and an _sfx sound in "
          f"{[tuple(round(v, 2) for v in w) for w in sorted(sound_windows)]} of its arm")


#: The game commands that pick the player's target, their Command.dsc sentence,
#: and the key ui_other.man binds (iron3d.dll's command handler, 0x10071cd0).
TARGET_PICKS = {
    "CMD_JAMES_SELECT_TARGET": ("Target selection", "SCAN_TAB"),
    "CMD_JAMES_SELECT_ENEMY": ("Select nearest enemy", "SCAN_E"),
    "CMD_JAMES_SELECT_FRIEND": ("Select nearest friend", "SCAN_T"),
    "CMD_JAMES_AIM_TARGET": ("Aim target", "SCAN_RMOUSE"),
}


def check_targeting(check, game: Path) -> None:
    """The player's target: the keys that pick it, and the gate a guided gun keeps."""
    labels = controls.commands(game)

    def picks(name: str) -> dict[str, str]:
        path = game / name
        return ({b.command: b.chord for b in controls.bindings(path) if b.command in TARGET_PICKS}
                if path.exists() else {})

    shipped = {name: picks(name) for name in ("ui_other.man", "addition.man")}
    default = picks("ui_other_d.man")
    want = {c: key for c, (_, key) in TARGET_PICKS.items()}
    check("ui_other.man: Tab, E, T and the right button pick the player's target",
          all(keys == want for keys in shipped.values())
          and default == {**want, "CMD_JAMES_SELECT_FRIEND": "SCAN_F"}
          and all(labels.get(c) == text for c, (text, _) in TARGET_PICKS.items())
          and [controls.CMD.get(c) for c in TARGET_PICKS] == [732, 733, 734, 750],
          f"{ {c[10:]: (controls.CMD.get(c), labels.get(c), k) for c, k in want.items()} }; "
          f"ui_other_d.man puts the friend on {default.get('CMD_JAMES_SELECT_FRIEND')}.  "
          f"iron3d.dll's handler sends 732-734 to 0x10090dc0, 0x10090e30, 0x10091070 and "
          f"750 to 0x100911f0 on the driven unit's target list")

    roles = {d.role: d.bindings for d in resources.descriptors(game / "ui" / "game_resources.cfg")}
    iron = resources.strings((game / "iron3d.dll").read_bytes())
    sounds = (roles.get("sounds", {}).get("TARGET_SELECTED"),
              roles.get("voices", {}).get("VOICE_UNIT_DETECTED"),
              roles.get("voices", {}).get("VOICE_ENEMY_DETECTED"))
    check("game_resources.cfg: a new target, a vacant vehicle and an enemy each have a sound",
          sounds == ("i_trg_sel.wav", "vc_u_det.wav", "vr_eu_det.wav")
          and iron.get(3040) == "Vacant vehicle detected..."
          and iron.get(5073) == "Sensor range:",
          f"TARGET_SELECTED, VOICE_UNIT_DETECTED, VOICE_ENEMY_DETECTED = {sounds} "
          f"(iron3d.dll:0x10090aec, 0x10075882, 0x10091ea6); string 3040 "
          f"{iron.get(3040)!r} with the vacant vehicle (0x10075832), and 5073 "
          f"{iron.get(5073)!r} beside property 0x50, the range a target is kept within")

    consts = {v.name: int(v.default, 0) for v in behaviour.variables(game)
              if v.name in ("ROBOT_HERO", "BUILDING_BRIDGE", "BUILDING_RUINE",
                            "ORDER_ROBOT_SHUTDOWN")}
    check("varset.var: the list leaves out heroes, bridges, ruins and shut-down enemies",
          consts == {"ROBOT_HERO": 0x1020000, "BUILDING_BRIDGE": 0x80001000,
                     "BUILDING_RUINE": 0x80002000, "ORDER_ROBOT_SHUTDOWN": 0x13},
          f"{ {k: hex(v) for k, v in sorted(consts.items())} }: the types "
          f"iron3d.dll:0x10091c80 compares and the first order 0x10077410 tests")

    names = frozenset(p.name.lower() for p in all_archives(game))
    turret_states: Counter[int | None] = Counter()
    gun_tails: Counter[bool] = Counter()
    for path in all_archives(game):
        archive = NResArchive.open(path)
        for entry in archive:
            if not entry.tag.upper().startswith("CTL"):
                continue
            try:
                c = control.parse(archive.read(entry), names)
            except control.ControlFormatError:
                continue
            for p in c.components:
                if p.type_id == control.TURRET_TYPE:
                    turret_states[p.state] += 1
                elif p.type_id in (control.GUN_TYPE, control.BUILDER_TYPE):
                    gun_tails[not any(p.values[8:11])] += 1
    check(".ctl: a turret keeps its class's state, and no gun files values 8-10",
          list(turret_states) == [None] and list(gun_tails) == [True],
          f"record +0x18 on {sum(turret_states.values())} turrets: {dict(turret_states)}, so "
          f"every turret starts in 0x200, CIS_MANUALCONTROL (Control.dll:0x10027103), where "
          f"its relink hands an unguided gun no target (0x10028164); values 8-10 are zero on "
          f"{gun_tails[True]} of {sum(gun_tails.values())} guns, and the gun fills them from "
          f"its round (0x100297e0-0x10029907)")

    arm = weapons.Armoury(game)
    rounds = NResArchive.open(game / "weapon.rlb")
    loaded = [arm.round(e.name[:-4]) for e in rounds if e.name.lower().endswith(".ctl")]
    loaded = [r for r in loaded if r is not None]
    guided = [r for r in loaded if r.guided]
    untargeted = sorted(r.member.lower() for r in guided if r.lock_ms <= 0)
    turrets = NResArchive.open(game / "turrets.rlb")
    tur = control.parse(turrets.read_name("o_tur_ht_02.ctl"), names)
    hero = [(p.resource.member.lower(), weapons.target_gate(arm.round(p.resource.member)))
            for p in tur.components if p.type_id == control.GUN_TYPE]
    shown = [(m, round(g.range), round(g.cone_cos, 4), round(g.lock_s, 2)) for m, g in hero]
    check("o_tur_ht_02: the plasma rifle and the missiles need a target, held for a lock",
          shown == [("bb_h_01", 500, -1.0, -1.0), ("bp_h_01", 150, 0.9689, 0.25),
                    ("bl_h_01", 1000, -1.0, -1.0), ("bm_h_01", 350, 0.66, 4.0)]
          and all(r.range > 0 for r in loaded)
          and len(untargeted) == 3,
          f"(round, value 8 m, value 10, value 9 s) = {shown}: a guided gun fires at a target "
          f"within value 8 and whose cosine off the barrel beats value 10, once value 9 has "
          f"counted down (Control.dll:0x10029d3a-0x10029f45).  All {len(loaded)} rounds have "
          f"a range; {len(guided) - len(untargeted)} of {len(guided)} seekers carry a "
          f"lock, and the guns of {untargeted} fire untargeted")

    first = next((d for d in gamedir.missions(game)
                  if d.as_posix().endswith("CAMPAIGN.00/Mission.01")), None)
    if first is None:
        return
    tut = mission.load(first / "data.tma")
    words = tut.relations()
    clan_names = [c.name for c in tut.clans]
    player = clan_names.index("Plr") if "Plr" in clan_names else 0
    hostile = [clan_names[j] for j, w in enumerate(words[player])
               if j != player and w == mission.RELATION_HOSTILE]
    by_clan: dict[str, list[str]] = defaultdict(list)
    for o in tut.objects:
        if o.kind == mission.KIND_UNIT and 0 <= o.clan_index < len(clan_names):
            by_clan[clan_names[o.clan_index]].append(o.path.split("\\")[-1].lower())
    check("Mission 01: E finds only Enm's walker; the dummies are neutral",
          hostile == ["Enm"] and by_clan["Enm"] == ["tut1_e1.dat"]
          and words[player][clan_names.index("Trgt")] == mission.RELATION_NEUTRAL,
          f"clans hostile to Plr: {hostile}, whose units are {by_clan['Enm']}; Trgt's "
          f"{sorted(set(by_clan['Trgt']))} are neutral to Plr, so Tab, the right button or the "
          f"nearest-listed rule picks them (iron3d.dll:0x10090d13), never E")


#: The clan-record tests ``iron3d.dll``'s marker colour calls, by their bodies
#: (``0x10039440``-``0x100394ba``): its type, or its SuperAI's word (slot 8) towards a clan.
_WORD_TEST = "8b5424048b41508b085250ff5120"
_CLAN_TESTS = {
    "type 0": bytes.fromhex("8b510c33c085d20f94c0c3"),
    "type 3": bytes.fromhex("8b510c33c083fa030f94c0c3"),
    "word 1": bytes.fromhex(_WORD_TEST + "48f7d81bc040c20400"),
    "word 2": bytes.fromhex(_WORD_TEST + "83e802f7d81bc040c20400"),
    "word 0": bytes.fromhex(_WORD_TEST + "f7d81bc040c20400"),
}


def _marker_rule(iron: bytes) -> list[tuple[str, int]] | None:
    """``iron3d.dll:0x10065440`` read off its bytes: the colour for the viewer's own clan,
    then each clan test it calls in order with the colour it returns when the test holds,
    and last the colour when none does.

    The function begins ``imul edx, edx, 0x68; cmp ecx, edi; lea esi, [edx+eax+0x724];
    jne; pop edi; mov eax, own``.  Each test after is ``[push edi] mov ecx, esi; call
    test; test al, al; je; pop edi; mov eax, colour; pop esi; ret 8``, and the last folds
    its answer into ``neg al; ... sbb eax, eax; and eax, a; add eax, b``.
    """
    anchor = iron.find(bytes.fromhex("6bd2683bcf8db40224070000750a5fb8"))
    if anchor < 0:
        return None
    sections, _ = resources._sections(iron)

    def test_at(call: int) -> str | None:
        rva = next((v + call - raw for v, size, raw in sections if raw <= call < raw + size), None)
        if rva is None:
            return None
        target = rva + 5 + struct.unpack_from("<i", iron, call + 1)[0]
        body = iron[resources._offset(sections, target):][:32]
        return next((name for name, want in _CLAN_TESTS.items() if body.startswith(want)), None)

    rule = [("own", struct.unpack_from("<I", iron, anchor + 16)[0])]
    at = anchor + 24  # past `mov eax, own; pop esi; ret 8`
    for _ in range(8):
        if iron[at] == 0x57:  # push edi: the word tests take the viewer's clan
            at += 1
        if iron[at:at + 3] != b"\x8b\xce\xe8":
            return None
        name = test_at(at + 2) or "?"
        tail = iron[at + 7:at + 7 + 16]
        if tail[:6] == bytes.fromhex("84c0740a5fb8"):
            rule.append((name, struct.unpack_from("<I", tail, 6)[0]))
            at += 7 + 14
        elif tail[:7] == bytes.fromhex("f6d85f5e1bc025") and tail[11] == 0x05:
            held = struct.unpack_from("<I", tail, 7)[0]
            other = struct.unpack_from("<I", tail, 12)[0]
            return [*rule, (name, (held + other) & 0xFFFFFFFF), ("otherwise", other)]
        else:
            return None
    return None


def check_target_marks(check, game: Path) -> None:
    """How the game colours what it marks: the rule, and what it gives Mission 01."""
    iron = (game / "iron3d.dll").read_bytes()
    rule = _marker_rule(iron)

    def rgb(argb: int) -> tuple[int, int, int]:
        return ((argb >> 16) & 0xFF, (argb >> 8) & 0xFF, argb & 0xFF)

    want = [("own", mission.MARKER_OWN), ("type 0", mission.MARKER_NATURE),
            ("type 3", mission.MARKER_NEUTRAL_CLAN),
            ("word 1", mission.MARKER_BY_RELATION[mission.RELATION_NEUTRAL]),
            ("word 2", mission.MARKER_BY_RELATION[mission.RELATION_ALLIED]),
            ("word 0", mission.MARKER_BY_RELATION[mission.RELATION_HOSTILE]),
            ("otherwise", mission.MARKER_OTHER)]
    got = [(name, rgb(c)) for name, c in rule] if rule else None
    check("iron3d.dll: a mark is light blue for its own clan, then by type, then by relation",
          got == want and all(c >> 24 == 0xFF for _, c in rule or []),
          f"0x10065440 in order: {got}; the tests are the clan record's type (+0xc) "
          f"and its SuperAI's word towards the viewer's clan")

    first = next((d for d in gamedir.missions(game)
                  if d.as_posix().endswith("CAMPAIGN.00/Mission.01")), None)
    if first is None:
        return
    tut = mission.load(first / "data.tma")
    names = [c.name for c in tut.clans]
    if "Plr" not in names:
        return
    player = names.index("Plr")
    colours = {c.name: tut.marker_colour(player, i) for i, c in enumerate(tut.clans)}
    by_clan: dict[str, list[str]] = defaultdict(list)
    for o in tut.objects:
        if o.kind in (mission.KIND_UNIT, mission.KIND_BUILDING) and 0 <= o.clan_index < len(names):
            by_clan[names[o.clan_index]].append(o.path.split("\\")[-1].lower())
    check("Mission 01: the dummies are marked magenta, tut1_e1 red, the neutral bots grey",
          colours == {"Plr": (128, 128, 255), "Trgt": (255, 0, 255), "Enm": (255, 0, 0),
                      "Ntrl": (160, 160, 160)}
          and sorted(set(by_clan["Trgt"])) == ["l_targ.dat", "m_targ.dat"]
          and by_clan["Enm"] == ["tut1_e1.dat"]
          and sorted(by_clan["Ntrl"]) == ["helic.dat", "tut1_mf1.dat"],
          "as Plr sees them: " + "; ".join(
              f"{n} (type {tut.clans[i].type}) {colours[n]} on {sorted(set(by_clan[n]))}"
              for i, n in enumerate(names)))

    roles = {d.role: d.bindings for d in resources.descriptors(game / "ui" / "game_resources.cfg")}
    page = roles.get("textures", {}).get("page9")
    ui = NResArchive.open(game / "ui" / "ui.lib")
    entries = list(ui)
    sprite = None
    if page is not None and 0 <= int(page) < len(entries):
        tex = texm.decode(ui.read(entries[int(page)]))
        alpha = [[tex.rgba[(y * tex.width + x) * 4 + 3] > 128 for x in range(55, 69)]
                 for y in range(28)]
        sprite = (entries[int(page)].name, tex.width, sum(map(sum, alpha)),
                  any(alpha[0]) or any(alpha[-1]))
    # The HUD's sprite 1: `push 0; push 28.0; push 14.0; push 0; push 55.0; push 256.0`
    # after page9's handle, at 0x10043500.
    loaded = iron.find(bytes.fromhex("5568 0000e041 68 00006041 55 68 00005c42 68 00008043"
                                     .replace(" ", "")))
    check("ui.lib: the mark's bracket is page9's 14 by 28 at (55, 0)",
          loaded >= 0 and sprite is not None and sprite[:2] == ("ui_tex9.tex", 256)
          and sprite[2] > 100 and not sprite[3],
          f"iron3d.dll:0x100433a0 cuts sprite 1 at (55, 0) 14 x 28 from page9 "
          f"(game_resources.cfg entry {page}): {sprite and sprite[0]} {sprite and sprite[1]} "
          f"wide, {sprite and sprite[2]} opaque pixels, none on its top or bottom row; "
          f"0x10077d80 draws it left of a unit and mirrored right, tinted by the rule")


#: The target panel's named sprites in ``ui/hq.cfg``: page, x, y, width, height.
TARGET_PANEL_SPRITES = {
    "targeter_back": ("ui_menu3", 0, 82, 150, 174),
    "targeter_range": ("ui_menu3", 0, 51, 39, 16),
    "targeter_life": ("ui_menu", 49, 94, 15, 15),
    "targeter_energy": ("ui_menu", 113, 126, 15, 15),
    "left_shld": ("ui_menu3", 192, 82, 26, 79),
    "frwd_shld": ("ui_menu3", 185, 59, 61, 19),
    "back_shld": ("ui_menu3", 151, 194, 89, 29),
    "top_shld": ("ui_menu3", 115, 59, 69, 19),
    "bott_shld": ("ui_menu3", 150, 224, 105, 31),
}

#: Where ``iron3d.dll`` draws them on the 640 x 480 HUD: for each call of the quad draw
#: ``0x1008f970`` walked from the address given, the sprite's offset in its widget, the
#: target panel's (x0, y0, x1, y1), the own panel's -- x0 > x1 is a mirrored sprite -- and
#: the colour, a constant or the register holding it.
TARGET_PANEL_QUADS = {
    "frame": (0x10040FA2, [(0x90, (0, 306, 150, 480), (640, 306, 490, 480), -1),
                           (0x11C, (129, 411, 144, 426), (496, 411, 511, 426), -1),
                           (0x1A8, (5, 411, 20, 426), (620, 411, 635, 426), -1)]),
    # left_shld twice, then frwd, back, top and bott: sectors 2, 3, 0, 1, 4, 5
    "shields": (0x10042B3D, [(4, (16, 334, 42, 413), (507, 334, 533, 413), "eax"),
                             (4, (133, 334, 107, 413), (624, 334, 598, 413), "ebp"),
                             (0x90, (44, 321, 105, 340), (535, 321, 596, 340), "edi"),
                             (0x11C, (30, 409, 119, 438), (521, 409, 610, 438), "ebx"),
                             # the last two load the same stack slots into other registers
                             (0x1A8, (40, 308, 109, 327), (531, 308, 600, 327), ("ecx", "eax")),
                             (0x234, (22, 420, 127, 451), (513, 420, 618, 451), ("edx", "ecx"))]),
}

#: The panel's floats in ``iron3d.dll``'s data, by address.
TARGET_PANEL_FLOATS = {
    0x100E5E6C: 9.0, 0x100E5E68: 315.0, 0x100E5E64: 137.0, 0x100E5E60: 443.0,
    0x100E5E5C: 503.0, 0x100E5E58: 631.0,
    0x100E5E78: 138.0, 0x100E5E74: 31.0, 0x100E5E70: 112.0,
    0x100E5DB8: math.pi / 6, 0x100E5D10: 1.25,
    0x100E5E4C: 0.025, 0x100E5E48: 1.1, 0x100E5CF0: 200.0,
    0x100E5C68: 0.2, 0x100E4BC4: 20.0, 0x100E59C8: 255.0,
}

#: The panel's colours: the life and energy arcs, the name, "Dangerous!", the distance.
TARGET_PANEL_COLOURS = {"life": 0xFF19FFAF, "energy": 0xFFFFB450, "name": 0xFFC8C8C8,
                        "danger": 0xFFC80000, "range": 0xFF00FF00}

#: The strings the panel prints: the distance's unit, the warning, the first status, and
#: the words a unit's name is made of.
TARGET_PANEL_STRINGS = {6178: "m", 6255: "Dangerous!", 6180: "no order", 6230: "Human",
                        6253: "Animal", 6076: "Tiny Tower", 6200: "Transport", 6201: "Builder",
                        6202: "Warrior", 6203: "Comm. Center", 6204: "Human", 6205: "Unknown"}

#: The names Mission 01's bots carry in the recording's target panel, less the count.
MISSION_01_PANEL_NAMES = {"tut1_mf1.dat": "MFW", "helic.dat": "TFW", "tut1_e1.dat": "TSW",
                          "l_targ.dat": "SSW", "m_targ.dat": "SSW"}


def _image_base(image: bytes) -> int:
    lfanew = struct.unpack_from("<I", image, 0x3C)[0]
    return struct.unpack_from("<I", image, lfanew + 24 + 28)[0]


def _va(image: bytes, va: int) -> int:
    """A virtual address's offset in its PE file."""
    sections, _ = resources._sections(image)
    return resources._offset(sections, va - _image_base(image))


def _quad_calls(image: bytes, start: int, count: int, follow: bool) -> list[tuple]:
    """Walk ``count`` calls from ``start``: each one's target, the offset loaded into ecx,
    and its pushed arguments in call order (constants, or the names of pushed registers).

    Only what the panels' sequences hold is decoded -- pushes, ``lea ecx``, register moves,
    tests, a ``jne`` (taken when ``follow``) and a ``jmp``; anything else ends the walk.
    """
    registers = ("eax", "ecx", "edx", "ebx", "esp", "ebp", "esi", "edi")
    sections, _ = resources._sections(image)
    base = _image_base(image)
    out: list[tuple] = []
    at = _va(image, start)
    pushes: list[object] = []
    loaded = None
    while len(out) < count:
        op = image[at]
        if op == 0x6A:
            pushes.append(struct.unpack_from("<b", image, at + 1)[0])
            at += 2
        elif op == 0x68:
            pushes.append(struct.unpack_from("<i", image, at + 1)[0])
            at += 5
        elif 0x50 <= op <= 0x57:
            pushes.append(registers[op - 0x50])
            at += 1
        elif image[at:at + 2] in (b"\x8d\x8e", b"\x8d\x8a"):
            loaded = struct.unpack_from("<i", image, at + 2)[0]
            at += 6
        elif image[at:at + 2] in (b"\x8d\x4e", b"\x8d\x4a"):
            loaded = image[at + 2]
            at += 3
        elif image[at:at + 3] in (b"\x8b\x4c\x24", b"\x8b\x54\x24", b"\x8b\x44\x24",
                                  b"\x89\x4c\x24"):
            at += 4
        elif image[at:at + 2] in (b"\x8b\xce", b"\x85\xc9", b"\x85\xc0"):
            at += 2
        elif image[at:at + 2] == b"\x0f\x85":
            at += 6 + (struct.unpack_from("<i", image, at + 2)[0] if follow else 0)
        elif op == 0xE9:
            at += 5 + struct.unpack_from("<i", image, at + 1)[0]
        elif op == 0xE8:
            rva = next(v + at - raw for v, size, raw in sections if raw <= at < raw + size)
            target = base + rva + 5 + struct.unpack_from("<i", image, at + 1)[0]
            out.append((target, loaded, tuple(reversed(pushes))))
            pushes, loaded = [], None
            at += 5
        else:
            break
    return out


def check_target_panel(check, game: Path) -> None:
    """The target panel and the player's own unit: sprites, places, figures, words."""
    path = game / "iron3d.dll"
    cfg = game / "ui" / "hq.cfg"
    if not path.exists() or not cfg.exists():
        return
    iron = path.read_bytes()

    # The sprites: hq.cfg's rects, named in iron3d.dll, on art that is there.
    blocks = {name: {k.lower(): v for k, v in props.items()}
              for name, props in mission.load_cfg(cfg).items()}
    rects = {name: (b.get("texture"), *(int(b.get(k, -1)) for k in
                                        ("offset_x", "offset_y", "width", "height")))
             for name, b in blocks.items() if name in TARGET_PANEL_SPRITES}
    roles = {d.role: d.bindings for d in resources.descriptors(game / "ui" / "game_resources.cfg")}
    pages = roles.get("textures", {})
    ui = NResArchive.open(game / "ui" / "ui.lib")
    entries = list(ui)
    cover: dict[str, tuple[str, float]] = {}
    for name, (page, x, y, w, h) in TARGET_PANEL_SPRITES.items():
        index = pages.get(page)
        if index is None or not 0 <= int(index) < len(entries):
            continue
        tex = texm.decode(ui.read(entries[int(index)]))
        opaque = sum(tex.rgba[(yy * tex.width + xx) * 4 + 3] > 16
                     for yy in range(y, y + h) for xx in range(x, x + w))
        cover[name] = (entries[int(index)].name, opaque / (w * h))
    named = all(name.encode() + b"\0" in iron for name in TARGET_PANEL_SPRITES)
    check("ui/hq.cfg: the target panel's nine sprites, on ui_menu3 and ui_menu",
          rects == TARGET_PANEL_SPRITES and named and b"ui/hq.cfg\0" in iron
          and len(cover) == len(TARGET_PANEL_SPRITES)
          and {page for page, _ in cover.values()} == {"ui_menu3.tex", "ui_menu1.tex"}
          and all(share > 0.2 for _, share in cover.values()),
          "; ".join(f"{n} {r[0]} ({r[1]}, {r[2]}) {r[3]}x{r[4]} "
                    f"{cover.get(n, ('?', 0.0))[1]:.0%} opaque" for n, r in rects.items())
          + f"; iron3d.dll names all nine: {named}, loaded by 0x1008f450 from ui/hq.cfg")

    # Where they go: the pushes before each quad draw.
    got, want = {}, {}
    for group, (start, calls) in TARGET_PANEL_QUADS.items():
        for side, follow in (("target", False), ("own", True)):
            got[(group, side)] = [(t, loaded, args[:5]) for t, loaded, args
                                  in _quad_calls(iron, start, len(calls), follow)]
            pick = side == "own"
            want[(group, side)] = [
                (0x1008F970, loaded, (*(own if pick else rect),
                                      colour[pick] if isinstance(colour, tuple) else colour))
                for loaded, rect, own, colour in calls]
    check("iron3d.dll: where the panels' frame, icons and six sectors are drawn",
          got == want,
          "; ".join(f"{g} {s}: " + ", ".join(f"{a[:4]}" for _, _, a in got[(g, s)])
                    for g, s in got)
          + "; the target panel at x 0-150, the own panel mirrored at 490-640, y 306-480")

    # The arcs, the range box and the texts' colours and places.
    body = iron[_va(iron, 0x10041EA0):_va(iron, 0x100426FB)]
    arcs = [body.find(b"\x68" + struct.pack("<I", TARGET_PANEL_COLOURS[k])
                      + b"\x68\x98\x01\x00\x00") for k in ("life", "energy")]
    cut = (b"\x68\x00\x00\x20\x42" in body and b"\x68\x00\x00\x17\x43" in body
           and body.count(b"\x6b\xc9\x5e") == 2 and b"\x8d\x57\x52" in body)
    box = _quad_calls(iron, 0x10042546, 1, False)
    texts = (all(b"\x68" + struct.pack("<I", TARGET_PANEL_COLOURS[k]) in body
                 for k in ("name", "range"))
             and b"\xc7\x44\x24\x14" + struct.pack("<I", TARGET_PANEL_COLOURS["danger"]) in body)
    check("iron3d.dll: the life and energy arcs, the name and the distance",
          all(a >= 0 for a in arcs) and cut and texts
          and box == [(0x1008F970, 4, (108, 440, 147, 456, -1, -0x1000000, 1, 0))]
          and b"\x68\xbc\x01\x00\x00" in body and b"\x68\x6f\x18\x00\x00" in body,
          f"life {TARGET_PANEL_COLOURS['life']:#x} and energy {TARGET_PANEL_COLOURS['energy']:#x} "
          f"arcs down to y 408, cut 40 wide from ui_menu3 at (151, 82 + 94 x (100 - pct) / 100); "
          f"the range box {box and box[0][2][:4]}; the name grey, 'Dangerous!' (6255) red, the "
          f"distance green at y 444")

    floats = {va: struct.unpack_from("<f", iron, _va(iron, va))[0] for va in TARGET_PANEL_FLOATS}
    check("iron3d.dll: the panel's figures -- view, centring, camera, frame, voices",
          all(math.isclose(floats[va], v, rel_tol=1e-6) for va, v in TARGET_PANEL_FLOATS.items())
          and b"\x68\x9a\x99\x99\x3f\x68\x00\x00\x00\x3f\x68\x00\x00\x96\x43" in iron
          and b"\x68\xf0\x07\x00\x00" in iron[_va(iron, 0x10041DC0):_va(iron, 0x10041DE0)],
          f"view (9, 315)-(137, 443), own (503, 315)-(631, 443); the name centred in 138 from "
          f"x 4 or 498, the distance in 31 from 112; half-angle {floats[0x100E5DB8]:.4f} rad, "
          f"field x {floats[0x100E5D10]}; frame scale below {floats[0x100E5E4C]} made 0.1, "
          f"held to {floats[0x100E5E48]}, x {floats[0x100E5CF0]}; low below "
          f"{floats[0x100E5C68]} and 20 life, every {floats[0x100E4BC4]} s; sectors x "
          f"{floats[0x100E59C8]}; the camera made with 300, 0.5, 1.2; the model drawn with 0x7f0")

    strings = resources.strings(iron)
    words = {k: strings.get(k) for k in TARGET_PANEL_STRINGS}
    letters = iron[_va(iron, 0x100762FD):_va(iron, 0x100763BB)]
    stored = [(0x10, 0x54), (0x10, 0x4D), (0x10, 0x4C), (0x11, 0x46), (0x11, 0x54), (0x11, 0x41),
              (0x11, 0x55), (0x12, 0x42), (0x12, 0x54), (0x12, 0x48), (0x12, 0x43)]
    size_letter = {1: "T", 2: "S", 3: "M", 4: "L"}
    chassis_letter = {"flying": "F", "walking": "S", "wheeled": "W", "tracked": "T"}
    prefixes = {}
    first = game / MISSION_01_DATA
    if first.exists():
        shop = units.Workshop(game)
        for o in mission.load(first).objects:
            leaf = o.path.split("\\")[-1].lower()
            if o.kind != mission.KIND_UNIT or leaf not in MISSION_01_PANEL_NAMES:
                continue
            found = [f for f in game.glob("UNITS/UNITS/**/*.dat") if f.name.lower() == leaf]
            unit = shop.describe(found[0]) if found else None
            if unit and unit.chassis:
                prefixes[leaf] = (size_letter.get(unit.size_class, "?")
                                  + chassis_letter.get(unit.chassis.locomotion, "?")
                                  + ("W" if unit.role == "warrior" else "?"))
    check("iron3d.dll: a unit's name is three letters, a count and a class word",
          words == TARGET_PANEL_STRINGS and b"%s-%d %s\0" in iron
          and b"\xb2\x53" in letters and b"\xb3\x57" in letters
          and all(b"\xc6\x44\x24" + bytes(pair) in letters for pair in stored)
          and prefixes == MISSION_01_PANEL_NAMES,
          "; ".join(f"{k} {v!r}" for k, v in words.items())
          + "; '%s-%d %s' (0x10075e63): size T S M L by property 0x201, chassis F S W T A U by "
          f"0x207, class B T W C H by Type (0x10076270); Mission 01 by size class, locomotion "
          f"and role: {prefixes}, as the recording names them")

    # What the arcs and the model read, in Control.dll and AniMesh.dll.
    control = (game / "Control.dll").read_bytes()

    def case(pid: int) -> bytes:
        index = control[_va(control, 0x1000E5E8) + pid - 1]
        target = struct.unpack_from("<I", control, _va(control, 0x1000E554) + 4 * index)[0]
        return control[_va(control, target):_va(control, target) + 40]

    life, energy, sectors = case(0x31), case(0x73), case(0x79)
    ani = (game / "AniMesh.dll").read_bytes()
    ramp = ani[_va(ani, 0x10006B7A):_va(ani, 0x10006BA6)]
    draw = ani[_va(ani, 0x10014C0A):_va(ani, 0x10014F40)]
    check("Control.dll, AniMesh.dll: the arcs read life, battery and sectors; nodes by life",
          b"\x8d\x88\x88\x05\x00\x00" in life and b"\xd9\x80\x90\x05\x00\x00" in life
          and b"\xd8\xb0\x8c\x05\x00\x00" in life
          and b"\x8b\x49\x38" in energy and b"\xba\x01\x00\x00\x00" in energy
          and b"\xff\x50\x10" in energy
          and b"\xc7\x01\x90\x33\x04\x10" in sectors and b"\xff\x50\x2c" in sectors
          and ramp == bytes.fromhex("c74424100000003f8b4c24108bd8c744241c0000803f8b7c241c"
                                    "c7442410000000bf890bc74424140000003f")
          and draw.startswith(b"\x25\x00\x02\x00\x00")
          and b"\xd9\x87\x24\x01\x00\x00" in draw and b"\xd8\x8e\x28\x02\x00\x00" in draw
          and b"\xd8\x86\x18\x02\x00\x00" in draw,
          "property 0x31 is +0x590 / +0x58c; 0x73 the device getter's id 1, the batteries' "
          "fill; 0x79 device manager slot 11 into 0x10043390, the six sectors; a mesh drawn "
          "with 0x200 gives each node (0.5, 0, 0, 1) + its life x (-0.5, 0.5, 0, 0)")


#: The attack task's named constants (``Behavior.dll:0x100165f0``-``0x100166d7``).
ATTACK_CONSTANTS = (
    "Attack_MinFightDistance", "Attack_DelFightDistance", "Attack_LeftRightRange",
    "Attack_MaxFireDistance", "Attack_MinAttackSpeedPercent",
    "Attack_DelAttackSpeedPercent", "Attack_MinNearingSpeedPercent",
    "Attack_DelNearingSpeedPercent", "Attack_Nearing_ChangeTrajectoryMinDelay",
    "Attack_Nearing_ChangeTrajectoryRandomDelay",
    "Attack_Fighting_ChangeTrajectoryMinDelay",
    "Attack_Fighting_ChangeTrajectoryRandomDelay",
)


def check_ai_fight(check, game: Path) -> None:
    """The attack's constants are compiled in; what Mission 01 leaves the AI to do."""
    probes = [n.encode() for n in ATTACK_CONSTANTS]
    naming: dict[str, set[str]] = defaultdict(set)
    files = 0
    for path in sorted(p for p in game.rglob("*") if p.is_file()):
        files += 1
        data = path.read_bytes()
        for probe in probes:
            if probe in data:
                naming[probe.decode()].add(path.name)
    where = {frozenset(v) for v in naming.values()}
    check("install: only Behavior.dll names the attack task's constants",
          len(naming) == len(ATTACK_CONSTANTS) and where == {frozenset({"Behavior.dll"})},
          f"{len(naming)} of {len(ATTACK_CONSTANTS)} Attack_* names occur, in "
          f"{sorted(set().union(*naming.values()))} of {files} files, so the compiled "
          f"defaults (0x10016250) hold: band 30+20, side 80, fire 200, speeds 0.7+0.3 and "
          f"0.8+0.2, re-pick 4+4 s")

    first = game / MISSION_01_DATA
    if not first.exists():
        return
    m = mission.load(first)
    clans = m.clans
    lib = objects.ObjectLibrary(game / "objects.rlb")
    held = profiles.load(game)
    units: dict[str, list[str]] = defaultdict(list)
    bots = {}
    buildings = []
    for o in m.objects:
        if not 0 <= o.clan_index < len(clans):
            continue
        clan = clans[o.clan_index]
        leaf = o.path.split("\\")[-1].lower()
        if o.kind == mission.KIND_BUILDING:
            buildings.append((clan.name, leaf))
            continue
        if o.kind != mission.KIND_UNIT:
            continue
        units[clan.name].append(leaf)
        found = [f for f in game.glob("UNITS/UNITS/**/*.dat") if f.name.lower() == leaf]
        if not found or clan.type != mission.CLAN_NEUTRAL:
            continue
        root = objects.load_unit(found[0]).components[0].ref.member.lower()
        record = lib.get(root)
        kind = (profiles.CHASSIS_TYPE[held[record.profile]["ChassisType"].value]
                if record is not None and record.profile in held else None)
        bots[leaf] = (root, kind, profiles.CHASSIS_SIZE[root[2]])
    capturers = sorted(n for n, (_, _, size) in bots.items() if size <= CAPTURE_SIZE)
    check("Mission 01: the two neutral bots fly; only helic is small enough to capture",
          bots == {"tut1_mf1.dat": ("r_m_02", "flying", 3),
                   "helic.dat": ("r_t_02", "flying", 1)}
          and capturers == ["helic.dat"],
          f"(chassis, locomotion, size class) {bots}; a capture needs class <= "
          f"{CAPTURE_SIZE} (Behavior.dll:0x100301a9); a Refit with no dock logs "
          f"'No Where to reX(for flyeing)' for both (0x1002e8c2)")

    places = _building_places(game)
    roots = {}
    for _, leaf in buildings:
        found = [f for f in game.glob("UNITS/BUILDS/**/*.dat") if f.name.lower() == leaf]
        if found:
            roots[leaf] = objects.load_unit(found[0]).components[0].ref.member.lower()
    marked = sum(1 for r in roots.values() if r in places
                 for a in places[r][1]
                 if a & (objmesh.PLACE_POD | objmesh.PLACE_DOCK))
    check("Mission 01: the only buildings are the player's bridge halves, with no pod or dock",
          buildings == [("Plr", "m_bridge.dat")] * 2 and roots == {"m_bridge.dat": "fr_m_brige"}
          and marked == 0,
          f"buildings {buildings} on {roots}; pod or dock places: {marked} "
          f"({'no hall-way graph' if 'fr_m_brige' not in places else 'graph read'}); "
          f"Search and capture skips bridges (0x10030859), so it roams and a Refit fails")


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

    # iron3d.dll's Type function reads +0x23 and +0x24 as the part's kind and
    # sub-kind: 9 and 33 make a turret, whose role byte then decides.
    kind_codes: dict[str, set[int]] = defaultdict(set)
    turret_sub: Counter[tuple[bool, bool]] = Counter()
    part_codes: dict[str, set[tuple[int, int, int, int]]] = defaultdict(set)
    for tree_path in research.trees(game):
        for item in research.read(tree_path).items:
            if not item.tail:
                continue
            for part in item.parts:
                part_codes[part.lower()].add((item.tail[1], item.tail[2], item.tail[4],
                                              item.tail[0]))
            entry = catalogue.get(item.parts[0]) if item.parts else None
            if not entry:
                continue
            kind_codes[entry.kind].add(item.tail[1])
            turret_sub[(item.tail[1] == objects.PART_KIND_UNIT
                        and item.tail[2] == objects.PART_SUB_TURRET,
                        entry.kind == "SHS" and entry.sub == "TUR")] += 1
    check(".trf: tail bytes +0x23 and +0x24 are the part's catalogue kind and sub-kind",
          {k: sorted(v) for k, v in kind_codes.items()}
          == {"BLD": [8], "SHS": [9], "ANM": [9], "AMM": [10], "DVC": [11], "WPN": [12]}
          and set(turret_sub) == {(True, True), (False, False)},
          "+0x23 by catalogue kind: "
          + ", ".join(f"{k} {sorted(v)}" for k, v in sorted(kind_codes.items()))
          + f"; +0x24 is 33 with +0x23 9 on exactly the {turret_sub[(True, True)]} "
          f"SHS:TUR item records, of {sum(turret_sub.values())}")

    ambiguous = sorted(p for p, c in part_codes.items() if len(c) > 1)
    robot_ok = robot_all = building_ok = building_all = 0
    building_off: Counter[tuple[int, int]] = Counter()
    for path in sorted(game.glob("UNITS/**/*.dat")):
        unit = objects.load_unit(path)
        members = [c.ref.member.lower() for c in unit.components]
        base = next(iter(part_codes.get(members[0], ())), None)
        if base is None:
            continue
        if unit.is_building:
            building_all += 1
            got = objects.part_type(base[0], base[1], base[2], base[3])
            building_ok += got == unit.kind
            if got != unit.kind:
                building_off[(base[1], unit.kind)] += 1
        elif members[0].startswith("r_") and any(m.startswith("e_tur") for m in members):
            turret = next(m for m in members if m.startswith("e_tur"))
            codes = next(iter(part_codes[turret]))
            robot_all += 1
            robot_ok += objects.part_type(*codes[:2], codes[2], codes[3]) == unit.kind
    ladder = set(controls.SCHEME_TYPES.values())
    check("UNITS: iron3d's Type function over the research codes gives the class word",
          not ambiguous and robot_all and robot_ok == robot_all
          and building_all - building_ok == sum(building_off.values())
          and set(sub for sub, _ in building_off) == {28}
          and set(objects.BUILDING_SUB_TYPES.values()) - ladder == {0x80001000}
          and set(objects.BUNKER_SIZE_TYPES.values()) <= ladder,
          f"a robot's turret item (kind, sub-kind, size, role) gives its .dat's class "
          f"word on {robot_ok}/{robot_all}; a building's base part on "
          f"{building_ok}/{building_all}, the rest ruins (sub-kind 28, which the "
          f"function leaves at 0): "
          + ", ".join(f"sub-kind {sub} class word {kind:#x} x{n}"
                      for (sub, kind), n in sorted(building_off.items()))
          + ".  Control: every building Type "
          "it gives is a BuildDat.lst scheme's but the bridge's 0x80001000")

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

    # A tree's category byte is state bits (MisLoad.dll:0x10002aa0, 0x10002c10):
    # 4 the item is in this tree, 1 it can be researched, 2 it has been.
    present, researched = 4, 2
    player = [p for p in research.trees(game) if re.search(r"(p|_pl)\.trf$", p.name, re.I)]
    specials_in: Counter[str] = Counter()
    hero_with_chassis = hero_trees = 0
    states: set[int] = set()
    for tree_path in research.trees(game):
        state = {}
        for item in research.read(tree_path).items:
            states.add(item.category)
            for part in item.parts:
                state[part.lower()] = item.category
        if state.get("e_tur_ht_02", 0) & present:
            hero_trees += 1
            hero_with_chassis += state.get("r_h_02", 0) != researched
        if tree_path in player:
            for tid in specials:
                specials_in[tid] += bool(state.get(tid, 0) & present)
    check(".trf: no player's tree holds a special turret, and no tree the hero's chassis",
          player and states == {0, 2, 4, 5, 7} and not any(specials_in.values())
          and hero_trees and hero_with_chassis == 0,
          f"state bytes {sorted(states)}; in the {len(player)} player trees (*p, *_pl) "
          f"the Transformer, Small tower and monster turrets are never present "
          f"{dict(specials_in)}; the hero turret is present in {hero_trees} trees and "
          f"its chassis r_h_02 in none of them, where it reads {researched}: "
          f"researched but not in the tree")

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
    # Call arguments only: a statement's trailer is a formula index, and read
    # as a variable name it once added RELOAD, FOLLOW, GETONBOARD and RANDOMGO
    # to the orders given (docs/15-behaviour.md).
    for path in behaviour.scripts(game):
        for h in behaviour.read(path).handlers:
            for n in h.nodes:
                if n.calls:
                    used.update(name for o in n.operands
                                if re.fullmatch(r"ORDER_(?:ROBOT|BUILDING)_[A-Z]+",
                                                name := behaviour.name_at(tables, o)))
    never = sorted(set(packages.ORDERS) - set(used))
    check("*.scr: patrol, attack and capture are the orders mission scripts give most",
          used and set(used) <= set(packages.ORDERS)
          and [n for n, _ in used.most_common(3)] == [
              "ORDER_ROBOT_PATROL", "ORDER_ROBOT_ATTACK", "ORDER_ROBOT_CAPTURE"]
          and never == ["ORDER_BUILDING_CHARGE", "ORDER_ROBOT_FOLLOW",
                        "ORDER_ROBOT_GETONBOARD", "ORDER_ROBOT_LEAVE",
                        "ORDER_ROBOT_RANDOMGO", "ORDER_ROBOT_RELOAD",
                        "ORDER_ROBOT_REPARE", "ORDER_ROBOT_STAYGROUND"],
          ", ".join(f"{n[6:]} {k}" for n, k in used.most_common())
          + f"; never: {', '.join(never)}")

    # The status line: a switch over the head order, each case loading a string id.
    by_order = _status_switch(iron) or {}
    wanted = {o: packages.STATUS_FIRST + i for o, i in packages.STATUS_BY_ORDER.items()}
    unknown = packages.STATUS_FIRST + len(packages.STATUS) - 1
    named = ", ".join(f"{o} {table.get(s)!r}" for o, s in sorted(by_order.items())
                      if s != unknown)
    rest = sorted(o for o, s in by_order.items() if s == unknown)
    check("iron3d.dll: the status line's string is picked by the head order",
          len(by_order) == 25 and by_order[0] == packages.STATUS_FIRST
          and all(by_order[o] == wanted.get(o, unknown) for o in range(1, 25)),
          f"the one 25-case switch on esi (iron3d.dll:0x10076f90) loads, by order: {named}; "
          f"orders {', '.join(map(str, rest))} load 'unknown'")

    # The wingman menu: the command that opens the second table, as the files name it.
    described = controls.commands(game).get("CMD_JAMES_WINGMAN_MENU", "")
    bound = sorted({b.chord for name in ("addition.man", "ui_other.man")
                    for b in controls.bindings(game / name)
                    if b.command == "CMD_JAMES_WINGMAN_MENU"})
    check("Command.dsc: the second menu's command activates the wingman menu, on the tilde",
          described.lower() == "activate wingman menu" and bound == ["SCAN_TILDA"]
          and controls.CMD.get("CMD_JAMES_WINGMAN_MENU") == 740,
          f"CMD_JAMES_WINGMAN_MENU (740) is described {described!r} and bound to "
          f"{', '.join(bound)} in addition.man and ui_other.man; its handler opens the "
          f"selector whose menu reads the 7-row second table")

    # Nothing reads a profile's task flags by name: only their binder and their values name them.
    naming: dict[str, set[str]] = defaultdict(set)
    probes = [f.encode() for f in packages.TASK_FLAGS] + [b"Patrol_Attack_Range",
                                                          b"Build_BuildDistance"]
    for path in sorted(p for p in game.iterdir() if p.is_file()):
        data = path.read_bytes()
        for probe in probes:
            if probe in data:
                naming[probe.decode()].add(path.name)
    flags_where = {frozenset(naming[f]) for f in packages.TASK_FLAGS}
    check("install: only Behavior.dll and behpsp.res name a task flag or the two unread constants",
          flags_where == {frozenset({"Behavior.dll", "behpsp.res"})}
          and naming["Patrol_Attack_Range"] == naming["Build_BuildDistance"] == {"Behavior.dll"},
          f"all {len(packages.TASK_FLAGS)} Task_* names occur in "
          f"{', '.join(sorted(next(iter(flags_where))))} and in no other top-level file of "
          f"{sum(1 for p in game.iterdir() if p.is_file())}; Patrol_Attack_Range and "
          f"Build_BuildDistance only in {', '.join(sorted(naming['Patrol_Attack_Range']))} -- "
          f"no module looks them up by name")


def _status_switch(image: bytes) -> dict[int, int] | None:
    """``iron3d.dll``'s status-line switch: order -> the string id its case loads.

    The switch is ``cmp esi, 0x18; ja default; jmp [esi*4 + table]`` and every
    case ``mov esi, [eax]; mov edx, id``.  Returns None if the pattern is absent.
    """
    at = image.find(b"\x83\xfe\x18\x0f\x87")
    if at < 0 or image[at + 9:at + 12] != b"\xff\x24\xb5":
        return None
    sections, _ = resources._sections(image)
    lfanew = struct.unpack_from("<I", image, 0x3C)[0]
    base = struct.unpack_from("<I", image, lfanew + 24 + 28)[0]
    table = struct.unpack_from("<I", image, at + 12)[0] - base
    out = {}
    for order in range(25):
        case = struct.unpack_from("<I", image, resources._offset(sections, table + 4 * order))[0]
        off = resources._offset(sections, case - base)
        if image[off:off + 3] != b"\x8b\x30\xba":
            return None
        out[order] = struct.unpack_from("<I", image, off + 3)[0]
    return out


#: The wingman menu's acknowledgement voices: five, in a plain, an ``_S`` and a
#: ``_B`` set, picked by the last ordered unit's record ``+0x30``.
ACKNOWLEDGE_VOICES = ("VOICE_ACKNOWLEDGE", "VOICE_AFFIRMATIVE", "VOICE_YES_SIR", "VOICE_OK",
                      "VOICE_EXECUTE")


def check_wingman(check, game: Path) -> None:
    """The wingman menu from first person: the orders' numbers, the digit keys, the voices."""
    names = ("ORDER_ROBOT_STAYGROUND", "ORDER_ROBOT_FOLLOW", "ORDER_ROBOT_SEARCH",
             "ORDER_ROBOT_ATTACK", "ORDER_ROBOT_RELOAD", "TARGET_BY_LOGIC_ID", "TARGET_BY_TYPE",
             "TARGET_NOT_DEFINED", "INSERT_ORDER_REPLACE")
    consts = {v.name: int(v.default, 0) for v in behaviour.variables(game) if v.name in names}
    check("varset.var: the wingman rows' orders, target kinds and the replacing insert",
          consts == {"ORDER_ROBOT_STAYGROUND": packages.STAYGROUND,
                     "ORDER_ROBOT_FOLLOW": packages.FOLLOW,
                     "ORDER_ROBOT_SEARCH": packages.SEARCH,
                     "ORDER_ROBOT_ATTACK": packages.ATTACK,
                     "ORDER_ROBOT_RELOAD": packages.RELOAD,
                     "TARGET_BY_LOGIC_ID": packages.TARGET_BY_LOGIC_ID,
                     "TARGET_BY_TYPE": packages.TARGET_BY_TYPE,
                     "TARGET_NOT_DEFINED": packages.TARGET_NOT_DEFINED,
                     "INSERT_ORDER_REPLACE": 3},
          f"{ {k: hex(v) for k, v in consts.items()} }: what the dispatcher "
          f"(iron3d.dll:0x10079230) writes for Standby, Follow me, the searches, Attack and "
          f"Refit, each given with insert mode 3")

    # The character handler: `lea eax, [ebp-0x13]; cmp eax, 0x7e; ...; mov dl,
    # [eax + index]; jmp [edx*4 + cases]`.  Group the 127 characters by case.
    iron = (game / "iron3d.dll").read_bytes()
    sections, _ = resources._sections(iron)
    lfanew = struct.unpack_from("<I", iron, 0x3C)[0]
    base = struct.unpack_from("<I", iron, lfanew + 24 + 28)[0]
    at = iron.find(b"\x8d\x45\xed\x83\xf8\x7e")
    by_case: dict[int, list[int]] = defaultdict(list)
    if at >= 0 and iron[at + 16:at + 20] == b"\x33\xd2\x8a\x90" \
            and iron[at + 24:at + 27] == b"\xff\x24\x95":
        index = struct.unpack_from("<I", iron, at + 20)[0]
        cases = struct.unpack_from("<I", iron, at + 27)[0]
        default = at + 16 + struct.unpack_from("<i", iron, at + 12)[0]  # ja's file offset
        off = resources._offset(sections, index - base)
        for n, case in enumerate(iron[off:off + 127]):
            target = struct.unpack_from(
                "<I", iron, resources._offset(sections, cases + 4 * case - base))[0]
            by_case["default" if resources._offset(sections, target - base) == default
                    else target].append(0x13 + n)
    groups = sorted((chars for key, chars in by_case.items() if key != "default"), key=len)
    digits = list(range(ord("1"), ord("9") + 1))
    check("iron3d.dll: the character handler gives '1'-'9' one case, the wingman selector's",
          groups == [[0x13], [0x1B], [0x91], digits] and len(by_case.get("default", [])) == 115,
          f"of the 127 characters from 0x13 its index table sends '1'-'9' to one case, "
          f"0x13, 0x1b and 0x91 to one each, and {len(by_case.get('default', []))} to the "
          f"default; the digit case (0x100710fa) calls the selector's pick (0x1006dd00) or "
          f"its order (0x1006df80)")

    found = resources.descriptors(game / "ui" / "game_resources.cfg")
    voices = next((d for d in found if d.role == "voices"), None)
    library = resources.locate(game, voices.library) if voices else None
    held = {e.name.lower() for e in NResArchive.open(library)} if library else set()
    sets = {suffix: [voices.bindings.get(v + suffix) if voices else None
                     for v in ACKNOWLEDGE_VOICES] for suffix in ("", "_S", "_B")}
    files = [f for bound in sets.values() for f in bound]
    check("game_resources.cfg: three sets of five acknowledgement voices, all in voices.lib",
          all(files) and len(set(files)) == 15 and all(f.lower() in held for f in files)
          and all(f.endswith("_s.wav") for f in sets["_S"])
          and all(f.endswith("_b.wav") for f in sets["_B"]),
          "; ".join(f"{s or 'plain'}: {', '.join(map(str, b))}" for s, b in sets.items())
          + f"; {sum(1 for f in files if f and f.lower() in held)} of 15 in "
          f"{voices.library if voices else None}")


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

    # 4b. a builder sets off with Building_Cost and pays the full sum, unclamped
    starts = sorted({n for _, n in held[objects.TYPE_BUILDER]})
    poorest = min(starts) if starts else 0.0
    dearer = sorted(n for n, cost in first.items() if cost > poorest)
    cheaper = sorted(n for n, cost in first.items() if cost <= poorest)
    check("objects.dlb: a builder placed with 200 ore goes into debt on all but a generator",
          starts and poorest >= profiles.BUILDING_COST and len(dearer) == 11
          and cheaper == ["Generator"],
          f"placed builders start with {', '.join(f'{s:g}' for s in starts)} ore, each at least "
          f"Building_Cost ({profiles.BUILDING_COST:g}), so they set off at once "
          f"(Behavior.dll:0x10028ff0); {len(dearer)} of {len(first)} first buildings cost more "
          f"than {poorest:g}, and the property setter stores the negative result "
          f"(0x100269e0); only {', '.join(cheaper)} ({first['Generator']:g}) is paid in full")

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

    # The mineral lodes are the trailer's records: every placed mine sits on some.
    lodes = total = found = typed = 0
    missions_with = 0
    amounts: set[float] = set()
    mines = Counter()
    others = Counter()
    for path in sorted(game.glob("MISSIONS/**/data.tma")):
        m = mission.load(path)
        got = packages.mineral_lodes(m)
        total += 1
        missions_with += bool(got)
        lodes += len(got)
        found += sum(lode.found for lode in got)
        typed += sum(lode.type_word == packages.MINERALS for lode in got)
        amounts |= {lode.amount for lode in got}
        for o in m.objects:
            kind = o.properties.get("Type")
            if kind is None or not (int(kind.value) & 0xFFFFFFFF) >> 31 or not got:
                continue
            near = any(math.hypot(lode.x - o.position[0], lode.y - o.position[1])
                       <= packages.MINE_LODE_RADIUS for lode in got)
            is_mine = (int(kind.value) & 0xFFFFFFFF) == 0x80000004
            (mines if is_mine else others)[near] += 1
    decades = {round(math.log10(a)) for a in amounts if a > 0}
    check("data.tma: the trailer's records are mineral lodes, and every mine sits by one",
          lodes and mines[True] and not mines[False] and others[True] * 10 < others[False]
          and typed > lodes // 2
          and all(abs(a / 10 ** round(math.log10(a)) - 1) < 1e-3 for a in amounts)
          and min(decades) >= 4,
          f"{lodes} records in {missions_with} of {total} missions; all {mines[True]} placed "
          f"mines lie within {packages.MINE_LODE_RADIUS:g} of one (M_Task_Mine takes the "
          f"amounts of those), against {others[True]} of {sum(others.values())} other "
          f"buildings; {typed} carry the type 0x10001000 Search minerals asks for; the "
          f"amounts are 10^n or 10^n - 1 for n {min(decades)}..{max(decades)}; {found} start "
          f"found, which a minerals search skips")


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


def _planned_run(parsed: control.Controller, frm: int, to: int) -> list[int] | None:
    """The states the planner queues from ``frm`` toward ``to``, as it searches.

    The search is rooted at the target: every state starts at the cost of its
    own edge into the target -- the target's self-edge too -- and each state
    settled passes its cost on to the states with an edge into it.  A state's
    predecessor in that search is its next hop, and the run follows the hops
    until it reaches the target (``Control.dll:0x100019d0``, ``0x10004f50``).
    """
    n = len(parsed.states)
    dist = [parsed.cost(to, j) for j in range(n)]
    hop = [to] * n
    settled = [False] * n
    while True:
        open_states = [j for j in range(n) if not settled[j] and dist[j] < control.NO_EDGE]
        if not open_states:
            return None
        pick = min(open_states, key=lambda j: dist[j])
        settled[pick] = True
        if pick == frm:
            break
        for k in range(n):
            through = dist[pick] + parsed.cost(pick, k)
            if not settled[k] and through < dist[k]:
                dist[k], hop[k] = through, pick
    run, state = [], frm
    while True:
        state = hop[state]
        run.append(state)
        if state == to or len(run) > n:
            return run


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
    # 2b. the kill states re-enter themselves every step while their code is held
    looping = Counter()
    for _blob, parsed in buildings.values():
        for index, state in enumerate(parsed.states):
            if state.request not in (2, 8, 10):
                continue
            looping[(state.anchor and bool(state.mode & control.STATE_FIXED)
                     and state.flags == 0 and state.length,
                     _planned_run(parsed, index, index) == [index])] += 1
    steps = {length for (length, _), _n in looping.items()}
    check("fortif.rlb: a kill state is a fixed anchor whose way back is its own self-edge",
          looping and set(looping) == {(250.0, True)}
          and sum(looping.values()) == 3 * len(buildings),
          f"all {sum(looping.values())} states asked for by code 2, 8 or 10 on "
          f"{len(buildings)} controllers are anchors fixed at "
          f"{', '.join(f'{s:g}' for s in steps)} ms "
          f"with their boxes off, and the planner's way back from each "
          f"(Control.dll:0x10004f50 over 0x100019d0) is its self-edge alone -- so while the "
          f"code is held the state is entered, and its kill run, every step: four times a second")

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
    check("controls: three commands the tables send are numbered above 16",
          len(outside) == 3,
          f"{sum(1 for r in rows if r.dispatched)}/{len(rows)} rows send a command "
          f"in {controls.DISPATCHED.start}..{controls.DISPATCHED.stop - 1}, the span once "
          f"read as Control.dll's dispatch (it is IDeviceManager's getter); the rest "
          + ", ".join(f"{n} x{c}" for n, c in sorted(outside.items())))

    walk = [r for r in rows if r.command == "MCMD_WALK_F"]
    tables = {name for name in controls.TABLES
              for r in controls.table(game / name) if r.command == "MCMD_WALK_F"}
    check("controls: the forward walk is sent by every table",
          len(walk) == 6 and len(tables) == len(controls.TABLES),
          f"MCMD_WALK_F is sent on {len(walk)} rows, two in each of "
          f"{len(tables)} tables -- and no module compares against its number: "
          f"World3D.dll's range table reaches it")

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

    # MCMD_ROTATE_Z's value is the spin's z (World3D.dll:0x1000fe77 -> IControl
    # slot 4, Control.dll:0x10004440), so the developers' own labels give +z's sense.
    turns = sorted({(r.note, r.value) for t in machines for r in t
                    if r.command == "MCMD_ROTATE_Z" and r.pressed})
    check("m1.tbl, m2.tbl: turning left is a positive spin, turning right a negative one",
          turns == [("OBJ_TURN_LEFT", 0.7), ("OBJ_TURN_RIGHT", -0.7)],
          f"MCMD_ROTATE_Z press rows {turns}: +z turns a machine left, which is why a "
          f"mouse count's -z turn (the pending turn is negated) turns it right")

    plus = next(r for r in hero if r.key == "SCAN_G_PLUS")
    command, held, runs = 0.0, 0.0, 0
    while command < 1.0 and runs < 1000:        # an update every 50 ms, say
        command, held, runs = plus.ramped(command, held), held + 50.0, runs + 1
    ini = settings.sections(game / settings.DISPLAY_FILE).get("CS", {})
    sensitivity = {k: int(ini.get(k, "-1")) for k in controls.INPUT_SETTINGS}
    check("Iron_3D.ini: the shipped mouse and joystick sensitivities hand World3D 1.0",
          sensitivity == {"MOUSE_SENS": 100, "JOY_SENS": 100, "MOUSE_REV_Y": 0, "JOY_REV_Y": 0}
          and runs == 31,
          f"{sensitivity}: iron3d.dll hands MOUSE_SENS x {controls.SENSITIVITY_SCALE} to "
          f"World3D.dll setting {controls.INPUT_SETTINGS['MOUSE_SENS']:#x}, the mouse "
          f"filter's multiplier, and both signs stay +1.  Control: a held keypad + takes "
          f"{runs} runs of the input update from a stopped cruise to full")

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

    # The camera builds its frame from the two points (Control.dll:0x100234c0):
    # TargetDirect's vector looks, CameraCenter's position is the eye and its
    # vector is the up that the side axis is crossed from.
    ups: Counter[tuple[bool, tuple[float, ...]]] = Counter()
    looks: Counter[tuple[float, ...]] = Counter()
    hung_up: list[str] = []
    for name in sorted(names):
        stem = name[:-4]
        if not name.endswith(".ctl") or stem + ".cpt" not in names:
            continue
        c = control.parse(turrets.read_name(name))
        cp = objmesh.parse_control_points(turrets.read_name(stem + ".cpt"))
        mount = next((p for p in c.components if p.type_id == control.TURRET_TYPE), None)
        upright = bool(mount and mount.flags & control.MOUNT_UPRIGHT)
        for comp in c.components:
            if comp.type_id == control.CAMERA_TYPE and comp.entries:
                chan = c.channels[comp.entries[0]]
                up = tuple(round(v, 3) for v in cp[chan.point - 1].direction)
                ups[(upright, up)] += 1
                looks[tuple(round(v, 3) for v in cp[chan.point].direction)] += 1
                if not upright and up[2] > 0:
                    hung_up.append(stem)
                break
    check("turrets.rlb: CameraCenter's vector is the eye's up, TargetDirect's its look",
          looks == Counter({(0.0, 1.0, 0.0): sum(looks.values())})
          and {u for (upright, u) in ups if upright} == {(0.0, 0.0, 1.0)}
          and ups[(False, (0.0, 0.0, -1.0))] > len(hung_up),
          f"TargetDirect is +y on all {sum(looks.values())} cameras; CameraCenter is "
          + ", ".join(f"{u} on {n} {'upright' if up else 'hung'}"
                      for (up, u), n in sorted(ups.items()))
          + f" -- the two hung turrets that keep +z are {', '.join(hung_up)}")

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

    # Every robot turret: a value is a frame, the keys are even, and the start is
    # level -- so, with the engine's slerp between keys (Ngi32.dll:0x10014630),
    # the aim is linear in the channel's value.
    library = objects.ObjectLibrary(game / "objects.rlb")
    yaw_ok = level_ok = turrets_seen = 0
    sweeps: Counter[tuple[float, float]] = Counter()
    for rid in sorted(r for r in library.records if r.lower().startswith("e_tur")):
        record = library.get(rid)
        c = control.parse(turrets.read_name(record.slot_with_suffix("ctl").member))
        mount = c.components[0]
        if mount.type_id != control.TURRET_TYPE or len(mount.entries) != 2:
            continue
        y, p = (c.channels[i] for i in mount.entries)
        member = record.slot_with_suffix("msh").member
        model = objmesh.parse(turrets.read_name(member), member)

        def pose(node: int, frame: int, model=model) -> objmesh.Pose:
            def local(k: int) -> objmesh.Pose:
                track = model.track(k)
                return model.keys[track[frame]].pose if track else model.local_pose(k)
            out, parent = local(node), model.nodes[node].parent
            while parent != objmesh.NO_PARENT:
                out, parent = objmesh.compose(local(parent), out), model.nodes[parent].parent
            return out

        turrets_seen += 1
        faces = []
        for f in range(int(y.first), int(y.last) + 1):
            x, fy, _ = objmesh.quaternion_rotate(pose(y.node, f)[1], (0.0, 1.0, 0.0))
            faces.append(math.degrees(math.atan2(x, fy)))
        yaw_ok += len(faces) == 5 and all(
            abs((a - b + 180) % 360 - 180) < 1.0
            for a, b in zip(faces, (180, -90, 0, 90, 180), strict=True))
        ups = []
        for f in range(int(p.first), int(p.last) + 1):
            x, fy, z = objmesh.quaternion_rotate(pose(p.node, f)[1], (0.0, 1.0, 0.0))
            ups.append(math.degrees(math.atan2(z, math.hypot(x, fy))))
        even = len(ups) == 3 and abs((ups[1] - ups[0]) - (ups[2] - ups[1])) < 0.5
        start = ups[0] + p.initial * (ups[-1] - ups[0])
        level_ok += even and abs(start) < 1.0
        sweeps[(round(ups[0], 1), round(ups[-1], 1))] += 1
    check("turrets.rlb: every turret's aim keys are even, and every one starts level",
          turrets_seen and yaw_ok == turrets_seen == level_ok,
          f"{yaw_ok}/{turrets_seen} yaw channels face 180, -90, 0, 90 and 180 degrees on "
          f"their five frames (so 0.25 looks to -x and 0.75 to +x); {level_ok} pitch "
          f"channels rise in two equal steps and sit within a degree of level at their "
          f"start value; pitch limits (degrees) "
          + ", ".join(f"{lo:+g}..{hi:+g} x{n}" for (lo, hi), n in sorted(sweeps.items())))

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
          f"{len(writers)}/{len(writers)} nodes that write carry a formula or an "
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
          f"{behaviour.MARKER_TAG}, the label they jump to, sits at the outermost "
          f"depth on {outer}/{marker}")

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

    check_behaviour_flow(check, game, scripts, table)


def check_behaviour_flow(check, game: Path, scripts, table) -> None:
    """What the executor does with a node, measured on the scripts.

    The kinds, the relation word, the formula index and the jump targets are
    read from ai.dll's executor (0x10012020); these hold the corpus to them.
    """
    B = behaviour
    nodes = [n for s in scripts for h in s.handlers for n in h.nodes]
    ifs = [n for n in nodes if not n.calls and n.opcode in B.BINARY]
    rest = [n for n in nodes if n.calls or n.opcode not in B.BINARY]
    check("behaviour: head[3] is the node's kind, and only an if uses the relation word",
          ifs and all(n.kind == B.IF for n in ifs)
          and all(n.opcode == B.VARIADIC for n in rest)
          and not any(n.kind == B.IF for n in rest),
          f"all {len(ifs)} comparisons are kind {B.IF}, and the fifth word is "
          f"{B.VARIADIC} on all {len(rest)} other nodes -- it is the relation "
          f"({' '.join(B.RELATIONS)}), not an opcode")

    kinds = Counter((table[n.operands[0]].type, table[n.operands[1]].type) for n in ifs)
    mixed = sum(v for (a, b), v in kinds.items() if a != b)
    check("behaviour: a comparison compares like with like",
          ifs and not mixed,
          f"{kinds[('DWORD', 'DWORD')]} compare two DWORDs and "
          f"{kinds[('float', 'float')]} two floats; {mixed} mix them, so which "
          f"path the executor takes -- by the first operand's type -- never matters")

    fml_count = matched = in_range = first = total = 0
    missing = []
    for s in scripts:
        fml = s.source.with_suffix(B.FORMULAS)
        if not fml.exists():
            missing.append(s.source.name)
            continue
        exprs = B.formulas(s.source)
        fml_count += len(exprs)
        trailers = [n.formula for h in s.handlers for n in h.nodes if n.formula != B.NULL]
        total += len(trailers)
        matched += len(trailers) == len(exprs)
        for i, t in enumerate(trailers):
            if 0 <= t < len(exprs):
                in_range += 1
                first += exprs[t] == exprs[i] and exprs.index(exprs[i]) == t
    check("behaviour: the trailer indexes the script's own .fml",
          scripts and not missing and matched == len(scripts) and in_range == total
          and first == total and fml_count == total,
          f"{matched}/{len(scripts)} scripts carry exactly as many FUNCTION lines as "
          f"nodes with a trailer ({fml_count} and {total} in all); {in_range}/{total} "
          f"trailers index their file, and on {first} the formula is the node's own "
          f"line or the first identical one before it -- the compiler shared them")

    gotos = labels = targeted = inside = 0
    switches = []
    bad = []
    for s in scripts:
        for h in s.handlers:
            marks = {i for i, n in enumerate(h.nodes) if not n.calls and n.kind == B.LABEL}
            labels += len(marks)
            aimed = set()
            for i, n in enumerate(h.nodes):
                if n.calls:
                    continue
                if n.kind == B.GOTO:
                    gotos += 1
                    if n.target in marks:
                        aimed.add(n.target)
                    else:
                        bad.append(f"{s.source.name}:{h.name}:{i}")
                if n.kind == B.SWITCH:
                    ok = 0 <= n.target < len(s.handlers)
                    switches.append((ok, s.handlers[n.target].name if ok else "", h.name))
            targeted += len(aimed)
            depth = 0
            for n in h.nodes:
                if n.closes:
                    depth = max(0, depth - 1)
                if not n.calls and n.kind == B.CONST and depth:
                    inside += 1
                if n.opens:
                    depth += 1
    check("behaviour: a goto lands on a label in its own handler",
          gotos and not bad,
          f"{gotos - len(bad)}/{gotos} tag-{B.GOTO} operands are the node index of a "
          f"tag-{B.LABEL} node in the same handler; {targeted} of the {labels} labels "
          f"are aimed at -- the operand read as fPry was node 26")

    reached = {name for ok, name, _ in switches if ok}
    carried = Counter(h.name for s in scripts for h in s.handlers
                      if not h.problem and h.name not in B.EVENTS)
    check("behaviour: a jump to a handler names one, and those handlers are reached no other way",
          switches and all(ok for ok, _, _ in switches) and reached == set(carried),
          f"{len(switches)}/{len(switches)} tag-{B.SWITCH} operands index a handler of "
          f"the same script, and the {len(reached)} they reach are exactly the handlers "
          f"that are neither events nor problems -- "
          + ", ".join(f"{n} in {k}" for n, k in sorted(carried.items()))
          + " scripts")

    consts = sum(1 for n in nodes if not n.calls and n.kind == B.CONST)
    check("behaviour: some constants sit inside a block the executor does not guard",
          consts and 0 < inside < consts,
          f"{inside} of the {consts} kind-{B.CONST} constants sit inside an if; "
          f"ai.dll:0x100121e2 writes them without testing the condition, unlike "
          f"every other kind")

    calls = [n for n in nodes if n.calls]
    over = [n for n in calls if len(n.operands) > B.ARGUMENTS[n.function]]
    longest = defaultdict(int)
    for n in calls:
        longest[n.function] = max(longest[n.function], len(n.operands))
    exact = sum(1 for f, k in longest.items() if k == B.ARGUMENTS[f])
    void = [n for n in calls if n.function in B.VOID_FUNCTIONS
            and n.destination != B.NULL]
    check("behaviour: a call passes what its handler reads",
          calls and all(n.function == 0 for n in over)
          and exact >= len(longest) - 2 and not void,
          f"on {exact} of the {len(longest)} functions the scripts call, the longest "
          f"call passes exactly the operands ai.dll's handler reads; "
          f"{len(over)} calls pass more, all to function 0, whose handler reads none; "
          f"no call to the {len(B.VOID_FUNCTIONS)} handlers that never write a "
          f"result names a destination")

    # --- the building bit and the destroyed sentinel -------------------------
    building = [v for v in table if v.name.startswith("BUILDING_")]
    other_types = [v for v in table if v.name.startswith(("ROBOT_", "RESOURCE_"))
                   or v.name in ("CLASS_ROBOT", "CLASS_ANIMAL")]
    as_int = {v.name: int(v.default, 0) for v in building + other_types
              if v.default and v.default[0].isdigit()}
    class_bit = next((v for v in table if v.name == "CLASS_BUILDING"), None)
    placed = Counter()
    by_stem = defaultdict(list)
    loaded = []
    for d in gamedir.missions(game):
        m = mission.load(d / "data.tma")
        loaded.append(m)
        for o in m.objects:
            flagged = bool(o.logical_id & B.CLASS_BUILDING)
            placed[(o.kind == mission.KIND_UNIT, flagged)] += 1
        for c in m.clans:
            by_stem[c.ai_script.replace("\\", "/").split("/")[-1].lower()].append(m)
    check("behaviour: CLASS_BUILDING is the top bit of a building's type and logical id",
          class_bit is not None and int(class_bit.default, 0) == B.CLASS_BUILDING
          and all(as_int[v.name] & B.CLASS_BUILDING for v in building)
          and not any(as_int[v.name] & B.CLASS_BUILDING for v in other_types)
          and placed[(True, True)] == 0 and placed[(False, False)] == 0,
          f"varset.var declares CLASS_BUILDING {B.CLASS_BUILDING:#x}, all "
          f"{len(building)} BUILDING_ types carry it and none of the "
          f"{len(other_types)} robot, resource and class words does; across the "
          f"missions {placed[(False, True)]}/{placed[(False, True)] + placed[(False, False)]} "
          f"placed objects that are not units carry it in their logical id and "
          f"{placed[(True, True)]}/{placed[(True, True)] + placed[(True, False)]} units do")

    named = found = unused = 0
    chance = []
    for s in scripts:
        users = by_stem.get(s.source.stem.lower(), [])
        for h in s.handlers:
            for n in h.nodes:
                if n.calls or n.kind != B.CONST or not n.head[2] & B.CLASS_BUILDING:
                    continue
                word = n.head[2] & 0xFFFF_FFFF
                if not users:
                    unused += 1
                    continue
                named += 1
                found += any(o.kind == mission.KIND_BUILDING
                             and o.logical_id & 0xFFFF_FFFF == word
                             for m in users for o in m.objects)
                chance.append(sum(any(o.kind == mission.KIND_BUILDING
                                      and o.logical_id & 0xFFFF_FFFF == word
                                      for o in m.objects) for m in loaded) / len(loaded))
    control = sum(chance) / len(chance) if chance else 1
    check("behaviour: a flagged literal is a building of the mission that runs the script",
          named and found >= named - 1 and control < 0.6,
          f"{found}/{named} flagged literals in scripts a mission names are the "
          f"logical id of a building that mission places, against "
          f"{100 * control:.0f}% of missions at large; {unused} more sit in scripts "
          f"no mission names")

    compared = against = 0
    for s in scripts:
        sentinel = {n.destination for h in s.handlers for n in h.nodes
                    if not n.calls and n.kind == B.CONST and n.head[2] in (4094, B.DESTROYED)}
        for h in s.handlers:
            last: dict[int, str] = {}
            for n in h.nodes:
                if not n.calls and n.kind == B.IF:
                    for x, y in (n.operands, n.operands[::-1]):
                        if x in sentinel and last.get(x, "sentinel") == "sentinel":
                            compared += 1
                            against += last.get(y) == "fn52"
                if n.destination != B.NULL:
                    last[n.destination] = (
                        f"fn{n.function}" if n.calls else
                        "sentinel" if n.kind == B.CONST and n.head[2] in (4094, B.DESTROYED)
                        else "other")
    check("behaviour: 65534 is compared with an object's owner",
          compared and against == compared,
          f"{against}/{compared} comparisons against a variable holding 65534 or "
          f"4094 test it against function 52's answer, the owner word of a logical "
          f"id, which a destroyed object sets to {B.DESTROYED:#x}")


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
    small = [o for path in save.saves(game) for o in save.read(path).objects
             if o.counts and len(o.chunks) == 3]
    grown = sum(o.size - save.WORLD_RECORD == o.chunks[2].size - save.CONTROL_SMALL
                and (o.chunks[2].size - save.CONTROL_SMALL) % save.WORLD_STEP == 0
                for o in small)
    rounds = [o for o in small if o.kind == save.KIND_ROUND]
    check("saves: the world record's step tells furniture from rounds in flight",
          steps and max(scenery) <= 1 and min(other) >= 1
          and grown == len(small) and all(o.kind in save.SMALL_KINDS for o in small),
          f"{scenery.count(0)}/{len(scenery)} scenery records take no step and "
          f"none takes more than one; all {len(other)} others -- rounds in "
          f"flight, {len(rounds)} of the {len(small)} three-chunk records -- take "
          f"at least one, and {varying} of {len({n for n, _ in steps})} names "
          f"appear with two counts; on {grown}/{len(small)} the step is the "
          f"control chunk growing past {save.CONTROL_SMALL} bytes")

    rising = counted = agree = 0
    for path in save.saves(game):
        s = save.read(path)
        blob = path.read_bytes()
        order = []
        heads = {o.offset + save.RECORD_ARCHIVE: o for o in s.objects}
        for r in sorted(s.references, key=lambda r: r.offset):
            if r.field != save.MEMBER_AT[1] or not r.member.startswith(save.SCENERY):
                continue
            at = r.offset + save.WORLD_INDEX_AT
            if at + 2 > len(blob):
                continue
            value = struct.unpack_from("<H", blob, at)[0]
            if value != save.NO_INDEX:
                order.append(value)
            # On a 450-byte record that offset is the next record's id.
            following = heads.get(r.offset + save.WORLD_RECORD)
            agree += following is not None and following.serial & 0xFFFF == value
        if order:
            counted += 1
            rising += all(b > a for a, b in zip(order, order[1:], strict=False))
    check("saves: a scenery record carries an index that rises in file order",
          counted and rising == counted and agree,
          f"{rising}/{counted} saves with scenery hold a uint16 at "
          f"+{save.WORLD_INDEX_AT:#x} that only increases down the file; on "
          f"{agree} records it is the serial of the next record's id, 450 "
          f"bytes on")

    parsed = [save.read(path, game) for path in save.saves(game)]
    complete = sum(s.complete for s in parsed)
    check("saves: every save parses to its last byte",
          parsed and complete == len(parsed),
          f"{complete}/{len(parsed)} read header, world, objectives, mind "
          f"lists, designs and one AI state per clan and end exactly -- "
          f"{sum(len(s.objects) for s in parsed)} objects, "
          f"{sum(len(s.objectives) for s in parsed)} objectives, "
          f"{sum(len(s.designs) for s in parsed)} designs; the mind lists' "
          f"lengths are the clans' minds words from data.tma")

    if (game / "objects.rlb").exists():
        library_rlb = objects.ObjectLibrary(game / "objects.rlb")
        models = [o for s in parsed for o in s.objects if o.archive]
        tagged = sum(1 for o in models if library_rlb.get(o.member) is not None
                     and save.KIND_TAGS.get(o.kind) == library_rlb.get(o.member).tag)
        others = Counter(hex(o.kind) for s in parsed for o in s.objects if not o.archive)
        check("saves: an object id's top byte is its member's objects.rlb tag",
              models and tagged == len(models),
              f"{tagged}/{len(models)} model objects -- "
              + ", ".join(f"{k:#x} {t}" for k, t in save.KIND_TAGS.items())
              + "; the rest are " + ", ".join(f"{n} x {k}" for k, n in sorted(others.items()))
              + " (land, sky, research trees)")

    levels = Counter(s.level_name for s in parsed)
    check("saves: the header's second byte is a difficulty",
          parsed and all(s.level in save.LEVELS for s in parsed),
          f"every save holds one of {sorted(save.LEVELS)} -- "
          + ", ".join(f"{n} {k}" for k, n in sorted(levels.items()))
          + " -- the byte iron3d.dll's level ratio reads as EASY/MEDIUM/HARD")

    counted_chunks = extra_ok = buildings = 0
    for s in parsed:
        for o in s.objects:
            if not o.counts:
                continue
            owed = 1 + sum(o.counts)
            if o.kind == save.KIND_BUILDING:
                buildings += 1
                extra_ok += len(o.chunks) == owed + 1 and o.chunks[-1].size == 4
            else:
                counted_chunks += len(o.chunks) == owed
    models = sum(1 for s in parsed for o in s.objects if o.counts)
    check("saves: chunk 0 counts every other chunk a model object holds",
          models and counted_chunks == models - buildings and extra_ok == buildings,
          f"{counted_chunks}/{models - buildings} units and scenery hold exactly "
          f"1 + the sum of chunk 0's bytes; all {extra_ok}/{buildings} buildings "
          f"hold one more, of 4 bytes")

    homes = Counter()
    for s in parsed:
        spans = []
        for o in s.objects:
            spans.append((o.offset, o.offset + save.RECORD_HEAD, "record head"))
            if o.counts:
                first = 1 + o.counts[0]
                for i, c in enumerate(o.chunks):
                    label = ("part list" if 1 <= i < first
                             else "control chunk" if i == first + 1 else "other chunk")
                    spans.append((c.offset, c.offset + c.size, label))
        designs_from = s.world.offset + s.world.size
        for r in s.references:
            label = "unit design" if r.offset >= designs_from else "unplaced"
            for a, b, name in spans:
                if a <= r.offset < b:
                    label = name
                    break
            homes[label] += 1
    design_parts = sum(len(d.components) for s in parsed for d in s.designs)
    placed_refs = sum(homes.values()) - homes["unplaced"] - homes["other chunk"]
    check("saves: every scanned reference sits in a field the parse names",
          homes and placed_refs == sum(homes.values())
          and homes["unit design"] == design_parts,
          f"{placed_refs}/{sum(homes.values())} references -- "
          + ", ".join(f"{n} in a {k}" for k, n in homes.most_common()))

    lists = unique = parts_total = resolved = 0
    for s in parsed:
        for o in s.objects:
            if not o.parts:
                continue
            lists += 1
            ids = [p.id for p in o.parts]
            unique += len(ids) == len(set(ids))
            known = set(ids) | {save.ROOT_PART}
            parts_total += len(ids)
            resolved += sum(p.parent in known for p in o.parts)
    check("saves: a part hangs off a part of the same object",
          lists and unique == lists and resolved == parts_total,
          f"{parts_total} part records in {lists} lists; +{save.PART_ID} is "
          f"unique within all {unique}, and +{save.PART_PARENT} names the object "
          f"({save.ROOT_PART}) or another part's id on {resolved}/{parts_total}")

    kinds = defaultdict(lambda: [0, 0, 0])
    on_guns = 0
    library = game / descriptions.LIBRARY
    if library.exists():
        catalogue = descriptions.read(library)
        for s in parsed:
            for o in s.objects:
                by_id = {p.id: p for p in o.parts}
                for p in o.parts:
                    part = catalogue.get(p.member)
                    if part is None:
                        continue
                    row = kinds[part.kind]
                    row[0] += 1
                    row[1] += p.parent != save.ROOT_PART
                    row[2] += p.attach != 0
                    holder = catalogue.get(by_id[p.parent].member) \
                        if p.parent in by_id else None
                    on_guns += part.kind == "AMM" and holder is not None \
                        and holder.kind == "WPN"
        ammo = kinds.get("AMM", [0, 0, 0])
        check("saves: ammunition sits in slot 0 of its weapon",
              ammo[0] and on_guns == ammo[0] and ammo[2] == 0,
              f"all {on_guns}/{ammo[0]} ammunition parts hang off a WPN part at "
              f"attachment 0; parent is a part / attachment non-zero on "
              + ", ".join(f"{k} {v[1]}/{v[0]} and {v[2]}/{v[0]}"
                          for k, v in sorted(kinds.items()) if k != "AMM"))

    found = by_name = upright = turned = scaled = 0
    for s in parsed:
        tma = game / s.mission / "data.tma"
        if not tma.exists():
            continue
        named = []
        for o in mission.load(tma).objects:
            if o.is_static:
                name = o.path
            else:
                unit = objects.load_unit(game / o.path.replace("\\", "/"))
                name = unit.components[0].ref.member if unit.components else ""
            named.append((name.lower(), o))
        for w in s.objects:
            if w.position is None:
                continue
            for name, o in named:
                if not all(abs(a - b) < 0.25 for a, b in zip(w.position, o.position, strict=True)):
                    continue
                found += 1
                by_name += name == w.member.lower()
                scaled += w.scale is not None and all(
                    abs(a - b) < 1e-4 for a, b in zip(w.scale, o.scale, strict=True))
                qw, qx, qy, qz = w.orientation
                if abs(qx) < 1e-3 and abs(qy) < 1e-3:
                    upright += 1
                    gap = (-2 * math.atan2(qz, qw) - o.rotation + math.pi) % (2 * math.pi)
                    turned += abs(gap - math.pi) < 0.01
    check("saves: an object's placement is in its own record",
          found and by_name >= found - 1 and scaled == found and turned == upright,
          f"{found} objects stand where their mission placed them, read at "
          f"+{save.CONTROL_POSITION} of their own control chunk; {by_name} carry "
          f"the placed name or its .dat's root part (the other is a plant "
          f"upgraded since); {scaled}/{found} hold its scale in the model chunk and "
          f"{turned}/{upright} upright ones its angle a as (cos a/2, 0, 0, -sin a/2)")

    near = exact = tilted_moved = scenery_total = 0
    farthest = 0.0
    for s in parsed:
        tma = game / s.mission / "data.tma"
        if not tma.exists():
            continue
        statics = [o for o in mission.load(tma).objects if o.is_static]
        for w in s.objects:
            if w.kind != save.KIND_SCENERY:
                continue
            scenery_total += 1
            same = [o.position for o in statics if o.path.lower() == w.member.lower()]
            if not same:
                continue
            gap = min(math.dist(w.position, p) for p in same)
            farthest = max(farthest, gap)
            near += gap < 6
            upright_here = abs(w.orientation[1]) < 1e-3 and abs(w.orientation[2]) < 1e-3
            if gap < 0.25 and upright_here:
                exact += 1
            else:
                tilted_moved += not upright_here
    check("saves: scenery keeps its own placement, tipped where it moved",
          scenery_total and near == scenery_total
          and exact + tilted_moved == scenery_total,
          f"{near}/{scenery_total} scenery records lie within {farthest:.1f} of a "
          f"placement of their own name; {exact} stand on it upright and every "
          f"one of the other {tilted_moved} is tipped off the vertical")

    table = behaviour.variables(game) if (
        game / "MISSIONS" / "SCRIPTS" / behaviour.VARSET).exists() else []
    sizes = [c.size for s in parsed for c in s.ai]
    floor = save.AI_FIXED + save.AI_PER_VARIABLE * len(table)
    check("saves: a clan's AI state is 2036 bytes and a word per script variable",
          sizes and table and min(sizes) == floor and all(n % 4 == 0 for n in sizes),
          f"{sizes.count(floor)}/{len(sizes)} clans hold exactly {save.AI_FIXED} + "
          f"{save.AI_PER_VARIABLE} x {len(table)} varset.var declarations = {floor}; "
          f"the rest {sorted(set(sizes) - {floor})}, whole words more")

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

    # bu_ and fr_ are not two sides: a building's FORT record names its body,
    # a BTLU record, which names the fr_ models.  Terrain.dll's CBuilding
    # loads the first slot through AniMesh.dll's LoadAgent (0x10055e95).
    records = objects.ObjectLibrary(library)
    forts = records.by_tag("FORT")
    bodies = {r.name.lower(): r for r in records.by_tag("BTLU")
              if r.name.lower().startswith("bu_")}
    named = drawn = 0
    for fort in forts:
        body = bodies.get(fort.slots[0].member.lower()) if fort.slots else None
        if body is None or fort.slots[0].member.lower() != "bu_" + fort.name.lower()[3:]:
            continue
        named += 1
        stem = fort.slots[1].member.lower().rsplit(".", 1)[0]
        drawn += all(s.library.lower() == "fortif.rlb"
                     and s.member.lower().rsplit(".", 1)[0] == stem for s in body.slots)
    check("vocabulary: bu_ is a building's body, not a second set of models",
          forts and named == len(forts) == len(bodies) and drawn == named,
          f"{named}/{len(forts)} FORT records, all fr_, name in their first slot "
          f"the bu_ BTLU record of the same suffix, and all {len(bodies)} bu_ "
          f"records are named so; {drawn} of them draw on the fr_ model their "
          f"building's .bas names -- one set of models, the six towers sharing two")

    # The component classes World3D.dll's resolver leaves unnamed.
    archive_names = frozenset(p.name.lower() for p in all_archives(game))
    classes = Counter()
    for path in all_archives(game):
        archive = NResArchive.open(path)
        for entry in archive:
            if entry.tag.upper().startswith("CTL"):
                for component in control.parse(archive.read(entry), archive_names).components:
                    classes[component.type_id] += 1
    unnamed = (6, 7, 14, 16, 17, 18)
    check("vocabulary: of the unnamed component classes only 17 ships",
          classes and [c for c in unnamed if classes[c]] == [control.SEEKER_TYPE],
          f"{sum(classes.values())} components over {len(classes)} classes; "
          + ", ".join(f"{c}: {classes[c]}" for c in unnamed)
          + " -- 17 is the rounds' seeker, and no controller carries 6, 7, 14, "
          "16 or 18")

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
          f"in a part list it is the part's own id, handed out lowest-free as "
          f"parts come and go, which is why it runs down as often as up")


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

    atmospheres = []
    for path in sorted(game.rglob("sky.ske")):
        try:
            atmospheres.append((path, sky.load(path)))
        except (sky.SkyFormatError, struct.error):
            continue

    # The sections are not alternatives: CAtmosphere keeps one cycle as long
    # as all their days together (0x1006efcd) and walks through them in turn
    # (0x10070040).  Each section's own header counts its keyframes.
    cycles = wholes = 0
    reused = []
    counts = []
    for _path, atmosphere in atmospheres:
        if atmosphere.section_count < 2:
            continue
        cycles += 1
        spans = []
        for index in range(atmosphere.section_count):
            group = atmosphere.section_keyframes(index)
            spans.append((group[0].hour, group[-1].hour))
        wholes += all(lo == 0 and hi == 24 for lo, hi in spans)
        blocks = [{b"".join(f.slots) for f in atmosphere.keyframes if f.section == i}
                  for i in range(2)]
        reused.append(len(blocks[0] & blocks[1]))
        counts.append(tuple(h.count for h in atmosphere.section_headers))
    if cycles:
        check("sky: a second section is a second whole day, played after the first",
              wholes == cycles,
              f"{wholes}/{cycles} files with two sections have both running 00h to 24h, "
              f"the second reusing {max(reused)} of the first's colour blocks at its own "
              f"times; each section header counts its own keyframes "
              f"{dict(Counter(counts))}, and the cycle is both days end to end")

    # The opcode is the word ahead of slot 0.  A name only matters on a SUN
    # start, so the check is that the bodies sit on SUN's opcodes.  Control:
    # the same word read as the previous keyframe's trailer, which is how it
    # was first tested and dropped.
    on_sun = shifted_on_sun = bodies = 0
    for _path, atmosphere in atmospheres:
        for index in range(atmosphere.section_count):
            group = atmosphere.section_keyframes(index)
            for i, frame in enumerate(group):
                if frame.name not in sky.BODY_ANGLES:
                    continue
                bodies += 1
                on_sun += frame.opcode in (0, 1)
                later = group[i + 1].opcode if i + 1 < len(group) else sky.NOTHING
                shifted_on_sun += later in (0, 1)
    check("sky: the word ahead of slot 0 is the event opcode",
          bodies and on_sun >= bodies - 2 and shifted_on_sun < bodies // 4,
          f"{on_sun}/{bodies} keyframes naming the sun or the moon carry SUN's start or "
          f"stop (the other {bodies - on_sun} are the middle keyframes of the two "
          f"24-hour skies, which name the sun on every keyframe); control: the next "
          f"keyframe's word, the old trailer reading, gives {shifted_on_sun}")

    # Every start is followed by its own stop in the same section.
    started: Counter[str] = Counter()
    stopped: Counter[str] = Counter()
    for _path, atmosphere in atmospheres:
        for index in range(atmosphere.section_count):
            for kind, _start, stop in atmosphere.windows(index):
                started[kind] += 1
                stopped[kind] += stop is not None
    check("sky: every start is stopped later in its section",
          started == stopped and set(started) == {"sun", "moon", "rain", "snow", "lightning"},
          ", ".join(f"{kind} {stopped[kind]}/{started[kind]}" for kind in sorted(started))
          + " -- rain by opcode 4, snow 6 and lightning 9 on keyframes that name nothing, "
          "which is why no stop could be found by name")

    # Snow.  sky.wea's slot 7 names SNOWFLAKE in 23 missions and DUST_ADD in 6,
    # and the six are the ones that snow from 00:00 to 23:59 -- a dust storm
    # the length of the day.
    dusty = all_day = snowflake_snow = 0
    for _path, atmosphere in atmospheres:
        material = atmosphere.texture("snow")
        snows = [(s, t) for kind, s, t in atmosphere.windows(0) if kind == "snow"]
        if material != "SNOWFLAKE":
            dusty += 1
            all_day += (bool(snows) and snows[0][0].minutes == 0
                        and snows[-1][1] is not None
                        and snows[-1][1].minutes == 23 * 60 + 59)
        elif snows:
            snowflake_snow += 1
    check("sky: the missions whose snow slot is not SNOWFLAKE snow all day",
          dusty and all_day == dusty,
          f"{all_day}/{dusty} missions naming DUST_ADD in sky.wea slot 7 start SNOW at "
          f"00:00 and stop it at 23:59; {snowflake_snow} SNOWFLAKE mission snows, in "
          f"three spells")

    # The fourth float is what rain, snow and lightning take (0x1006ce00):
    # non-zero only while weather runs, the stopping keyframe included.
    wet = wet_inside = dry = 0
    for _path, atmosphere in atmospheres:
        for index in range(atmosphere.section_count):
            group = atmosphere.section_keyframes(index)
            inside: set[int] = set()
            for kind, start, stop in atmosphere.windows(index):
                if kind in sky.BODY_ANGLES:
                    continue
                first = group.index(start)
                last = group.index(stop) if stop is not None else len(group) - 1
                inside.update(range(first, last + 1))
            for i, frame in enumerate(group):
                if frame.weather_intensity:
                    wet += 1
                    wet_inside += i in inside
                elif i not in inside:
                    dry += 1
    check("sky.ske: the fourth float is the running weather's intensity",
          wet and wet_inside == wet,
          f"{wet_inside}/{wet} keyframes with a non-zero fourth float lie inside a rain, "
          f"snow or lightning spell (its stop included); it is 0 on all {dry} "
          f"keyframes outside one")

    # The file closes on the clock's start time (0x1006fab0), and every
    # mission opens with the sun up.
    in_section_0 = sunlit = 0
    for _path, atmosphere in atmospheres:
        in_section_0 += atmosphere.start.section == 0
        minute = atmosphere.start.hour * 60 + atmosphere.start.minute
        sunlit += any(kind == "sun" and s.minutes <= minute < (t.minutes if t else 1440)
                      for kind, s, t in atmosphere.windows(atmosphere.start.section))
    opening = Counter(f"{a.start.hour:02d}:{a.start.minute:02d}" for _p, a in atmospheres)
    check("sky.ske: the closing time is where the clock starts, with the sun up",
          in_section_0 == sunlit == len(atmospheres),
          f"{in_section_0}/{len(atmospheres)} in section 0, at "
          f"{dict(sorted(opening.items()))}; the sun is up at that time on {sunlit}")

    # What the rest of the file holds.  The last int32 is the sky's sixth
    # parameter, which CSky stores and never reads; it is 1 exactly where the
    # section headers carry the uninitialised word 6939832, which marks the
    # editor session that saved the file rather than a choice of the mission.
    versions = Counter(f.version for _p, a in atmospheres for f in a.keyframes)
    headers = [h for _p, a in atmospheres for h in a.section_headers]
    ends = Counter((h.end.hour, h.end.minute) for h in headers)
    flag_matches = sum(
        (a.sky_flag == 1) == (a.section_headers[0].day.words[7] == 6939832)
        for _p, a in atmospheres)
    check("sky.ske: the header's other fields are constants and editor state",
          set(versions) == {sky.KEYFRAME_VERSION}
          and all(h.version == sky.SECTION_VERSION for h in headers)
          and set(ends) == {sky.SECTION_END}
          and all(a.trailer_word == 0 for _p, a in atmospheres)
          and flag_matches == len(atmospheres),
          f"keyframe version {sky.KEYFRAME_VERSION} on {versions[sky.KEYFRAME_VERSION]}, "
          f"section version 1 and a first time of 23:59 on {len(headers)}/{len(headers)} "
          f"sections, the int after the closing time 0 on all; the last int is 1 on "
          f"{sum(a.sky_flag == 1 for _p, a in atmospheres)} files, and agrees with the "
          f"header's uninitialised word 6939832 on {flag_matches}/{len(atmospheres)}")


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
          f"TRF1, and {flagged} set the boolean the loader keeps beside it -- "
          f"the flag iron3d.dll reads as 'contains debugging information'")

    # +0x18 is a TRF9 offset: IResearch slot 19 adds it to the stream's base.
    landed = records = described = 0
    for path, d in zip(paths, directories, strict=True):
        archive = NResArchive(path.read_bytes())
        base, text = archive.read(d["TRF0"]), archive.read(d["TRF9"])
        starts = {0} | {i + 1 for i, byte in enumerate(text) if byte == 0}
        for index in range(len(base) // research.RECORD):
            at = struct.unpack_from("<i", base, index * research.RECORD + 0x18)[0]
            records += 1
            landed += at in starts
            described += text[at:at + 1] not in (b"", b"\0")
    check("research: the record's +0x18 is its description in TRF9",
          records and landed == records and described == 150 * len(paths),
          f"{landed}/{records} land on a string start in TRF9, and "
          f"{described // len(paths)} per archive on text -- the 150 items that "
          f"have a description; the rest point at an empty string")

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

    library = game / descriptions.LIBRARY
    if library.exists():
        parts = descriptions.read(library)
        tokens: list[defaultdict[int, set[str]]] = [defaultdict(set) for _ in range(3)]
        joined = named = 0
        for tree in trees:
            for item in tree.items:
                for pid in item.parts:
                    part = parts.get(pid)
                    if part is None:
                        continue
                    joined += 1
                    chain = part.sub.split(":")
                    kind = "SHS" if part.kind == "ANM" else part.kind
                    branch = chain[1] if part.kind == "BLD" and len(chain) > 1 else ""
                    tokens[0][item.tail[1]].add(kind)
                    tokens[1][item.tail[2]].add(chain[0])
                    tokens[2][item.tail[3]].add(branch)
                    named += (item.part_kind == kind and item.part_sub == chain[0]
                              and item.part_branch == branch)
        single = all(len(s) == 1 for table in tokens for s in table.values())
        kinds = ", ".join(sorted({t for s in tokens[0].values() for t in s}))
        check("research: +0x23..+0x25 are the library's classification line",
              joined and single and named == joined,
              f"over {joined} part entries every value of the three bytes stands "
              f"for one token of {descriptions.LIBRARY}'s line -- kind "
              f"{len(tokens[0])} values ({kinds}), "
              f"sub-kind {len(tokens[1])}, a building's second sub-kind "
              f"{len(tokens[2]) - 1} and 255 -- and {named} match "
              f"research.PART_KINDS/SUBS/BRANCHES")

    states = Counter(item.category for tree in trees for item in tree.items)
    opened = [(tree, item) for tree in trees for item in tree.items
              if item.category == research.IN_TREE | research.AVAILABLE]
    waiting = [(tree, item) for tree in trees for item in tree.items
               if item.category == research.IN_TREE]
    ready = sum(all(tree[r].researched for r in item.requires) for tree, item in opened)
    blocked = sum(not all(tree[r].researched for r in item.requires)
                  for tree, item in waiting)
    check("research: TRF1 is in-tree, researched and available bits",
          set(states) <= {0, 2, 4, 5, 7} and opened and ready == len(opened)
          and waiting and blocked == len(waiting),
          f"five values across {sum(states.values())} items ("
          + ", ".join(f"{k}: {v}" for k, v in sorted(states.items()))
          + f"); every 5, in the tree and available, has all its prerequisites "
          f"researched ({ready}/{len(opened)}) and every 4 waits on one "
          f"({blocked}/{len(waiting)}) -- the rule MisLoad.dll re-applies when a "
          f"research completes; 7s are granted outright")

    typed = matched = 0
    ruins = Counter()
    first = trees[0] if trees else None
    for d in gamedir.missions(game) if first else ():
        m = mission.load(d / "data.tma")
        for o in m.objects:
            if o.kind != mission.KIND_BUILDING or o.type_id is None:
                continue
            unit = objects.load_unit(game / o.path.replace("\\", "/"))
            item = first.item_for(unit.components[0].ref.member)
            if item is None:
                continue
            typed += 1
            if item.object_type == o.type_id & 0xFFFFFFFF:
                matched += 1
            else:
                ruins[(item.part_sub, o.type_id & 0xFFFFFFFF)] += 1
    check("research: a building's Type follows from its sub-kind",
          typed and matched == typed - 3 and set(ruins) == {("RUN", 0x80002000)},
          f"{matched}/{typed} placed buildings carry the Type iron3d.dll derives "
          f"from their root part's sub-kind and, for a bunker, size "
          f"(0x1008a590); the other "
          + ", ".join(f"{n} {sub} placed as {t:#x}" for (sub, t), n in ruins.items())
          + ", which the derivation leaves at 0")

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

    graded = [p for p in parts.values() if p.graded]
    ungraded = Counter(p.kind for p in parts.values() if not p.graded)
    check("descriptions: the A<n> token is a size grade",
          len(graded) == len(parts) - 37 and set(ungraded) == {"WPN", "AMM"},
          f"{len(graded)}/{len(parts)} parts carry the grade their size letter "
          f"implies -- "
          + ", ".join(f"{k} {v}" for k, v in descriptions.GRADES.items())
          + "; the other "
          + " and ".join(f"{n} {k}" for k, n in sorted(ungraded.items()))
          + " are launchers, their packs and the level-0 large guns")

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

    orbits = [w for _, w in stops if w.wait == "flyaround"]
    turning = [w for _, w in stops if w.rotate_time]
    once = sum(abs(w.dwell - w.rotate_time) < 0.05 and w.wait_for_time for w in orbits)
    splines = [w for _, w in stops if w.edge == "spline"]
    check("briefing: a flyaround dwells for one turn of RotateTime",
          orbits and turning == orbits and once == len(orbits)
          and all(w.edge_time > 0 for w in splines),
          f"RotateTime is set on exactly the {len(orbits)} flyaround waypoints "
          f"and on no other of {len(stops)}; {once}/{len(orbits)} dwell within "
          f"0.05 s of one revolution; all {len(splines)} spline edges have a "
          f"positive EdgeTime, as the loader demands")

    outs = ins = timed_outs = changes = carried = 0
    dips = dipped = 0
    for path in paths:
        route = briefing.waypoints(path)
        for i, w in enumerate(route[:-1]):
            after = route[briefing.next_stop(route, i)]
            if w.edge == "jump":
                dips += 1
                dipped += w.fade == after.fade == 0
                continue
            if w.fade != after.fade:
                changes += 1
                carried += w.fade_time > 0
                if w.fade < after.fade:
                    outs += 1
                    timed_outs += abs(w.fade_time - w.edge_time) < 0.05
                else:
                    ins += 1
    black = sum(r[0].fade == r[-1].fade == 100
                for r in (briefing.waypoints(p) for p in paths))
    check("briefing: FadePercent is the black, FadeTime its change to the next",
          changes and carried == changes and timed_outs == outs
          and dipped == dips and black == len(paths),
          f"{carried}/{changes} changes of FadePercent between a waypoint and "
          f"the next carry a FadeTime; every one of the {outs} fades to black "
          f"lasts the edge leaving the waypoint ({timed_outs}), the {ins} fades "
          f"in no longer; {dipped}/{dips} jumps sit between two clear waypoints "
          f"and dip on their own; {black}/{len(paths)} briefings open and close "
          f"black")

    unused = sum(w.noise == 0 and not w.zoom and not w.zoom_time
                 and not w.night_vision for _, w in stops)
    check("briefing: the picture effects are never switched on",
          unused == len(stops),
          f"{unused}/{len(stops)} waypoints hold NoisePercent 0, ZoomOn and "
          f"NightVisionOn false and no ZoomTime -- NoisePercent is not read by "
          f"the player at all, and the other three would be")

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

    scripts_dir = game / "MISSIONS" / "SCRIPTS"
    if not (scripts_dir / behaviour.VARSET).exists():
        return
    table = behaviour.variables(game)
    names = {v.name: i for i, v in enumerate(table)}
    by_stem = {p.stem.lower(): p for p in behaviour.scripts(game)}
    asked = known = missions_with = 0
    for f in files:
        ids = {m.index for m in briefing.messages(f)}
        literal = []
        for clan in mission.load(f.parent / "data.tma").clans:
            stem = clan.ai_script.replace("\\", "/").rsplit("/", 1)[-1].lower()
            if stem not in by_stem:
                continue
            for handler in behaviour.read(by_stem[stem]).handlers:
                for node in handler.nodes:
                    if (node.calls and len(node.operands) > 1
                            and node.operands[0] == names.get("MESSAGE_INFO")):
                        arg = node.operands[1]
                        if 0 <= arg < len(table) and table[arg].literal:
                            literal.append(int(table[arg].name[1]))
        missions_with += bool(literal)
        asked += len(literal)
        known += sum(v in ids for v in literal)
    holders = {i: [f.parent for f in files
                   if i in {m.index for m in briefing.messages(f)}]
               for i in briefing.ENGINE_MESSAGES}
    training = all(len(h) == 1 and h[0].parent.name.upper() == "CAMPAIGN.00"
                   for h in holders.values())
    check("briefing: a script asks for a message by its id",
          asked and known == asked and training,
          f"{known}/{asked} literal ids {len(files)} missions' clan scripts pass "
          f"with MESSAGE_INFO are message_index values of that mission's "
          f"messages.cfg ({missions_with} missions); the ids iron3d.dll asks "
          f"for itself are in one file each -- "
          + ", ".join(f"{i} in {'/'.join(h[0].parts[-2:]) if h else '-'}"
                      for i, h in holders.items()))


#: The briefing screen's GUI-server calls (``iron3d.dll:0x100315b0``), in the order it
#: makes them; each a run of bytes, several runs one after another.
BRIEFING_SCREEN = (
    ("fade: black, alpha round(fade x 255), alpha kept",
     ("d9433c8b5500d80dc8590e10", "6a0133f6df7c24208b4c2420c1e118")),
    ("top bar (0,0)-(640,75)", ("5668000000ff6a4b68800200005656",)),
    ("bottom bar (0,405)-(640,480)", ("5668000000ff68e00100006880020000689501000056",)),
    ("title: MENU_FONT (+0x14), (20,20), #808080", ("8b40148b550068808080ff6a146a145350",)),
    ("step: round((GAME_FONT height + 1) / sy)",
     ("8b7010", "4089442418db4424188bcfd95c2418ff5214d87c2418")),
    ("lines: (10, 410 + i x step), #808080", ("bf9a010000", "68808080ff576a0a", "83c10c03fa")),
    ("viewport back to y 75 .. height - 75", ("c74424284b000000", "83e84b")),
)

#: The briefing camera (``iron3d.dll:0x1003152d``): two zeros, a handle, then the angle,
#: the near and the far pushed as floats.
BRIEFING_CAMERA = (1.04, 3.0, 700.0)


def _wav_seconds(data: bytes) -> float | None:
    """A RIFF WAVE's length in seconds: ``fact`` samples over the rate, else data over bytes/s."""
    if data[:4] != b"RIFF" or data[8:12] != b"WAVE":
        return None
    rate = per_second = samples = size = None
    i = 12
    while i + 8 <= len(data):
        chunk, length = data[i:i + 4], struct.unpack_from("<I", data, i + 4)[0]
        if chunk == b"fmt ":
            _, _, rate, per_second = struct.unpack_from("<HHII", data, i + 8)
        elif chunk == b"fact":
            samples = struct.unpack_from("<I", data, i + 8)[0]
        elif chunk == b"data":
            size = length
        i += 8 + length + (length & 1)
    if samples and rate:
        return samples / rate
    return size / per_second if size is not None and per_second else None


def check_briefing_screen(check, game: Path) -> None:
    """How iron3d.dll shows a briefing: the screen, the camera, the curve, the paused world."""
    path = game / "iron3d.dll"
    if not path.exists():
        return
    at = _image_at(path.read_bytes())

    def floats(va: int, n: int = 1) -> tuple[float, ...]:
        return struct.unpack(f"<{n}f", at(va, 4 * n))

    def runs(body: bytes, patterns: tuple[str, ...], start: int = 0) -> int | None:
        """Where the last of ``patterns`` ends when each follows the one before, or None."""
        for hexed in patterns:
            found = body.find(bytes.fromhex(hexed), start)
            if found < 0:
                return None
            start = found + len(hexed) // 2
        return start

    screen = at(0x100315B0, 0x286)
    where, order = 0, []
    for name, patterns in BRIEFING_SCREEN:
        end = runs(screen, patterns, where)
        order.append((name, end is not None))
        if end is not None:
            where = end
    check("iron3d.dll: the briefing screen is a fade, two bars, a title, subtitles",
          all(ok for _, ok in order) and floats(0x100E59C8) == (255.0,),
          "0x100315b0 in order: " + "; ".join(
              f"{name}{'' if ok else ' MISSING'}" for name, ok in order)
          + f"; the fade's factor at 0x100e59c8 is {floats(0x100E59C8)[0]:g}")

    pushes = at(0x1003152D, 0x14)
    angle = near = far = None
    if pushes[:5] == bytes.fromhex("6a006a0050") and pushes[5::5][:3] == b"hhh":
        angle, near, far = (struct.unpack_from("<f", pushes, 6 + 5 * k)[0] for k in range(3))
    got = tuple(round(v, 6) for v in (angle, near, far)) if angle is not None else ()
    rect = at(0x100319E0, 0x3C)
    zoom = at(0x10036D02, 6) == bytes.fromhex("d80d685c0e10") and floats(0x100E5C68)[0] == \
        struct.unpack("<f", struct.pack("<f", 0.2))[0]
    check("iron3d.dll: the briefing camera is 1.04 rad across, 3 to 700, framed 75 in",
          got == tuple(round(struct.unpack("<f", struct.pack("<f", v))[0], 6)
                       for v in BRIEFING_CAMERA)
          and b"\xc7\x46\x04\x4b\x00\x00\x00" in rect and b"\x83\xe8\x4b" in rect and zoom,
          f"0x10031532 pushes angle {angle or 0:.2f}, near {near or 0:g}, far {far or 0:g}; "
          f"0x100319e0 makes the "
          f"viewport (0, 75)-(width, height - 75); the zoom 0x10036c90 blends the angle "
          f"toward {floats(0x100E5C68)[0]:.2f}")

    def text(va: int) -> bytes:
        return at(va, 16).split(b"\0")[0]

    title_read = (at(0x1005DFB1, 5) == b"\x68" + struct.pack("<I", 0x10103434)
                  and text(0x10103434) == b"descr"
                  and at(0x1005E04F, 5) == bytes.fromhex("6880000000")
                  and at(0x1005E084, 6) == bytes.fromhex("8d8ed4000000"))
    title_used = at(0x100313CE, 5) == bytes.fromhex("05d4000000")
    switch = (at(0x100313EA, 10) == b"\x68" + struct.pack("<I", 0x10103584)
              + b"\x68" + struct.pack("<I", 0x10102468)
              and text(0x10103584) == b"SUBTITLES" and text(0x10102468) == b"CS")
    titles = {}
    for cfg in briefing.briefings(game):
        descr = cfg.parent / "descr"
        first = descr.read_bytes().split(b"\n")[0].rstrip(b"\r") if descr.exists() else b""
        titles[cfg.parent] = first.decode("latin-1")
    one = titles.get(game / MISSION_01, "")
    check("iron3d.dll: the title is descr's first line; [CS] SUBTITLES shows the text",
          title_read and title_used and switch and titles
          and all(titles.values()) and one == "Line of Fire",
          f"the mission loader fgets 128 bytes of 'descr' into the game's +0xd4 "
          f"(0x1005dfb1); the briefing copies +0xd4 (0x100313ce) and reads "
          f"[{text(0x10102468).decode()}] {text(0x10103584).decode()}; "
          f"{sum(map(bool, titles.values()))}/{len(titles)} briefing missions have a "
          f"titled descr, Mission 01's '{one}'")

    wrap_call = next((t for c, t in _calls(at, 0x100318E3, 10) if c == 0x100318E3), 0)
    check("iron3d.dll: the subtitle wraps at 0.98 of 640 in GAME_FONT",
          at(0x100318CF, 10) == bytes.fromhex("8b40106a016880020000")
          and wrap_call == 0x10093140
          and at(0x100931BD, 6) == bytes.fromhex("d80d28670e10")
          and abs(floats(0x100E6728)[0] - 0.98) < 1e-6,
          f"0x10031840 wraps the +0x10 font's text to 640 through {wrap_call:#x}, "
          f"which fills while the words fit {floats(0x100E6728)[0]:.2f} x 640 x sx pixels")

    build = at(0x1002CC50, 0x238)
    three = bytes.fromhex("d80d145c0e10")
    setup = at(0x10030D33, 0x90)
    curves = [c for c, t in _calls(at, 0x10030D33, 0x90) if t == 0x1002CC50]
    eye_first = setup.find(bytes.fromhex("8d8e94000000e8"))
    look_then = setup.find(bytes.fromhex("8d8efc000000e8"))
    check("iron3d.dll: a spline edge is a cubic Hermite over EdgeTime",
          floats(0x100E5C14) == (3.0,) and build.count(three) == 3
          and build.count(b"\xdc\xc0") == 3 and b"\xc2\x14\x00" in build[-8:]
          and bytes.fromhex("d87134") in at(0x1002CE90, 0x100)
          and at(0x1002CFA0, 0xF3).count(three) == 1
          and len(curves) == 2 and 0 <= eye_first < look_then,
          "0x1002cc50 keeps p0, a = T V0, c2 = 3(p1 - p0) - 2a - b, c3 = a + b - 2(p1 - p0) "
          "(three x 3.0, three doublings); 0x1002ce90 divides by T; 0x1002cfa0 has the one "
          "3u^2; 0x10030a10 builds the eye's curve (+0x94) and then the look-at's (+0xfc)")

    speed = at(0x1002FD63, 0x15) == bytes.fromhex(
        "d871208d8e74010000d9542420d954241cd95c2418")
    omega = (at(0x1002E885, 13) == bytes.fromhex("d905205c0e10d8742420d95d6c")
             and abs(floats(0x100E5C20)[0] - 2 * math.pi) < 1e-5)
    check("iron3d.dll: a linear edge's velocity is one speed three times",
          speed and omega,
          "0x1002fd63: |P1 - P0| / EdgeTime stored into all three of +0x174, which a "
          "following spline takes as its start tangent; the loader keeps 2pi / RotateTime "
          "at +0x6c (0x1002e885), the orbit rate the flyaround and its tangent use")

    pause_calls = [(c, t) for c, t in _calls(at, 0x100A2A7A, 0x21)]
    end_calls = [(c, t) for c, t in _calls(at, 0x1005E7FE, 0x18)]
    paused = (at(0x100A2A7A, 2) == b"\x6a\x01" and (0x100A2A7E, 0x100A1CA0) in pause_calls
              and at(0x100A2A91, 10) == bytes.fromhex("c78510070000") + struct.pack("<I", 5)
              and at(0x100A1D02, 5) == bytes.fromhex("3d00000201")
              and at(0x100A1D12, 5) == bytes.fromhex("680a020000")
              and at(0x1005E801, 1) == b"\x53" and (0x1005E802, 0x100A1CA0) in end_calls
              and at(0x1005E80A, 2) == b"\x6a\x01" and (0x1005E80C, 0x100A4F90) in end_calls)
    gated = (at(0x1008D302, 7) == bytes.fromhex("83bf1007000005")
             and at(0x1005E7A4, 7) == bytes.fromhex("83b81007000005")
             and at(0x1005E7B6, 6) == bytes.fromhex("8a88a4010000")
             and at(0x10070E64, 7) == bytes.fromhex("83be1007000005")
             and at(0x10070E75, 7) == bytes.fromhex("c680a401000001"))
    behaviour_ok = False
    bpath = game / "Behavior.dll"
    if bpath.exists():
        bat = _image_at(bpath.read_bytes())
        behaviour_ok = (bat(0x1000AAF9, 6) == bytes.fromhex("81e905020000")
                        and bat(0x1000AB01, 3) == bytes.fromhex("83e904")
                        and bat(0x1000AB06, 1) == b"\x49"
                        and bat(0x1000AB18, 6) == bytes.fromhex("89b7080a0000")
                        and bat(0x10004F20, 8) == bytes.fromhex("39ae080a00000f85"))
    check("iron3d.dll: in state 5 only the briefing draws, and all but the hero pause",
          paused and gated and behaviour_ok,
          "0x100a2a7a sets property 0x20a to 1 on every object not Type 0x1020000 "
          "(0x100a1ca0) and the state word to 5; the frame (0x1005e7a4) ends it on the "
          "finished byte +0x1a4, which Esc sets (0x10070e75), with 0x20a back to 0 and "
          "state 1; the screen layer draws only the briefing in 5 (0x1008d302); "
          "Behavior.dll keeps 0x205 + 4 + 1 at +0xa08 (0x1000ab18) and its takt returns "
          "while it is set (0x10004f20)")

    lib_path = game / "voices.lib"
    if not lib_path.exists():
        return
    lib = NResArchive.open(lib_path)
    members = {e.name.lower(): e for e in lib}
    voiced = fit = zero = stops = 0
    worst = (0.0, "")
    one_length = 0.0
    for cfg in briefing.briefings(game):
        route = briefing.waypoints(cfg)
        stops += len(route)
        zero += sum(w.edge_time <= 0 for w in route)
        bound: dict[str, str] = {}
        for d in resources.descriptors(cfg.parent / "mission.cfg"):
            if d.role == briefing.BRIEFING_ROLE:
                bound = {k.lower(): v for k, v in d.bindings.items()}
        clock, i, arrivals, seen = 0.0, 0, [], 0
        while i is not None and seen <= len(route):
            arrivals.append((i, clock))
            w = route[i]
            clock += (w.dwell if w.wait_for_time else 0.0) + w.edge_time
            i = briefing.next_stop(route, i)
            seen += 1
        if cfg.parent == game / MISSION_01:
            one_length = clock
        spoken = [(t, route[k].sound_id) for k, t in arrivals if route[k].sound_id]
        for n, (t, sound) in enumerate(spoken):
            member = members.get(bound.get(sound.lower(), "").lower())
            seconds = _wav_seconds(lib.read(member)) if member else None
            if seconds is None:
                continue
            voiced += 1
            until = spoken[n + 1][0] if n + 1 < len(spoken) else clock
            over = t + seconds - until
            fit += over <= 0
            if over > worst[0]:
                worst = (over, f"{cfg.parent.parent.name}/{cfg.parent.name} {sound}")
    check("briefing: the voices fit their waypoints, and no edge is instant",
          voiced == 165 and fit == voiced - 1 and worst[0] < 0.1 and zero == 0
          and abs(one_length - 97.965) < 1e-3,
          f"played from arrival, {fit}/{voiced} voices end before the next voice or the "
          f"briefing's end; the other, {worst[1]}, runs {worst[0]:.2f} s over; "
          f"{zero}/{stops} waypoints have an EdgeTime of 0; Mission 01 lasts "
          f"{one_length:.3f} s")


#: Mission 01, the first training mission, whose progression docs/34 walks.
MISSION_01 = "MISSIONS/CAMPAIGN/CAMPAIGN.00/Mission.01"

#: Mission 01's routes 1 to 4 as docs/34 gives them: x and y extents, rounded.
MISSION_01_ROUTE_BOXES = {
    1: (441, 713, 322, 677), 2: (671, 1020, 268, 667),
    3: (610, 1008, 666, 938), 4: (364, 969, 961, 1392),
}

#: ``varset.var``'s ``CLASS_ROBOT``: function 31's mask for "robots".
CLASS_ROBOT = 0x01000000

#: The two script functions a mission's progression turns on: 31 counts a
#: clan's units by class, 32 asks whether a unit stands in a route.
FN_COUNT, FN_IN_ROUTE = 31, 32

#: ``iron3d.dll``'s own strings for an objective's end and a repeated message.
PROGRESSION_STRINGS = {
    5040: "Objective is completed",
    5041: "Objective has failed",
    6170: "Recieved message is already in history",
    6223: "Press %s to see it.",
}


#: The voices the mission callback names, as ``ui/game_resources.cfg`` binds them.
PROGRESSION_VOICES = {
    "VOICE_OBJ_COMPLETE": "vc_obj_cpl.wav",
    "VOICE_MISSION_COMPLETE": "vc_mis_cpl.wav",
    "VOICE_MISSION_FAIL": "vc_mis_fail.wav",
}


def _script_value(table, nodes, at: int, var: int) -> int | None:
    """What operand ``var`` holds at node ``at``: a pool constant, or the last
    literal the handler wrote to it before then; ``None`` when neither."""
    if 0 <= var < len(table) and table[var].literal:
        return int(table[var].default)
    for node in reversed(nodes[:at]):
        if node.destination != var:
            continue
        if not node.calls and node.tag == behaviour.CONST and node.literal != behaviour.NULL:
            return node.literal
        return None
    return None


def check_progression(check, game: Path) -> None:
    """A mission's progression: routes, zone tests, counts, messages, objectives."""
    scripts_dir = game / "MISSIONS" / "SCRIPTS"
    if not (scripts_dir / behaviour.VARSET).exists():
        return
    table = behaviour.variables(game)
    names = {v.name: i for i, v in enumerate(table)}
    by_stem = {p.stem.lower(): p for p in behaviour.scripts(game)}
    dirs = sorted(p.parent for p in (game / "MISSIONS").rglob("data.tma"))
    loaded = [(d, mission.load(d / "data.tma")) for d in dirs]

    def clan_scripts(m):
        for clan in m.clans:
            stem = clan.ai_script.replace("\\", "/").rsplit("/", 1)[-1].lower()
            if stem in by_stem:
                yield clan, behaviour.read(by_stem[stem])

    numbered = sum(sorted(r.id for r in m.routes) == list(range(len(m.routes)))
                   for _, m in loaded)
    routes = sum(len(m.routes) for _, m in loaded)
    calls = in_route = on_unit = heroes = 0
    for _, m in loaded:
        ids = {r.id for r in m.routes}
        units = {o.logical_id: o for o in m.objects if o.kind == mission.KIND_UNIT}
        for _, script in clan_scripts(m):
            for handler in script.handlers:
                for i, node in enumerate(handler.nodes):
                    if not (node.calls and node.function == FN_IN_ROUTE):
                        continue
                    calls += 1
                    route = _script_value(table, handler.nodes, i, node.operands[0])
                    unit = _script_value(table, handler.nodes, i, node.operands[1])
                    in_route += route in ids
                    on_unit += unit in units
                    heroes += unit in units and "\\HERO\\" in units[unit].path.upper()
    check("progression: a route is a tactical areal, by its id",
          numbered == len(loaded) and routes,
          f"{routes} routes on {len(loaded)} missions, ids 0..n-1 on {numbered}: "
          f"IMission slot 8 gives tactical areal i the route whose id is i")
    check("progression: function 32 asks whether a unit is in a route",
          calls and in_route == on_unit == calls,
          f"{calls} calls in the scripts missions name: the first argument is one "
          f"of that mission's route ids on {in_route}, the second a unit's logical "
          f"id there on {on_unit} -- {heroes} the hero's, {calls - heroes} another unit's")

    d01 = game / MISSION_01
    if not (d01 / "data.tma").exists():
        return
    m01 = mission.load(d01 / "data.tma")
    by_id = {r.id: r for r in m01.routes}
    units = [o for o in m01.objects if o.kind == mission.KIND_UNIT]
    hero = next(o for o in units if "\\HERO\\" in o.path.upper())
    x, y, _ = hero.position
    holding = [i for i, r in sorted(by_id.items()) if r.contains(x, y)]
    neutral = [o for o in units if m01.clans[o.clan_id].index == mission.CLAN_NEUTRAL]
    neutral_in = {i for o in neutral for i, r in by_id.items()
                  if r.contains(o.position[0], o.position[1])}
    corner = max(math.hypot(px - x, py - y) for px, py, _ in by_id[0].points)
    boxes = {i: (round(min(q[0] for q in r.points)), round(max(q[0] for q in r.points)),
                 round(min(q[1] for q in r.points)), round(max(q[1] for q in r.points)))
             for i, r in by_id.items() if i}
    check("Mission 01: route 0 is the hero's start, route 3 the neutrals'",
          holding == [0] and neutral_in == {3} and len(neutral) == 2
          and len(by_id[0].points) == 4 and corner < 3
          and boxes == MISSION_01_ROUTE_BOXES,
          f"the hero (logical id {hero.logical_id}) starts inside route {holding}, "
          f"four corners at most {corner:.1f} m away; the {len(neutral)} "
          f"neutral units ({', '.join(o.path.rsplit(chr(92), 1)[-1] for o in neutral)}) "
          f"stand in route {sorted(neutral_in)}; routes 1-4 span x, y {boxes}")

    robots = Counter(o.clan_id for o in units if (o.type_id or 0) & CLASS_ROBOT)
    counted = []
    for _, script in clan_scripts(m01):
        for handler in script.handlers:
            for i, node in enumerate(handler.nodes):
                if node.calls and node.function == FN_COUNT:
                    counted.append(_script_value(table, handler.nodes, i, node.operands[0]))
    target_clan = next(i for i, c in enumerate(m01.clans) if c.name == "Trgt")
    check("Mission 01: the robots function 31 counts, by clan",
          dict(robots) == {0: 1, target_clan: 5, 2: 1, 3: 2}
          and all(o.type_id & CLASS_ROBOT for o in units)
          and set(counted) == {0, 1, 2, 3},
          f"CLASS_ROBOT is set on all {len(units)} units' Type; by clan "
          f"{dict(sorted(robots.items()))} -- the hero alone for Plr, five targets, "
          f"one enemy, two neutrals (a warrior each); tut1_pl2 counts clans "
          f"{sorted(set(counted))}")

    kinds = {names[k] for k in ("OBJECTIVE_COMPLETE", "OBJECTIVE_FAILED",
                                "OBJECTIVE_PROGRESS")}
    fits = total = 0
    misses: set[str] = set()
    m01_values: list[int] = []
    for d, m in loaded:
        cfg = d / "mission.cfg"
        if not cfg.exists():
            continue
        size = len(mission.objectives(cfg))
        for _, script in clan_scripts(m):
            for handler in script.handlers:
                for i, node in enumerate(handler.nodes):
                    if not (node.calls and node.function == 30
                            and node.operands[0] in kinds):
                        continue
                    value = _script_value(table, handler.nodes, i, node.operands[1])
                    if value is None:
                        continue
                    total += 1
                    fits += value < size
                    if value >= size:
                        misses.add(script.source.stem)
                    if d == d01:
                        m01_values.append(value)
    listed = mission.objectives(d01 / "mission.cfg")
    c2m3 = mission.objectives(game / "MISSIONS/CAMPAIGN/CAMPAIGN.02/Mission.03/mission.cfg")
    check("progression: a script names an objective by its place in the list",
          total and fits == total - 2 and misses == {"c2m3p"}
          and len(listed) == 3 and not any(o.exempt for o in listed)
          and sorted(m01_values) == [0, 1, 2],
          f"{fits}/{total} objective values the scripts pass fit their mission's "
          f"primary + bonus list; the 2 that do not are c2m3p's, past a list of "
          f"{len(c2m3)}; "
          f"Mission 01 lists {len(listed)} primary objectives and completes "
          f"{sorted(m01_values)}")

    voices = resources.locate(game, "voices.lib")
    members = ({e.name.lower() for e in NResArchive.open(voices).entries}
               if voices else set())
    files = sorted((game / "MISSIONS" / "CAMPAIGN" / "CAMPAIGN.00").glob(
        f"*/{briefing.MESSAGES}"))
    exact = present = lines = 0
    for f in files:
        roles = {d.role: d for d in resources.descriptors(f.parent / "mission.cfg")}
        spoken = {w.sound_id.lower() for w in briefing.waypoints(f.parent / briefing.BRIEFING)
                  if w.sound_id}
        for msg in briefing.messages(f):
            lines += 1
            by_briefing = roles.get(briefing.BRIEFING_ROLE)
            by_tutorial = roles.get(briefing.MESSAGE_ROLE)
            member = ((by_briefing and by_briefing.get(msg.voice_id))
                      or (by_tutorial and by_tutorial.get(msg.voice_id)) or "")
            present += member.lower() in members
            briefed = bool(by_briefing and by_briefing.get(msg.voice_id))
            exact += briefed == (msg.voice_id.lower() in spoken) and (
                briefed or bool(by_tutorial and by_tutorial.get(msg.voice_id)))
    check("progression: a training message's voice is the briefing's or the tutorial's",
          files and exact == lines == present,
          f"{lines} messages in the {len(files)} training missions: {exact} are bound "
          f"by briefing_sounds exactly when the briefing speaks them and by "
          f"tutorial_voices otherwise; {present} name a member of voices.lib")

    every = [(f, m) for f in sorted(game.rglob(briefing.MESSAGES))
             for m in briefing.messages(f)]
    flagged = [(f, m) for f, m in every if m.info_system]
    training = {f.parent for f in files}
    helps = sum("_H" in m.text_id.upper() for f, m in flagged if f.parent in training)
    later = sum(f.parent not in training for f, _ in flagged)
    check("progression: info_system marks the help lines and the later campaigns'",
          len(every) == briefing.MESSAGES_TOTAL
          and helps == sum("_H" in m.text_id.upper() for f, m in every if f.parent in training)
          and later == sum(f.parent not in training for f, _ in every)
          and len(flagged) == helps + later,
          f"{len(flagged)}/{len(every)} messages set it: all {helps} training lines "
          f"named _H, and all {later} messages outside the training campaign; no "
          f"training briefing or instructor line")

    strings = resources.strings((game / "iron3d.dll").read_bytes())
    game_voices: dict[str, str] = {}
    for d in resources.descriptors(game / "ui" / "game_resources.cfg"):
        game_voices.update({k: v.lower() for k, v in d.bindings.items()})
    endings = {k: game_voices.get(k) for k in PROGRESSION_VOICES}
    check("progression: iron3d.dll's words for objectives and repeated messages",
          all(strings.get(k) == v for k, v in PROGRESSION_STRINGS.items())
          and endings == PROGRESSION_VOICES,
          "; ".join(f"{k} {strings.get(k)!r}" for k in PROGRESSION_STRINGS)
          + f"; ui/game_resources.cfg voices {endings}")


#: The outcome panel's words, by the game's state word: its title, then its lines
#: (``iron3d.dll:0x1009f8b0``).
OUTCOME_STRINGS = {
    1012: "MISSION COMPLETE !",
    1013: "MISSION FAILED...",
    5082: "Press 'Esc' to continue",
    5083: "Exiting...",
    3075: "Press 'R' to restart mission",
    3076: "Press 'L' to load saved game",
    3077: "WARNING! YOU`RE THE SERVER. IF YOU KILL THE GAME YOU'LL KILL OTHER PLAYERS.",
    6225: "Multiplayer session lost",
}

#: The panel's colours, ARGB: the lines, the title of a win, of a failure, and the box.
OUTCOME_GREY, OUTCOME_GREEN, OUTCOME_RED, OUTCOME_BOX = (
    0xFFF0F0F0, 0xFF64FF64, 0xFFFF6464, 0x99000000)

#: The missions whose ``mission.cfg`` makes them a briefing alone.
ONLY_BRIEFING = ("CAMPAIGN/CAMPAIGN.01/Mission.01", "CAMPAIGN/CAMPAIGN.05/Mission.02")


def check_outcome(check, game: Path) -> None:
    """What follows a mission's outcome: its panel, the keys that leave it, briefings alone."""
    path = game / "iron3d.dll"
    if not path.exists():
        return
    iron = path.read_bytes()
    strings = resources.strings(iron)
    words = {k: strings.get(k) for k in OUTCOME_STRINGS}

    # The panel: the lines' grey loaded into ecx, then within the function the green
    # and red titles, the box's alpha, and a push of each string id it can show.
    grey = iron.find(b"\xb9" + struct.pack("<I", OUTCOME_GREY))
    body = iron[grey:grey + 0x400] if grey >= 0 else b""
    colours = [b"\xc7\x44\x24\x3c" + struct.pack("<I", c) for c in (OUTCOME_GREEN, OUTCOME_RED)]
    pushes = {k: body.find(b"\x68" + struct.pack("<I", k)) for k in OUTCOME_STRINGS}
    check("iron3d.dll: the outcome panel's words and colours",
          words == OUTCOME_STRINGS and grey >= 0
          and iron.count(b"\xb9" + struct.pack("<I", OUTCOME_GREY)) == 1
          and all(c in body for c in colours)
          and b"\x68" + struct.pack("<I", OUTCOME_BOX) in body
          and all(at >= 0 for at in pushes.values()),
          "; ".join(f"{k} {v!r}" for k, v in words.items())
          + f"; grey #{OUTCOME_GREY & 0xFFFFFF:06x} lines, green #{OUTCOME_GREEN & 0xFFFFFF:06x} "
          f"and red #{OUTCOME_RED & 0xFFFFFF:06x} titles, a box of alpha "
          f"{OUTCOME_BOX >> 24:#x}, each string pushed in "
          f"{sum(at >= 0 for at in pushes.values())}/{len(pushes)} of the panel's first 1024 bytes")

    # After a failure: `lea eax, [edi-0x4c]; cmp eax, 0x9e; ja; xor edx, edx; mov dl,
    # [eax + index]; jmp [edx*4 + cases]`, each case pushing an exit code.
    sections, _ = resources._sections(iron)
    lfanew = struct.unpack_from("<I", iron, 0x3C)[0]
    base = struct.unpack_from("<I", iron, lfanew + 24 + 28)[0]
    at = iron.find(b"\x8d\x47\xb4\x3d\x9e\x00\x00\x00\x77")
    by_code: dict[object, list[str]] = defaultdict(list)
    if at >= 0 and iron[at + 10:at + 14] == b"\x33\xd2\x8a\x90" \
            and iron[at + 18:at + 21] == b"\xff\x24\x95":
        index = struct.unpack_from("<I", iron, at + 14)[0]
        cases = struct.unpack_from("<I", iron, at + 21)[0]
        off = resources._offset(sections, index - base)
        for n, case in enumerate(iron[off:off + 0x9F]):
            target = struct.unpack_from(
                "<I", iron, resources._offset(sections, cases + 4 * case - base))[0]
            code = iron[resources._offset(sections, target - base):][:2]
            key = code[1] if code[0] == 0x6A else "other"
            by_code[key].append(bytes([0x4C + n]).decode("cp1251", "replace"))
    check("iron3d.dll: after a failure R restarts and L loads, on either keyboard layout",
          sorted(by_code.get(2, [])) == sorted("RrКк")
          and sorted(by_code.get(3, [])) == sorted("LlДд")
          and len(by_code.get("other", [])) == 0x9F - 8,
          f"exit code 2 for {''.join(by_code.get(2, []))}, 3 for {''.join(by_code.get(3, []))}, "
          f"{len(by_code.get('other', []))} other characters to the next handler")

    flagged = {}
    for d in gamedir.missions(game):
        cfg = d / "mission.cfg"
        found = mission.load_cfg(cfg) if cfg.exists() else {}
        value = found.get("mission", {}).get("only_briefing")
        flagged[d.relative_to(game / "MISSIONS").as_posix()] = value
    true = tuple(sorted(k for k, v in flagged.items() if v == "true"))
    absent = sorted(k for k, v in flagged.items() if v is None)
    check("missions: two campaign missions are a briefing alone",
          true == ONLY_BRIEFING and all(v in ("true", "false", None) for v in flagged.values()),
          f"only_briefing true on {', '.join(true)}; false on "
          f"{sum(v == 'false' for v in flagged.values())}; absent from {', '.join(absent)}")


#: ``ui/compaund.cfg``'s pieces as ``iron3d.dll:0x100989b0`` files them, by slot (the
#: offset into the skin over 0x8c).  Slot 13 is never filled; the radio buttons are never
#: read, and two long buttons are read twice.
HUD_SKIN = {
    0: "ccres_red_lamp", 1: "ccres_yellow1_lamp", 2: "ccres_yellow2_lamp",
    3: "ccres_green_lamp", 4: "ccres_black_lamp", 5: "ccres_ray_emitter_off",
    6: "ccres_ray_emitter_normal", 7: "ccres_ray_emitter_pressed", 8: "ccres_ray_ending",
    9: "ccres_ray_body", 10: "ccres_exit_button_off", 11: "ccres_exit_button_normal",
    12: "ccres_exit_button_pressed", 14: "ccres_short_button_off",
    15: "ccres_short_button_normal", 16: "ccres_short_button_pressed",
    17: "ccres_long_button_off", 18: "ccres_long_button_normal",
    19: "ccres_long_button_pressed", 20: "ccres_body_stub", 21: "ccres_body_text",
    22: "ccres_long_button_off", 23: "ccres_long_button_normal", 24: "ccres_ending_stub",
    25: "ccres_ending_text", 26: "ccres_lamp_stub_ending_off",
    27: "ccres_lamp_stub_ending_normal", 28: "ccres_lamp_stub_ending_pressed",
    29: "ccres_lamp_text_ending_off", 30: "ccres_lamp_text_ending_normal",
    31: "ccres_lamp_text_ending_pressed", 32: "ccres_separator_left_text",
    33: "ccres_separator_double_text", 34: "ccres_frame_corner_1", 35: "ccres_frame_corner_2",
    36: "ccres_frame_corner_3", 37: "ccres_frame_corner_4", 38: "ccres_frame_edge_v",
    39: "ccres_frame_edge_h",
}

#: The skin's lookup (``iron3d.dll:0x10098850``): kind -> (its first slot, whether the
#: variant is added to it).  Any other kind gets slot 24.
HUD_SKIN_KINDS = {
    0: (24, True), 1: (26, True), 2: (29, True), 3: (32, True), 4: (20, True), 5: (5, True),
    6: (9, False), 7: (8, False), 8: (0, True), 9: (34, True), 10: (38, True),
    11: (10, True), 12: (14, True), 13: (17, True), 14: (22, True), 15: (40, False),
}

#: A weapon row's pen primitives, right to left from x 640 (``iron3d.dll:0x1009d4a1``):
#: each primitive's address, the skin kind it draws, and the width or variant its call is
#: given (None: a register holding 0).
WEAPON_ROW = (
    (0x10099A30, 0, 1),     # ccres_ending_text
    (0x10099F60, 4, 12),    # the key box, body_text
    (0x1009A8F0, 8, None),  # the lamp, by state
    (0x10099F60, 4, 24),    # the rounds box
    (0x10099C90, 3, None),  # ccres_separator_left_text
    (0x10099D80, 5, None),  # ccres_ray_emitter_off
    (0x1009A380, 6, 120),   # the name bar, ray_body
    (0x10099E70, 7, None),  # ccres_ray_ending
)

#: The lamp a selected gun shows for each of its report words (``iron3d.dll:0x1009d8cc``):
#: kind 8's variant (0 red, 1 yellow1, 2 yellow2, 3 green) and whether the name becomes
#: string 6250.
WEAPON_LAMPS = {0: (3, False), 1: (2, False), 2: (2, True), 3: (2, False), 4: (1, False),
                5: (0, False), 6: (1, False), 7: (2, True), 8: (2, True)}

#: The strings the weapons list and the message box show, by id.
HUD_TOP_STRINGS = {
    3071: "AUTOCANNON 25mm", 3072: "PLASMA RIFLE LS", 3073: "BATTLE LASER ER",
    3074: "AWB MISSILE", 5094: "INF", 6250: "OUT OF RANGE", 6171: "from: %s",
    1541: "from: System", 3057: "from: Training assistant",
    6214: "from: Information assistant", 6208: "Press %s to see more",
}

#: The message box's header by message kind (``iron3d.dll:0x1007f750``).
MESSAGE_HEADERS = {1: 6171, 2: 1541, 3: 3057, 4: 6214}


def _image_at(image: bytes):
    """A reader of ``image``'s bytes by virtual address, and the image base."""
    sections, _ = resources._sections(image)
    lfanew = struct.unpack_from("<I", image, 0x3C)[0]
    base = struct.unpack_from("<I", image, lfanew + 24 + 28)[0]

    def at(va: int, n: int) -> bytes:
        off = resources._offset(sections, va - base)
        return image[off:off + n]

    return at


def _calls(at, start: int, size: int) -> list[tuple[int, int]]:
    """Every ``call rel32`` in ``size`` bytes from ``start``: (its address, its target).
    A byte scan, so a stray 0xe8 inside another instruction can add a false one."""
    body = at(start, size)
    return [(start + i, (start + i + 5 + struct.unpack_from("<i", body, i + 1)[0]) & 0xFFFFFFFF)
            for i in range(len(body) - 5) if body[i] == 0xE8]


def _pushed_before(at, call: int, back: int = 8) -> int | None:
    """The last ``push imm8`` in the ``back`` bytes before ``call``."""
    body = at(call - back, back)
    found = [body[i + 1] for i in range(len(body) - 1) if body[i] == 0x6A]
    return found[-1] if found else None


def check_hud_top(check, game: Path) -> None:
    """The cockpit HUD's weapons list and message box: the skin, the row, the box."""
    path = game / "iron3d.dll"
    if not path.exists():
        return
    iron = path.read_bytes()
    at = _image_at(iron)
    strings = resources.strings(iron)

    # The skin: `compaund.cfg`'s pieces on ui_menu, every one on its page and holding art.
    cfg = resources.load_cfg(game / "ui" / "compaund.cfg")
    roles = {d.role: d.bindings for d in resources.descriptors(game / "ui" / "game_resources.cfg")}
    pages = roles.get("textures", {})
    ui = NResArchive.open(game / "ui" / "ui.lib")
    entries = list(ui)
    decoded: dict[int, object] = {}
    placed = []
    for name, p in cfg.items():
        index = int(pages.get(p.get("texture", ""), -1))
        if not 0 <= index < len(entries):
            placed.append((name, None, False))
            continue
        if index not in decoded:
            decoded[index] = texm.decode(ui.read(entries[index]))
        tex = decoded[index]
        x, y, w, h = (int(p[k]) for k in ("offset_x", "offset_y", "width", "height"))
        inside = x >= 0 and y >= 0 and x + w <= tex.width and y + h <= tex.height
        inked = inside and sum(tex.rgba[(yy * tex.width + xx) * 4 + 3] > 0
                               for yy in range(y, y + h) for xx in range(x, x + w)) >= w * h / 4
        placed.append((name, entries[index].name, inked))
    on_pages = {page for _, page, _ in placed}
    check("ui/compaund.cfg: 39 compound-control pieces, all holding art on ui_menu1",
          len(placed) == 39 and on_pages == {"ui_menu1.tex"} and all(ok for *_, ok in placed),
          f"{len(placed)} pieces on {sorted(map(str, on_pages))} ('ui_menu' is "
          f"game_resources.cfg's entry {pages.get('ui_menu')}); "
          f"{sum(ok for *_, ok in placed)} lie inside the page with alpha on a quarter of "
          f"their pixels or more")

    # The loader: each `push name` then `lea ecx, [esi + slot * 0x8c]` (or `mov ecx, esi`)
    # before its call to the piece loader 0x1008f450.
    loader_start, loader_size = 0x100989B0, 0xFB4
    body = at(loader_start, loader_size)
    loads = {c: t for c, t in _calls(at, loader_start, loader_size)}
    slots: dict[int, str] = {}
    for i in range(len(body) - 5):
        if body[i] != 0x68:
            continue
        va = struct.unpack_from("<I", body, i + 1)[0]
        try:
            text = at(va, 40).split(b"\0")[0]
        except resources.ResourceFormatError:
            continue
        if not text.startswith(b"ccres_"):
            continue
        call = next((c for c in sorted(loads) if c > loader_start + i
                     and loads[c] == 0x1008F450), None)
        if call is None:
            continue
        span = at(loader_start + i, call - loader_start - i)
        lea = span.find(b"\x8d\x8e")
        if lea >= 0:
            slot, rest = divmod(struct.unpack_from("<I", span, lea + 2)[0], 0x8C)
        elif b"\x8b\xce" in span:
            slot, rest = 0, 0
        else:
            continue
        if rest == 0:
            slots[slot] = text.decode()
    named = set(HUD_SKIN.values())
    check("iron3d.dll: the skin loader files 37 of compaund.cfg's pieces in 39 slots",
          slots == HUD_SKIN and named <= set(cfg) and len(named) == 37
          and set(cfg) - named == {"ccres_radio_button_off", "ccres_radio_button_on"},
          f"0x100989b0 fills slots {min(slots, default=None)}-{max(slots, default=None)} "
          f"less {sorted(set(range(40)) - set(slots))}; {len(named)} distinct names, "
          f"never {sorted(set(cfg) - named)}; the long buttons' off and normal twice")

    # The lookup: `cmp eax, 0xf; ja; jmp [eax*4 + table]`, each case a slot base.
    head = at(0x10098850, 16)
    kinds: dict[int, tuple[int, bool]] = {}
    if head[:7] == b"\x8b\x44\x24\x04\x83\xf8\x0f" and head[13:16] == b"\xff\x24\x85":
        table = struct.unpack_from("<I", at(0x10098850 + 16, 4))[0]
        for kind in range(16):
            case = at(struct.unpack_from("<I", at(table + 4 * kind, 4))[0], 16)
            if case[:2] == b"\x8d\x81":
                kinds[kind] = (struct.unpack_from("<I", case, 2)[0] // 0x8C, False)
            elif case[:4] in (b"\x8b\x44\x24\x08", b"\x8b\x54\x24\x08"):
                if case[4:6] == b"\x69\xc0":
                    kinds[kind] = (0, True)
                elif case[4:6] in (b"\x83\xc0", b"\x8d\x42"):
                    kinds[kind] = (case[6], True)
    check("iron3d.dll: the skin's lookup turns a kind and a variant into a slot",
          kinds == HUD_SKIN_KINDS,
          "0x10098850: " + ", ".join(
              f"{k} {HUD_SKIN.get(s, s) if not v else f'{s}+v'}" for k, (s, v) in kinds.items()))

    # The weapon row: the primitives' order and widths, and the kind each one draws.
    row_calls = [(c, t) for c, t in _calls(at, 0x1009D4A1, 0x1009D5B6 - 0x1009D4A1)
                 if t in {p for p, _, _ in WEAPON_ROW}]
    got_row = []
    every = [c for c, _ in _calls(at, 0x1009D4A1, 0x1009D5B6 - 0x1009D4A1)]
    for call, target in row_calls:
        previous = max((c for c in every if c < call), default=call - 24)
        prim = next((c, t2) for c, t2 in _calls(at, target, 0x140)
                    if t2 in (0x10098850, 0x1009B470))
        kind = _pushed_before(at, prim[0])
        if kind is None and at(prim[0] - 1, 1) == b"\x56":  # push esi, cleared on entry
            kind = 0
        got_row.append((target, kind, _pushed_before(at, call, min(24, call - previous - 5))))
    matches = len(got_row) == len(WEAPON_ROW) and all(
        (t, k, v) == (pt, pk, pv)
        for (t, k, v), (pt, pk, pv) in zip(got_row, WEAPON_ROW, strict=False))
    holder = at(0x1003ECB0, 0xA9)
    row = at(0x1009CD30, 0xB9A)
    check("iron3d.dll: a weapon row is eight pieces right to left from x 640, rows 19 apart",
          matches and b"\x68\x80\x02\x00\x00" in holder
          and b"\x83\xc3\x13" in holder and b"\xc7\x44\x24\x20\x80\x02\x00\x00" in row,
          "0x1009cd30's pen: " + "; ".join(
              f"{t:#x} kind {k} given {v}" for t, k, v in got_row)
          + "; the holder 0x1003ecb0 steps y by 0x13 a gun")

    # The lamps: `jmp [eax*4 + 0x1009d8cc]` over the report word, each case a state.
    jump = row.find(b"\xff\x24\x85")
    lamps: dict[int, tuple[int, bool]] = {}
    if jump >= 0:
        table = struct.unpack_from("<I", row, jump + 3)[0]
        for report in range(9):
            case_va = struct.unpack_from("<I", at(table + 4 * report, 4))[0]
            case = at(case_va, 0x40)
            mov = case.find(b"\xc7\x44\x24\x18")
            if 0 <= mov < 12:
                # `mov [esp+0x18], state`, and on the out-of-range cases `mov [esp+0x38],
                # 0xffff5c5c` next, before the push of 6250.
                state = struct.unpack_from("<I", case, mov + 4)[0]
                red = case[mov + 8:mov + 16] == b"\xc7\x44\x24\x38\x5c\x5c\xff\xff" \
                    and b"\x68\x6a\x18\x00\x00" in case[mov + 16:mov + 32]
            else:  # `mov [esp+0x18], ebx`, zero
                state = 0 if case.find(b"\x89\x5c\x24\x18") in range(12) else None
                red = False
            lamps[report] = (state, red)
    black = b"\xc7\x44\x24\x18\x04\x00\x00\x00" in row
    check("iron3d.dll: a selected gun's lamp by its report word, OUT OF RANGE on 2, 7 and 8",
          lamps == WEAPON_LAMPS and black,
          "0x1009d8cc: " + ", ".join(
              f"{r} {['red', 'yellow1', 'yellow2', 'green', 'black'][s] if s is not None else s}"
              f"{' +6250' if o else ''}" for r, (s, o) in lamps.items())
          + f"; an unselected gun's lamp black (state 4): {black}")

    # The bar: three fill colours at 20 and 80 per cent, in either direction.
    bar = at(0x1009A380, 0x420)
    colours = [c for c in (0x80800000, 0x80808000, 0x80008000)
               if b"\xb9" + struct.pack("<I", c) in bar and b"\xbe" + struct.pack("<I", c) in bar]
    names_pushed = at(0x10074AF0, 0x1C0)
    hero_names = [k for k in (3071, 3072, 3073, 3074)
                  if b"\xba" + struct.pack("<I", k) in names_pushed]
    words = {k: strings.get(k) for k in HUD_TOP_STRINGS}
    check("iron3d.dll: the row's words, and a charge bar red under 20%, olive under 80%",
          len(colours) == 3 and bar.count(b"\x83\xf8\x14") == 4 and bar.count(b"\x83\xf8\x50") == 2
          and hero_names == [3071, 3072, 3073, 3074] and b"\x68\xe6\x13\x00\x00" in row
          and b"\x68\x6a\x18\x00\x00" in row and b"\x83\xfe\xff" in row
          and words == HUD_TOP_STRINGS,
          f"0x1009a380 fills in {[hex(c) for c in colours]}; the hero's guns named "
          f"{[words.get(k) for k in hero_names]} at 0x10074af0; 5094 {words.get(5094)!r} for "
          f"a magazine of -1, 6250 {words.get(6250)!r}")

    # The message box: its headers, its place and size, its colours, its life.
    ctor = at(0x1007F750, 0x250)
    jump = ctor.find(b"\xff\x24\x85")
    headers: dict[int, int | None] = {}
    if jump >= 0:
        table = struct.unpack_from("<I", ctor, jump + 3)[0]
        for kind in (1, 2, 3, 4):
            case = at(struct.unpack_from("<I", at(table + 4 * (kind - 1), 4))[0], 0x20)
            push = re.search(rb"\x68(..)\x00\x00", case, re.S)
            headers[kind] = struct.unpack_from("<H", push.group(1))[0] if push else None
    draw = at(0x1007FE80, 0x366)
    wrap = at(0x1007FAD0, 0x3A8)
    tick = at(0x1007F4F0, 0x4A)
    life_at = tick.find(b"\xd8\x1d")
    life = struct.unpack_from("<f", at(struct.unpack_from("<I", tick, life_at + 2)[0], 4))[0] \
        if life_at >= 0 else None
    geometry = all(p in draw for p in (
        b"\x81\xe2\x90\x00\x00\x00\x81\xc2\xe6\x00\x00\x00",  # x 230, or 374
        b"\x81\xe1\x60\x01\x00\x00",                          # y 0, or 352
        b"\x83\xe0\x54\x05\xb6\x00\x00\x00",                  # width 182, or 266
        b"\x8d\x44\x10\x10",                                  # the height's + 16
        b"\xb9\x00\x80\x00\x80",                              # the fill, 0x80008000
        b"\x68\xdc\xdc\xdc\xff",                              # the lines' #dcdcdc
        b"\x83\xc7\x08", b"\x83\xc6\x09"))                    # the text at +8, +9
    text = all(p in wrap for p in (
        b"\x8d\x47\xf6",           # wrapped to the width less 10
        b"\x83\xfb\x06", b"\x6a\x06",  # at most six lines
        b"\x68\xed\x02\x00\x00",   # the key bound to CMD_HELP, 749
        b"\x68\x40\x18\x00\x00"))  # 6208, the footer
    keys = {b.command: b.chord for b in controls.bindings(game / "addition.man")}
    check("iron3d.dll: the message box's headers, place, lines and 20 seconds",
          headers == MESSAGE_HEADERS and geometry and text and life == 20.0
          and keys.get("CMD_PAGER") == "SCAN_F2" and keys.get("CMD_HELP") == "SCAN_F1",
          f"kinds {', '.join(f'{k}: {strings.get(v) if v else v!r}' for k, v in headers.items())}; "
          f"0x1007fe80 places it at x 230 y 0 w 182 (x 374 y 352 w 266 wide) filled 0x80008000 "
          f"{geometry}; 0x1007fad0 wraps at w - 10 to six lines {text}; 0x1007f4f0 keeps it "
          f"{life} s; CMD_PAGER on {keys.get('CMD_PAGER')}, CMD_HELP on {keys.get('CMD_HELP')}")

    voices = next((d for d in resources.descriptors(game / "ui" / "game_resources.cfg")
                   if d.role == "voices"), None)
    wanted = ("VOICE_WEAPON_DESTR", "VOICE_WEAP_AMMO_OUT", "VOICE_WEAP_ENERGY_OUT")
    bound = {v: voices.bindings.get(v) if voices else None for v in wanted}
    named_in = [v for v in wanted if v.encode() + b"\0" in iron]
    check("game_resources.cfg: the three weapon voices the row plays",
          all(bound.values()) and named_in == list(wanted),
          f"{bound}; each name a string in iron3d.dll: {len(named_in)}/3")


#: The radar's sprites, cut in this order by its constructor (``iron3d.dll:0x1003f340``):
#: the page the doc names, then x, y, w, h.  Then the gauges' bar art, cut each frame, and
#: page7's fan, which the wedge takes on its own UVs.
RADAR_CUTS = (("page6", 156, 0, 99, 153), ("page6", 155, 0, 100, 153), ("page6", 192, 154, 9, 11),
              ("ui_menu3", 215, 0, 15, 28), ("ui_menu3", 199, 0, 15, 28))
RADAR_ART = (("page6", 121, 0, 31, 105), ("page7", 128, 0, 128, 64))
#: Where ``0x1003fb90`` draws the halves and the two icons: (x0, y0, x1, y1), white.
RADAR_DRAWS = ((320, 327, 221, 480), (320, 327, 421, 480), (230, 383, 245, 411),
               (396, 383, 411, 411))
#: The floats the radar's draw reads: the wedge's radius, its fan's radius, centre and
#: texel, the arrows' ring, the contacts' disc, 5 degrees and the angles, the ring's period
#: factor, the range figure's box, the halving, and the marks' sizes.
RADAR_FLOATS = (75.0, 64.0, 192.0, 1 / 256, 68.0, 56.0, 60.0, math.radians(5.0), 4.71228,
                -4.71228, math.pi / 2, math.pi, 0.002, 37.0, 303.0, 0.5, 5.0, 4.0, 3.0)
#: The gauges' floats (``0x1003f6a0``, ``0x1003f900``): the box, its two x, and km/h.
GAUGE_FLOATS = (28.0, 226.0, 388.0, 3.6, 0.5)
#: The two gauges' bars (x0, x1, and the colour; y from 347 + e to 452).
GAUGE_BARS = ((271, 240, 0xFFFFB450), (370, 401, 0xFFFFB450))

#: The indicators' sprites (``0x1003ed60``): three lamps, then eight icons by slot.
INDICATOR_CUTS = (("page6", 202, 154, 17, 20), ("page6", 220, 154, 17, 20),
                  ("page6", 238, 154, 17, 20),
                  ("ui_menu", 97, 126, 15, 15), ("ui_menu", 238, 222, 15, 15),
                  ("ui_menu", 239, 126, 15, 15), ("ui_menu", 223, 142, 15, 15),
                  ("ui_menu3", 180, 27, 15, 15), ("ui_menu", 113, 94, 15, 15),
                  ("ui_menu", 113, 110, 15, 15), ("ui_menu", 81, 94, 15, 15))
#: Each slot's x (``0x1003f270``).
INDICATOR_X = (222, 240, 258, 280, 344, 366, 384, 402)

#: The reticle's sprites (``0x10042d10``) and where ``0x10042ed0`` draws them, in #37ff37.
RETICLE_CUTS = (("page9", 168, 0, 82, 15), ("page9", 77, 0, 64, 64), ("page9", 54, 28, 23, 23))
RETICLE_DRAWS = ((288, 208, 352, 272), (279, 220, 300, 199), (279, 260, 300, 281),
                 (361, 220, 340, 199), (361, 260, 340, 281), (232, 233, 314, 248),
                 (408, 233, 326, 248))

_HUD_PUSH = rb"(?:\x53|\x6a.|\x68.{4})"
#: A cut's call: turn (then maybe `mov byte [esp+d], 0`), h, w, y, x; 256.0 and the page;
#: `this` into ecx, maybe `mov [esp+d], bl`; the call.
_HUD_CUT = re.compile(
    rb"(" + _HUD_PUSH + rb"(?:\xc6\x44\x24.\x00)?" + _HUD_PUSH + rb"{4})"
    rb"\x68\x00\x00\x80\x43[\x50-\x57](?:\x8d\x4e.|\x8d\x8e.{4}|\x8b[\xc8-\xcf])"
    rb"(?:\x88\x5c\x24.)?\xe8(.{4})", re.S)
#: A draw's call with the stage 0, blend 1 and black specular every HUD sprite here takes:
#: the colour, then y1, x1, y0, x0 (each maybe followed by a `lea esi`), `this`, the call.
_HUD_DRAW = re.compile(
    rb"\x6a\x00\x6a\x01\x68\x00\x00\x00\xff(\x6a.|\x68.{4})"
    rb"((?:(?:\x68.{4}|\x6a.)(?:\x8d\xb7.{4}|\x8d\x77.)?){4})"
    rb"(?:\x8d\x4e.|\x8d\x8e.{4}|\x8d\x8f.{4}\x89\x54\x24.\x89\x44\x24.|\x8b[\xc8-\xcf])"
    rb"\xe8(.{4})", re.S)


def _push_run(run: bytes, floats: bool) -> list | None:
    """A run of pushes' values in push order: `push ebx` (zero in these widgets) is 0, an
    imm8 is sign-extended, an imm32 an int or a float; the fillers the patterns allow are
    stepped over."""
    out: list = []
    pos = 0
    while pos < len(run):
        op = run[pos]
        if op == 0x53:
            out.append(0)
            pos += 1
        elif op == 0x6A:
            out.append(struct.unpack_from("<b", run, pos + 1)[0])
            pos += 2
        elif op == 0x68:
            out.append(struct.unpack_from("<f" if floats else "<i", run, pos + 1)[0])
            pos += 5
        elif run[pos:pos + 3] == b"\xc6\x44\x24":
            pos += 5
        elif run[pos:pos + 2] == b"\x8d\xb7":
            pos += 6
        elif run[pos:pos + 2] == b"\x8d\x77":
            pos += 3
        else:
            return None
    return out


def _hud_sprites(at, start: int, size: int) -> tuple[list, list]:
    """A function's sprite cuts (x, y, w, h, turn) and draws (x0, y0, x1, y1, colour), in
    order, from its calls to ``0x1008f830`` and ``0x1008f970``."""
    body = at(start, size)
    cuts, draws = [], []
    for m in _HUD_CUT.finditer(body):
        call = start + m.end() - 5
        if (call + 5 + struct.unpack("<i", m.group(2))[0]) & 0xFFFFFFFF != 0x1008F830:
            continue
        values = _push_run(m.group(1), True)
        if values and len(values) == 5:
            turn, h, w, y, x = values
            cuts.append((round(x), round(y), round(w), round(h), round(turn)))
    for m in _HUD_DRAW.finditer(body):
        call = start + m.end() - 5
        if (call + 5 + struct.unpack("<i", m.group(3))[0]) & 0xFFFFFFFF != 0x1008F970:
            continue
        colour = _push_run(m.group(1), False)
        corners = _push_run(m.group(2), False)
        if colour and corners and len(corners) == 4:
            y1, x1, y0, x0 = corners
            draws.append((x0, y0, x1, y1, colour[0] & 0xFFFFFFFF))
    return cuts, draws


def _floats_read(at, start: int, size: int) -> list[float]:
    """The float32 constants a function's x87 ops read by absolute address."""
    out = []
    for m in re.finditer(rb"[\xd8\xd9]([\x05\x0d\x15\x1d\x25\x2d\x35\x3d])(.{4})",
                         at(start, size), re.S):
        try:
            out.append(struct.unpack("<f", at(struct.unpack("<I", m.group(2))[0], 4))[0])
        except (resources.ResourceFormatError, struct.error):
            continue
    return out


def _hud_art(game: Path):
    """A function giving, for a page name and a rect, the page's entry name and whether the
    rect lies on the page with alpha on at least a sixteenth of its pixels."""
    roles = {d.role: d.bindings for d in resources.descriptors(game / "ui" / "game_resources.cfg")}
    pages = roles.get("textures", {})
    ui = NResArchive.open(game / "ui" / "ui.lib")
    entries = list(ui)
    decoded: dict[int, object] = {}

    def art(page: str, x: int, y: int, w: int, h: int) -> tuple[str | None, bool]:
        index = int(pages.get(page, -1))
        if not 0 <= index < len(entries):
            return None, False
        if index not in decoded:
            decoded[index] = texm.decode(ui.read(entries[index]))
        tex = decoded[index]
        if x < 0 or y < 0 or x + w > tex.width or y + h > tex.height:
            return entries[index].name, False
        inked = sum(tex.rgba[(yy * tex.width + xx) * 4 + 3] > 0
                    for yy in range(y, y + h) for xx in range(x, x + w))
        return entries[index].name, inked * 16 >= w * h

    return art


def check_hud_radar(check, game: Path) -> None:
    """The cockpit HUD's radar, its gauges, the indicators under it and the reticle."""
    path = game / "iron3d.dll"
    services = game / "services.dll"
    if not path.exists() or not services.exists():
        return
    iron = path.read_bytes()
    at = _image_at(iron)
    svc = _image_at(services.read_bytes())
    art = _hud_art(game)

    # The two scales: the display mode's record keeps width x 1/640 and height x 1/480, and
    # IDisplay slots 4 and 5 return them.
    mode = re.search(rb"\xd8\x0d(.{4})\xd9\x58\x10\xd8\x0d(.{4})\xd9\x58\x14", svc(0x10004610, 64),
                     re.S)
    scales = [struct.unpack("<f", svc(struct.unpack("<I", g)[0], 4))[0] for g in mode.groups()] \
        if mode else []
    vtable = [struct.unpack("<I", svc(0x1003A1F8 + 4 * i, 4))[0] for i in (4, 5)]
    getters = [svc(v, 7) for v in vtable]
    gui = [struct.unpack("<I", svc(0x1003A198 + 4 * i, 4))[0] for i in range(7)]
    circle, fill = svc(gui[2], 0x150), svc(gui[4], 0x130)
    check("services.dll: the HUD's 640 x 480 is stretched by width/640 and height/480",
          len(scales) == 2 and math.isclose(scales[0] * 640, 1, rel_tol=1e-5)
          and math.isclose(scales[1] * 480, 1, rel_tol=1e-5)
          and getters == [b"\xd9\x81\x14\x05\x00\x00\xc3", b"\xd9\x81\x18\x05\x00\x00\xc3"]
          and b"\xd9\x40\x10\xd9\x99\x14\x05\x00\x00\xd9\x40\x14\xd9\x99\x18\x05\x00\x00"
          in svc(0x10004ec0, 0x40)
          and b"\x6a\x10\x68\xe0\x01\x00\x00\x68\x80\x02\x00\x00" in svc(0x10004462, 16)
          and svc(gui[0], 7) == b"\x8a\x44\x24\x04\x88\x41\x04"
          and b"\x6a\x15\x52\x6a\x0d\x6a\x03" in circle and b"\x6a\x04\x52\x6a\x0c\x6a\x05" in fill,
          f"0x10004610 keeps x and y by {[round(1 / s, 3) for s in scales if s]}ths; slots 4 "
          f"and 5 ({[hex(v) for v in vtable]}) read +0x514 and +0x518, which a mode change "
          f"copies; the default mode 640 x 480; the GUI server's slot 0 keeps the scaling flag,"
          f" slot 2 draws a 21-point line strip, slot 4 a 4-point triangle strip")

    # The radar: its cuts, where it draws them, and the art under each rect.
    cuts, _ = _hud_sprites(at, 0x1003F340, 0x35C)
    _, draws = _hud_sprites(at, 0x1003FB90, 0x200)
    placed = [art(*r) for r in RADAR_CUTS + RADAR_ART]
    centre = b"\xc7\x46\x0c\x00\x00\xa0\x43\xc7\x46\x10\x00\x00\xc6\x43" in at(0x1003F340, 0x80)
    check("iron3d.dll: the radar's disc, icons and arrow, cut and placed about (320, 396)",
          [c[:4] for c in cuts] == [r[1:] for r in RADAR_CUTS] and all(c[4] == 0 for c in cuts)
          and [d[:4] for d in draws[:4]] == list(RADAR_DRAWS)
          and all(d[4] == 0xFFFFFFFF for d in draws[:4]) and centre
          and [p for p, _ in placed] == ["ui_tex6.tex"] * 3 + ["ui_menu3.tex"] * 2
          + ["ui_tex6.tex", "ui_tex7.tex"] and all(ok for _, ok in placed),
          f"0x1003f340 cuts {cuts}; 0x1003fb90 draws {draws[:4]}; centre (320, 396) held "
          f"{centre}; pages {[p for p, _ in placed]}, every rect on its page and inked: "
          f"{sum(ok for _, ok in placed)}/{len(placed)}")

    # The radar's draw: its constants, its colours, and what it asks the unit.
    body = at(0x1003FB90, 0xC90)
    floats = _floats_read(at, 0x1003FB90, 0xC90)
    missing = [f for f in RADAR_FLOATS if not any(math.isclose(f, g, rel_tol=1e-5) for g in floats)]
    pushed = {name: b"\x68" + struct.pack("<I", v) in body for name, v in (
        ("north 0xff0a0ae1", 0xFF0A0AE1), ("south 0xffe10a0a", 0xFFE10A0A),
        ("ring 0x8c009b00", 0x8C009B00), ("range 0xff00ff00", 0xFF00FF00),
        ("chosen 0xffff00ff", 0xFFFF00FF), ("period id 0xa9", 0xA9), ("range y 467", 0x1D3))}
    shape = at(0x10075F70, 28) == bytes.fromhex(
        "8b414485c074188b08680702000050ff51688b0833d283f9010f94c2")
    wedge = b"\x6a\x03\x8d\x86\x60\x03\x00\x00\x50\x6a\x0d\x6a\x05" in body \
        and b"\xd9\x84\x24\xc8\x00\x00\x00" in body and b"\xba\x06\x00\x00\x00" in body
    sounds = next((d.bindings for d in resources.descriptors(game / "ui" / "game_resources.cfg")
                   if d.role == "sounds"), {})
    check("iron3d.dll: the radar's wedge, north and south, contacts, ring and range figure",
          not missing and all(pushed.values()) and shape and wedge
          and b"RADAR\0" in iron and sounds.get("RADAR", "").lower() == "i_radar.wav"
          and b"\x6a\x50" in at(0x10091B30, 0x20),
          f"0x1003fb90 reads {len(floats)} floats, lacking {missing}; pushes {pushed}; "
          f"0x10075f70 tests the behaviour's variable 0x207 for 1: {shape}; the wedge is a "
          f"3-vertex fan from the camera's parameters +0x14 x 0.5: {wedge}; RADAR is "
          f"{sounds.get('RADAR')}; 0x10091b30 asks property 0x50")

    # The gauges: the figures' boxes, km/h, the bars' art, place and colour, and the sums.
    gauges = at(0x1003F6A0, 0x4F0)
    bars = [(struct.unpack("<i", m.group(5))[0], struct.unpack("<i", m.group(3))[0],
             struct.unpack("<I", m.group(1))[0], struct.unpack("<i", m.group(2))[0],
             struct.unpack("<I", m.group(4))[0])
            for m in re.finditer(rb"\x68(.{4})\x68(.{4})\x68(.{4})\x81\xc6(.{4})\x56\x68(.{4})"
                                 rb"\x8b\xcf\xe8", gauges, re.S)]
    cut_art = len(re.findall(rb"\x68\x00\x00\xf8\x41.{0,12}\x68\x00\x00\xf2\x42"
                             rb"\x68\x00\x00\x80\x43\x52\xe8", gauges, re.S))
    gfloats = _floats_read(at, 0x1003F6A0, 0x4F0)
    gmissing = [f for f in GAUGE_FLOATS
                if not any(math.isclose(f, g, rel_tol=1e-5) for g in gfloats)]
    sums = all(p in gauges for p in (
        bytes.fromhex("8d4e646bc964b81f85eb51f7e9c1fa06"),  # (a + 100) x 100 / 200
        bytes.fromhex("8bc66bc06499f7f9"),                  # v x 100 / top
        bytes.fromhex("d981d80a0000"), bytes.fromhex("d9400c"),  # the world's W, the record's z
        b"\x6a\x28", b"\x68\x90\x00\x00\x00", b"\x6a\xff\x68\xbe\x01\x00\x00"))
    check("iron3d.dll: altitude above the world's +0xad8 and speed in km/h, bars of 105",
          [(b[0], b[1], b[2]) for b in bars] == list(GAUGE_BARS)
          and all(b[3] == 452 and b[4] == 347 for b in bars) and cut_art == 2 and not gmissing
          and sums and gauges.count(bytes.fromhex("b9690000002bce")) == 2
          and gauges.count(b"\x6b\xc9\x69") == 2,
          f"bars (x0, x1, colour, y1, y0 +) {[(b[0], b[1], hex(b[2]), b[3], b[4]) for b in bars]}; "
          f"page6 (121, e) 31 wide cut {cut_art} times; floats lacking {gmissing}; the sums, "
          f"properties 0x28 and 0x90, white figures at y 446: {sums}")

    # What the figures ask: property 0xa9 is the radar's period and 0x90 the forward top
    # speed (Control.dll's two getters' case tables).
    ctl = game / "Control.dll"
    period = top = None
    if ctl.exists():
        cat = _image_at(ctl.read_bytes())
        machine = re.search(rb"\x8d\x41\xda\x3d\x8d\x00\x00\x00\x0f\x87.{4}\x33\xd2\x8a\x90(.{4})"
                            rb"\xff\x24\x95(.{4})", cat(0x1000E6C0, 0x40), re.S)
        base_get = re.search(rb"\x48\x3d\xb3\x00\x00\x00\x0f\x87.{4}\x33\xc9\x8a\x88(.{4})"
                             rb"\xff\x24\x8d(.{4})", cat(0x1000DCC0, 0xA0), re.S)

        def case(match, bias: int, prop: int) -> bytes:
            index, table = (struct.unpack("<I", g)[0] for g in match.groups())
            slot = cat(index + prop - bias, 1)[0]
            return cat(struct.unpack("<I", cat(table + 4 * slot, 4))[0], 40)

        if machine and base_get:
            period = b"\xba\x09\x00\x00\x00" in case(machine, 0x26, 0xA9)
            top = b"\x8b\x91\x6c\x04\x00\x00\x83\xc2\x1c" in case(base_get, 1, 0x90)
    check("Control.dll: property 0xa9 is the radar's period (id 9), 0x90 the top speed at +48",
          period is True and top is True,
          f"the machine getter 0x1000e6c0 sends 0xa9 to the device getter as 9: {period}; the "
          f"base getter 0x1000dcc0 answers 0x90 with the +0x46c block's +0x1c, file +48: {top}")

    # The altitude's zero: ITerrain slot 11 returns the landscape's +0x7cdc, the first water
    # face's first vertex z (-1 without one), which the world keeps at +0xad8.
    terrain = game / "Terrain.dll"
    zero = False
    if terrain.exists():
        t_image = terrain.read_bytes()
        tat = _image_at(t_image)
        slot11 = tat(struct.unpack("<I", tat(0x1009A438 + 44, 4))[0], 16)
        zero = slot11 == bytes.fromhex("558bec51894dfc8b45fcd980a87b0000") \
            and bytes.fromhex("c781dc7c0000000080bf") in t_image \
            and bytes.fromhex("250000020085c0") in t_image \
            and bytes.fromhex("8b42088981dc7c0000") in t_image and 0x7CDC - 0x7BA8 == 0x134
    kept = re.search(rb"\xba\x05\x00\x00\x00\xff\x10\x85\xc0\x74.\x8b\x4c\x24.\x8b\x01\xff\x50\x2c"
                     rb"\xd9\x9e\xd8\x0a\x00\x00", iron, re.S) is not None
    tut = game / "DATA" / "MAPS" / "Tut_1" / "Land.msh"
    level = landmesh.load(tut).water_level() if tut.exists() else None
    check("Terrain.dll: the altitude's zero is the water plane; Tut_1's at -1.725 m",
          zero and kept and level is not None and math.isclose(level, -1.7255, abs_tol=1e-3),
          f"ITerrain (landscape +0x134) slot 11 reads +0x7ba8, the landscape's +0x7cdc, set "
          f"from the first face with surface bit 0x02 (runtime 0x20000) or -1: {zero}; "
          f"iron3d.dll keeps it at the world's +0xad8: {kept}; Tut_1's water plane {level}")

    # The flyer's cross: MBehaviour variable 0x207 is the chassis profile, ChassisType first.
    behavior = game / "Behavior.dll"
    first = None
    if behavior.exists():
        b_image = behavior.read_bytes()
        bat = _image_at(b_image)
        switch = re.search(rb"\x8d\x88\xfe\xfd\xff\xff\x83\xf9\x0a\x0f\x87.{4}\xff\x24\x8d(.{4})",
                           b_image, re.S)
        binder = re.search(rb"\x81\xc1\xc0\x07\x00\x00\xe8(.{4})", b_image, re.S)
        profile = False
        if switch:
            table = struct.unpack("<I", switch.group(1))[0]
            profile = bat(struct.unpack("<I", bat(table + 4 * 5, 4))[0], 13) == bytes.fromhex(
                "8b8424240100005e05c0070000")
        if binder and profile:
            sections, _ = resources._sections(b_image)
            lfanew = struct.unpack_from("<I", b_image, 0x3C)[0]
            base = struct.unpack_from("<I", b_image, lfanew + 24 + 28)[0]
            call = next(v + binder.start() + 6 - raw for v, size, raw in sections
                        if raw <= binder.start() + 6 < raw + size) + base
            target = (call + 5 + struct.unpack("<i", binder.group(1))[0]) & 0xFFFFFFFF
            head = bat(target, 40)
            name_at = head.find(b"\x57\x68")
            if name_at >= 0:
                first = bat(struct.unpack_from("<I", head, name_at + 2)[0], 16).split(b"\0")[0]
    check("Behavior.dll: the radar's cross is a flyer, ChassisType 1 of the chassis profile",
          first == b"ChassisType" and profiles.CHASSIS_TYPE.get(1) == "flying",
          f"variable 0x207 answers the profile at +0x7c0, whose binder links {first!r} first; "
          f"type 1 is {profiles.CHASSIS_TYPE.get(1)}")

    # The indicators: lamps and icons, their slots' x, and what lights each.
    icuts, _ = _hud_sprites(at, 0x1003ED60, 0x3B4)
    iplaced = [art(*r) for r in INDICATOR_CUTS]
    icon = at(0x1003F270, 0xD0)
    table_x = re.search(rb"\x69\xc9\x8c\x00\x00\x00((?:\xc7\x44\x24..{4}){8})", icon, re.S)
    xs = [struct.unpack_from("<i", table_x.group(1), 4 + 8 * i)[0] for i in range(8)] \
        if table_x else []
    look = all(p in icon for p in (
        bytes.fromhex("68e40100008d46125068cb01000056"),    # lamp (x, 459) - (x + 18, 484)
        bytes.fromhex("68de0100008d56105268cf0100004656"),  # icon (x + 1, 463) - (x + 16, 478)
        bytes.fromhex("8d4c290c"), bytes.fromhex("8d8c2bb0010000"),  # lamp by state, icon by slot
        bytes.fromhex("bf808080ff"), bytes.fromhex("81e70100ffff4f")))  # grey 0; red 2; white 1
    draw = at(0x1003F150, 0x118)
    states = all(p in draw for p in (
        bytes.fromhex("85c0740983f801740433c0eb05b801000000506a03"),  # CState mode 0 or 1
        bytes.fromhex("8a4b31f6d91bc983e102516a04"),                  # +0x31 -> state 2
        bytes.fromhex("8b8f9c00000033d285c90f94c28bce526a05"),        # auto-driver 0, 1, 2
        bytes.fromhex("83fa010f94c08bce506a06"), bytes.fromhex("83fb020f94c1516a07")))
    readers = bytes.fromhex(
        "568b71608b066a0f6a0056ff502483f8ff74108b0e5056ff511483f820") in at(0x10076E10, 40) \
        and bytes.fromhex("6a0a6a0056ff502483f8ff74128b0e5056ff51143d00100000") \
        in at(0x10076E40, 32) \
        and at(0x10035C20, 16) == bytes.fromhex("85c9740d8b0151ff5050c1e80583e001")
    camera = False
    if ctl.exists():
        handler = _image_at(ctl.read_bytes())(0x10023A00, 0xA4)
        camera = re.search(rb"\x81\xfe\x00\x10\x00\x00\x5f\x74.\x81\xfe\x00\x20\x00\x00\x74."
                           rb"\x81\xfe\x00\x40\x00\x00\x75.\x8b\x4c\x24\x08\xa8\x20", handler,
                           re.S) is not None and b"\x0c\x20" in handler and b"\x24\xdf" in handler
    rows = {(a.key, a.target, a.state) for a in controls.table(game / "hero.tbl")}
    keyed = {("SCAN_G", "CICLS_REPAIRSYS", "CIS_SWITCH_INV"),
             ("SCAN_H", "CICLS_DETECTSHIELD", "CIS_CHAMELEON_INV"),
             ("SCAN_N", "CICLS_CAMERA", "CIS_INFRARED_INV")} <= rows
    driver = {b.command: b.chord for b in controls.bindings(game / "ui_other.man")}
    strings = resources.strings(iron)
    risk = b"\xbb\x01\x00\x00\x00\xba\x43\x18\x00\x00" in at(0x10063500, 0x180) \
        and b"\x88\x5e\x31" in at(0x10063500, 0x180) \
        and bytes.fromhex("d81d145c0e10dfe0f6c4417513c6463100") in at(0x10062950, 0xE0) \
        and struct.unpack("<f", at(0x100E5C14, 4))[0] == 3.0
    check("iron3d.dll: the indicators -- repair, infrared, camouflage, mode, risk, auto-driver",
          [c[:4] for c in icuts] == [r[1:] for r in INDICATOR_CUTS]
          and xs == list(INDICATOR_X) and look and states and readers and camera and keyed
          and driver.get("CMD_JAMES_AUTO_DRIVER") == "SCAN_Y" and risk
          and strings.get(6211) == "Risk area! Landing impossible."
          and all(ok for _, ok in iplaced)
          and [p for p, _ in iplaced] == ["ui_tex6.tex"] * 3 + ["ui_menu1.tex"] * 4
          + ["ui_menu3.tex"] + ["ui_menu1.tex"] * 3,
          f"0x1003ed60 cuts {icuts}; slots at x {xs}; lamp and icon rects, grey/white/red "
          f"{look}; 0x1003f150's states {states}; class 15 state 0x20, class 10 state 0x1000, "
          f"view flag 0x20: {readers}; the camera class sets, clears and flips 0x20: {camera}; "
          f"hero.tbl G/H/N {keyed}; Y {driver.get('CMD_JAMES_AUTO_DRIVER')}; 6211 "
          f"{strings.get(6211)!r} lights slot 4 red for 3.0 s {risk}; art inked "
          f"{sum(ok for _, ok in iplaced)}/{len(iplaced)}")

    # The reticle.
    rcuts, _ = _hud_sprites(at, 0x10042D10, 0x190)
    _, rdraws = _hud_sprites(at, 0x10042ED0, 0x223)
    rplaced = [art(*r) for r in RETICLE_CUTS]
    dot = b"\x68\x00\xff\x00\xff" in at(0x10042ED0, 0x223) \
        and any(math.isclose(f, 24.0) for f in _floats_read(at, 0x10042ED0, 0x223))
    check("iron3d.dll: the reticle's circle, arcs and ticks about (320, 240), and its dot",
          [c[:4] for c in rcuts] == [r[1:] for r in RETICLE_CUTS]
          and [d[:4] for d in rdraws] == list(RETICLE_DRAWS)
          and all(d[4] == 0xFF37FF37 for d in rdraws) and dot
          and all(ok and p == "ui_tex9.tex" for p, ok in rplaced),
          f"0x10042d10 cuts {rcuts}; 0x10042ed0 draws {[d[:4] for d in rdraws]} in "
          f"{sorted({hex(d[4]) for d in rdraws})}; a green dot 24 x the camera's offsets: {dot}")


#: The objectives screen's strings (``iron3d.dll:0x1006a210``, ``0x1006af90``).
OBJECTIVE_STRINGS = {
    3062: "Primary objectives", 3061: "Additional objectives", 1017: "Press %s to close",
    1015: "in progress", 1014: "complete", 1027: "failed", 6247: "Multiplayer statistics",
}
#: An objective's state word and its colour (``0x1006af90``): the string pushed, then the
#: colour written.
OBJECTIVE_STATES = ((1015, 0xFF787878), (1014, 0xFFEBEBEB), (1027, 0xFFFF6464))
#: The satellite map's rects as ``0x10072f90`` builds them: the cockpit's, then the
#: commander's outer and inner.
MAP_RECTS = ((374, 0, 640, 266), (374, 43, 640, 350), (374, 63, 640, 329))
#: The map's building icons: the ``varset.var`` type each ``0x1009f4c0`` gives an index,
#: and that index's 24 x 24 cell of the ``icons`` page (``0x10064f10``).
MAP_ICONS = {
    "BUILDING_GENERATOR": (0, 24), "BUILDING_MINE": (24, 24), "BUILDING_STORAGE": (48, 0),
    "BUILDING_PLANT": (72, 24), "BUILDING_BUNKER_SMALL": (96, 24),
    "BUILDING_BUNKER_MEDIUM": (96, 24), "BUILDING_BUNKER_LARGE": (96, 24),
    "BUILDING_HANGAR": (120, 24), "BUILDING_INSTITUTE": (48, 24),
    "BUILDING_TOWER_MEDIUM": (168, 24), "BUILDING_TOWER_LARGE": (168, 24),
    "BUILDING_MAINTELEPORT": (216, 24), "BUILDING_BRIDGE": None, "BUILDING_RUINE": None,
}


def _compare_tree(code: bytes, value: int) -> int | None:
    """What a compiled ``switch`` of ``cmp ecx, imm32`` / ``je`` / ``jg`` / ``jne`` returns in
    ``eax`` for ``ecx = value``: ``mov eax, imm32``, ``xor eax, eax`` and ``or eax, -1``
    before a ``ret``.  None on any other instruction."""
    pc, signed = 0, value - (1 << 32) if value & 0x80000000 else value
    last, eax = 0, None
    for _ in range(64):
        op = code[pc]
        if code[pc:pc + 2] == b"\x81\xf9":
            imm = struct.unpack_from("<i", code, pc + 2)[0]
            last = (signed > imm) - (signed < imm)
            pc += 6
        elif op in (0x74, 0x75, 0x7F):
            taken = {0x74: last == 0, 0x75: last != 0, 0x7F: last > 0}[op]
            pc += 2 + (struct.unpack_from("<b", code, pc + 1)[0] if taken else 0)
        elif op == 0xB8:
            eax = struct.unpack_from("<I", code, pc + 1)[0]
            pc += 5
        elif code[pc:pc + 2] in (b"\x33\xc0", b"\x31\xc0"):
            eax, pc = 0, pc + 2
        elif code[pc:pc + 3] == b"\x83\xc8\xff":
            eax, pc = 0xFFFFFFFF, pc + 3
        elif op == 0xC3:
            return eax
        else:
            return None
    return None


def check_hud_screens(check, game: Path) -> None:
    """The objectives screen (F12) and the satellite map (M) over the cockpit HUD."""
    path = game / "iron3d.dll"
    if not path.exists():
        return
    iron = path.read_bytes()
    at = _image_at(iron)
    strings = resources.strings(iron)
    art = _hud_art(game)
    keys = {b.command: b.chord for b in controls.bindings(game / "ui_other.man")}

    def u32(va: int) -> int:
        return struct.unpack("<I", at(va, 4))[0]

    def f32(va: int) -> float:
        return struct.unpack("<f", at(va, 4))[0]

    # The objectives screen's words: built once by 0x1006a210, the state words by 0x1006af90.
    ctor = at(0x1006A210, 0x330)
    built = [k for k in (1017, 3062, 3061, 6247) if b"\xba" + struct.pack("<I", k) in ctor]
    states = at(0x1006AF90, 0x198)
    coloured = []
    for sid, colour in OBJECTIVE_STATES:
        i = states.find(b"\x68" + struct.pack("<I", sid))
        j = states.find(b"\xc7\x00" + struct.pack("<I", colour), i)
        coloured.append(i >= 0 and 0 < j - i < 0x40)
    fmt = b"\x68\x84\x4d\x10\x10" in states and at(0x10104D84, 8) == b"%s : %s\0"
    words = {k: strings.get(k) for k in OBJECTIVE_STRINGS}
    check("iron3d.dll: the objectives screen's header, footer and state words",
          built == [1017, 3062, 3061, 6247] and all(coloured) and fmt
          and b"\x68\xdb\x02\x00\x00" in ctor and words == OBJECTIVE_STRINGS
          and controls.CMD_GAME.get("CMD_JAMES_MISSION_OBJ") == 731
          and keys.get("CMD_JAMES_MISSION_OBJ") == "SCAN_F12",
          f"0x1006a210 loads {[words.get(k) for k in built]} and the key of 731 "
          f"({keys.get('CMD_JAMES_MISSION_OBJ')}); 0x1006af90 formats '%s : %s' with "
          + ", ".join(f"{words.get(s)!r} {c:#x}" for (s, c), ok in zip(OBJECTIVE_STATES, coloured,
                                                                        strict=True) if ok))

    # The layout: the dim, the header at 80, lines from 110 by 20, the bonus list at 260 and
    # 290, the footer at 450, all centred on 320 in MENU_FONT (the game's +0x14).
    draw = at(0x1006A9D0, 0x240)
    lines = at(0x1006AC10, 0x380)
    dim = b"\x68\xe0\x01\x00\x00\x68\x80\x02\x00\x00\x6a\x00\x6a\x00" in draw \
        and b"\x83\xe1\x14\x83\xc1\x3c\x69\xc9\xff\x00\x00\x00" in draw
    # The footer's `push -1` and `push 0x1c2` have an `fmul` between them.
    places = all(p in draw for p in (
        b"\x68\x00\xff\x00\xff\x6a\x50", b"\xba\x40\x01\x00\x00", b"\x8b\x70\x14")) \
        and re.search(rb"\x6a\xff.{0,8}\x68\xc2\x01\x00\x00", draw, re.S) is not None \
        and all(p in lines for p in (
            b"\xc7\x44\x24\x18\x6e\x00\x00\x00", b"\x68\x04\x01\x00\x00",
            b"\xc7\x44\x24\x18\x22\x01\x00\x00", b"\x68\x00\xff\x00\xff", b"\x8b\x70\x14")) \
        and lines.count(b"\x83\xc7\x14") == 2
    check("iron3d.dll: the objectives screen over a 60% dim, header y 80, lines 110 + 20i",
          dim and places,
          f"0x1006a9d0 fills (0, 0)-(640, 480) at (60 + 20 in CState modes 3-5)% black {dim}; "
          f"green header at y 80, white footer at 450, lines from 110 by 20, bonus header "
          f"260 and lines 290, centred on 320 in the game's +0x14 font: {places}")

    # When it opens and closes: armed at mission start, closed 7 s after its first draw; F12
    # toggles it and disarms the timer on closing; Esc closes it; while up, the screens' draw
    # skips the HUD, the map and the message box.
    start = at(0x1005E117, 0x4A)
    armed = all(p in start for p in (
        b"\x38\x9e\xe5\x00\x00\x00", b"\x80\xbd\x54\x01\x00\x00\x01",
        b"\xc6\x80\x90\x05\x00\x00\x01", b"\xc6\x80\x91\x05\x00\x00\x01"))
    timer = b"\xd8\x1d\xa8\x63\x0e\x10" in draw and f32(0x100E63A8) == 7.0 \
        and b"\xc6\x83\x91\x05\x00\x00\x00" in draw
    table = 0x100726DC
    f12 = u32(table + 4)
    toggle = at(f12, 0x80)
    toggles = f12 == 0x1007210E and b"\x88\x85\x92\x05\x00\x00" in toggle \
        and b"\x88\x85\x90\x05\x00\x00" in toggle and b"\x83\x3a\x07" in toggle
    esc = at(0x10070E85, 0x40)
    escapes = b"\x8a\x81\x92\x05\x00\x00" in esc and b"\x88\x87\x92\x05\x00\x00" in esc
    order = [t for _, t in _calls(at, 0x1008D37F, 0x1008D5AB - 0x1008D37F)
             if t in (0x10043B20, 0x10073750, 0x1007F4F0, 0x1006A9D0)]
    hides = order[:4] == [0x10043B20, 0x10073750, 0x1007F4F0, 0x1006A9D0] \
        and at(0x1008D3AC, 6) == b"\x8a\x81\x92\x05\x00\x00" \
        and at(0x1008D3F6, 6) == b"\x0f\x85\x9f\x01\x00\x00"
    check("iron3d.dll: it opens with a new mission for 7 s; F12 and Esc; it hides the HUD",
          armed and timer and toggles and escapes and hides,
          f"0x1005e117 sets +0x592, +0x590 and +0x591 unless +0xe5, if +0x154: {armed}; "
          f"0x1006a9d0 stamps its first draw and closes past {f32(0x100E63A8)} s: {timer}; "
          f"731 at {f12:#x} toggles +0x592, clearing +0x590 on closing, not in CState mode 7: "
          f"{toggles}; Esc closes it {escapes}; 0x1008d37f draws {[hex(t) for t in order]} "
          f"and jumps to the last while +0x592 is set: {hides}")

    # The satellite map: its rects, its alpha, its art.
    mctor = at(0x10072F90, 0x3B0)
    rects = []
    for x0, y0, x1, y1 in MAP_RECTS:
        # push y1, x1, y0 (ebx when 0), x0 -- a `lea` may sit before the last.
        y0push = b"\x53" if y0 == 0 else b"\x6a" + bytes([y0])
        rects.append(re.search(re.escape(b"\x68" + struct.pack("<I", y1) + b"\x68"
                                         + struct.pack("<I", x1) + y0push)
                               + rb".{0,8}" + re.escape(b"\x68" + struct.pack("<I", x0)),
                               mctor, re.S) is not None)
    alpha = b"\xc7\x85\x90\x00\x00\x00\x80\x00\x00\x00" in mctor \
        and b"\x83\xfe\x1e" in mctor and b"\x81\xfe\xff\x00\x00\x00" in mctor \
        and b"\x68\xec\x4e\x10\x10" in mctor and at(0x10104EEC, 10) == b"MAP_ALPHA\0"
    ini = settings.sections(game / "Iron_3D.ini") if (game / "Iron_3D.ini").exists() else {}
    shipped = ini.get("CS", {}).get("MAP_ALPHA")
    loader = at(0x10073550, 0x200)
    loads = b"\x53\x68\x00\x00\x80\x43\x68\x00\x00\x80\x43\x53\x53\x68\x00\x00\x80\x43\x57" \
        in loader and all(b"\x68" + struct.pack("<I", va) in loader
                          for va in (0x10104C98, 0x10104F00, 0x10104EF8)) \
        and [at(va, 17).split(b"\0")[0] for va in (0x10104C98, 0x10104F00, 0x10104EF8)] \
        == [b"exit_icon", b"map_compass_icon", b"minimap"] \
        and b"\xba\xd2\x13\x00\x00" in loader and strings.get(5074) == "Satellite map"
    hq = {k.lower(): {kk.lower(): vv for kk, vv in v.items()}
          for k, v in resources.load_cfg(game / "ui" / "hq.cfg").items()}
    pieces = {}
    for name in ("map_compass_icon", "exit_icon"):
        p = hq.get(name, {})
        rect = tuple(int(p.get(k, -1)) for k in ("offset_x", "offset_y", "width", "height"))
        pieces[name] = (p.get("texture"), rect, art(p.get("texture", ""), *rect))
    mini = NResArchive.open(game / "ui" / "minimap.lib")
    sizes = Counter()
    for e in mini:
        tex = texm.decode(mini.read(e))
        sizes[(tex.width, tex.height)] += 1
    check("iron3d.dll: the satellite map at (374, 0)-(640, 266), a 256 x 256 minimap, alpha 128",
          all(rects) and alpha and shipped == "128" and loads
          and pieces["map_compass_icon"][:2] == ("page6", (132, 106, 21, 42))
          and pieces["exit_icon"][:2] == ("ui_menu", (241, 70, 13, 13))
          and all(ok for *_, (_, ok) in pieces.values()) and set(sizes) == {(256, 256)},
          f"0x10072f90 builds {[r for r, ok in zip(MAP_RECTS, rects, strict=True) if ok]}; "
          f"MAP_ALPHA 128 by default, held to 30-255: {alpha} (Iron_3D.ini {shipped}); "
          f"0x10073550 cuts the minimap (0, 0, 256, 256) and loads exit_icon, map_compass_icon, "
          f"5074: {loads}; hq.cfg {pieces}; ui/minimap.lib {dict(sizes)}")

    # Opening it, and its alpha: M toggles; ] and [ step 12, only while open, and show a label.
    upper = 0x100726F8
    m_case, inc_case, dec_case = (u32(upper + 4 * i) for i in (1, 13, 14))
    toggled = {t for _, t in _calls(at, m_case, 0x21)} == {0x10074100, 0x100740F0}
    stepped = [t for _, t in _calls(at, inc_case, 0x14)] == [0x10074550] \
        and [t for _, t in _calls(at, dec_case, 0x14)] == [0x100745B0]
    inc, dec = at(0x10074550, 0x50), at(0x100745B0, 0x50)
    steps = all(b"\x83\xf8\x1e" in f and b"\x3d\xff\x00\x00\x00" in f
                and b"\xc6\x86\x62\x02\x00\x00\x01" in f for f in (inc, dec)) \
        and b"\x83\xc2\x0c" in inc and b"\x83\xc2\xf4" in dec
    label = at(0x10074640, 0x25D)
    labelled = all(p in label for p in (
        b"\x68\x00\x73\x00\xff", b"\x68\x37\xff\x37\xff", b"\x8b\x70\x10", b"\x6b\xd2\x64",
        b"\xbf\x05\x00\x00\x00")) and at(0x10104D94, 2) == b"%\0" \
        and 1.0 in _floats_read(at, 0x10074640, 0x25D)
    codes = {c: controls.CMD_GAME.get(c) for c in
             ("CMD_JAMES_SATELLITE_MAP", "CMD_INC_MAP_ALPHA", "CMD_DEC_MAP_ALPHA")}
    check("iron3d.dll: M opens the map; ] and [ step its alpha by 12 and label it for 1 s",
          toggled and stepped and steps and labelled
          and codes == {"CMD_JAMES_SATELLITE_MAP": 739, "CMD_INC_MAP_ALPHA": 751,
                        "CMD_DEC_MAP_ALPHA": 752}
          and [keys.get(c) for c in codes] == ["SCAN_M", "SCAN_RBRACKET", "SCAN_LBRACKET"],
          f"739 at {m_case:#x} opens or closes {toggled}; 751 at {inc_case:#x} and 752 at "
          f"{dec_case:#x} call +12 and -12 {stepped}, held to 30-255 {steps}; the label, "
          f"alpha x 100 / 255 down to a 5, '%' in GAME_FONT #37ff37 on 0xff007300, 1 s: "
          f"{labelled}; keys {[keys.get(c) for c in codes]}")

    # The panel: by view state, the frame, the minimap tinted #37ff37 at the alpha, the compass;
    # a world point at 256 / side; the frame's pieces in order.
    variant = at(0x10073750, 0x70)
    by_state = all(b"\x83\xf8" + bytes([s]) in variant for s in (1, 3, 6, 4)) \
        and [t for _, t in _calls(at, 0x10073750, 0x70)] == [0x10073830, 0x100737C0, 0x10074220]
    panel_calls = [t for _, t in _calls(at, 0x100748A0, 0x117)]
    panel = at(0x100748A0, 0x117)
    drawn = panel_calls == [0x10025560, 0x10025590, 0x1009ABF0, 0x1008F970, 0x10074640,
                            0x1008F970] \
        and b"\xc1\xe5\x18\x81\xcd\x37\xff\x37\x00" in panel \
        and b"\xba\x05\x00\x00\x00" in panel and b"\xbb\xfb\xff\xff\xff" in panel
    world = at(0x100741A0, 0x80)
    placed = _floats_read(at, 0x100741A0, 0x80).count(256.0) == 2 \
        and b"\x8d\x54\x11\x05" in world and b"\x83\xea\x05" in world
    frame = at(0x1009ABF0, 0x3B0)
    kinds = [frame.find(b"\x6a" + bytes([v]) + b"\x6a" + bytes([k]))
             for k, v in ((9, 0), (9, 1), (9, 2), (9, 3), (10, 1), (10, 0))]
    framed = all(i >= 0 for i in kinds) and kinds == sorted(kinds) \
        and sum(t == 0x1008F970 for _, t in _calls(at, 0x1009ABF0, 0x3B0)) == 8
    cc = resources.load_cfg(game / "ui" / "compaund.cfg")
    shapes = [(int(cc[n]["width"]), int(cc[n]["height"]), int(cc[n]["rotate"]))
              for n in ("ccres_frame_corner_1", "ccres_frame_corner_2", "ccres_frame_corner_3",
                        "ccres_frame_corner_4", "ccres_frame_edge_h", "ccres_frame_edge_v")]
    check("iron3d.dll: the cockpit's map panel -- frame, minimap from 5 in, compass, places",
          by_state and drawn and placed and framed
          and shapes == [(8, 8, 0), (8, 8, 90), (8, 8, 180), (8, 8, 270), (47, 5, 0), (47, 5, 90)],
          f"0x10073750 takes the panel in view states 1, 3, 4 and 6: {by_state}; 0x100748a0 "
          f"frames the rect, draws the minimap from 5 in at (alpha << 24) | 0x37ff37 and the "
          f"compass in the corner: {drawn}; 0x100741a0 puts a point at 256 / side from the "
          f"inner corner: {placed}; 0x1009abf0 loads corners 1-4, edge_h, edge_v and draws 8 "
          f"pieces: {framed}; compaund.cfg's frame pieces {shapes}")

    # The marks: units, buildings by type, the +0x700 list, the commander's camera.
    units_fn = at(0x10077690, 0x6E5)
    unit_floats = _floats_read(at, 0x10077690, 0x6E5)
    marked = all(p in units_fn for p in (
        b"\x68\x64\xc8\x64\xff", b"\x68\x64\xc8\xc8\xff", b"\x81\x7d\x2c\x00\x00\x02\x01",
        b"\x68\x07\x02\x00\x00", b"\x25\xff\x00\xff\x00\x05\x00\xff\x00\xff")) \
        and {4.0, 3.0} <= set(unit_floats) and any(math.isclose(f, 1.8, rel_tol=1e-6)
                                                   for f in unit_floats)
    build_fn = at(0x100347F0, 0x110)
    icon_draw = all(p in build_fn for p in (
        b"\x8d\x50\x0a", b"\x83\xc1\xf6", b"\x69\xc9\x8c\x00\x00\x00\x81\xc1\x60\xb6\x10\x10")) \
        and {t for _, t in _calls(at, 0x100347F0, 0x110)} >= {0x1009F4C0, 0x100741A0, 0x1008F970}
    cells = {}
    for m in re.finditer(rb"((?:\x53|\x68.{4}){5})\x68\x00\x00\x80\x43\x56\xb9(.{4})\xe8",
                         at(0x10064F80, 0x280), re.S):
        values = _push_run(m.group(1), True)
        slot = struct.unpack("<I", m.group(2))[0] - 0x1010B660
        if values and slot >= 0 and slot % 0x8C == 0:
            _, h, w, y, x = values
            cells[slot // 0x8C] = (round(x), round(y), round(w), round(h))
    types = {v.name: int(v.default, 0) for v in behaviour.variables(game)
             if v.name in MAP_ICONS}
    lookup = at(0x1009F4C0, 0xA0)
    icons = {}
    for name, value in types.items():
        index = _compare_tree(lookup, value)
        icons[name] = None if index in (None, 0xFFFFFFFF) else cells.get(index, ("?",))[:2]
    inked = [art("icons", x, y, 24, 24)[1] for x, y in {c for c in MAP_ICONS.values() if c}]
    extras = b"\x68\x00\x64\x64\xff" in at(0x10081A70, 0xC0) \
        and at(0x10074220, 0x210).count(b"\x68\x00\xff\xff\xff") == 4 \
        and _floats_read(at, 0x10074220, 0x210).count(7.0) == 2
    check("iron3d.dll: the map's marks -- units in screen pixels, buildings by type from icons",
          marked and icon_draw and icons == MAP_ICONS and all(inked) and extras,
          f"0x10077690: a flyer's cross 4, a square 3, a heading 1.8, the route in 0xff64c864 "
          f"and 0xffc8c864, the hero green: {marked}; 0x100347f0 draws a 20 x 20 icon: "
          f"{icon_draw}; icons by type {icons}, inked {sum(inked)}/{len(inked)}; the +0x700 "
          f"list's 0xff646400 squares and the yellow camera with its 7-unit tick: {extras}")


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


def check_walker(check, game: Path) -> None:
    """The movement points a walker hands the wizard, as the chassis profiles decide them."""
    held = profiles.load(game)
    # Behavior.dll:0x1003daab -- a point keeps its side and vertical velocity
    # only when the profile's CanFly is set and WalkChassis is not
    free = {var: bool(v["CanFly"].value) and not v["WalkChassis"].value
            for var, v in held.items() if var.startswith("chas_") and "CanFly" in v}
    kinds = {var: held[var]["ChassisType"].value for var in free}
    check("behpsp.res: only chas_fly keeps a point's side and vertical speed",
          free and [v for v, f in free.items() if f] == ["chas_fly.var"]
          and kinds["chas_fly.var"] == 1,
          f"CanFly and not WalkChassis on {sorted(v for v, f in free.items() if f)} of "
          f"{len(free)} chassis profiles; the rest get flags 0x3030, x and z held to 0")

    shop = units.Workshop(game)
    battle = game / "UNITS" / "UNITS" / "BATTLE"
    seen = {}
    for name in ("helic.dat", "tut1_mf1.dat", "tut1_e1.dat"):
        unit = objects.load_unit(battle / name)
        member = unit.components[0].ref.member.lower()
        record = shop.library.get(member)
        parsed = shop.armoury.controller(member)
        top = parsed.triples[control.TRIPLE_TOP_SPEED]
        turn = parsed.triples[control.TRIPLE_TURN]
        seen[name] = (member, record.profile, parsed.mode,
                      tuple(round(x, 1) for x in top), round(turn[2], 2))
    check("Mission 01: helic and tut1_mf1 fly, tut1_e1 walks",
          seen == {"helic.dat": ("r_t_02", "chas_fly.var", 0, (20.0, 33.3, 35.0), 4.0),
                   "tut1_mf1.dat": ("r_m_02", "chas_fly.var", 0, (4.0, 34.7, 20.0), 3.8),
                   "tut1_e1.dat": ("r_t_01", "chas_wlk.var", 0, (10.0, 26.4, 1.0), 3.5)},
          f"{seen}: root chassis, profile, controller mode, top speed x/y/z m/s "
          f"(triple 3) and yaw turn rate (triple 4 z)")


def run(game: Path) -> int:
    """Run every check against ``game``.  Returns a process exit code."""
    results: list[tuple[str, bool, str]] = []

    def check(name: str, ok: bool, evidence: str) -> None:
        results.append((name, bool(ok), evidence))
        print(f"{'PASS' if ok else 'FAIL'}  {name:<46} {evidence}")

    print(f"verifying against {game}\n")
    checks = (
        check_nres, check_texm, check_terrain, check_uv,
        check_water, check_water_reflection, check_layers, check_materials, check_material_draw,
        check_sky,
        check_render_state, check_minimap_agreement, check_arealmap,
        check_grid, check_missions, check_scale, check_objects, check_poses, check_lod,
        check_damage, check_node_stages,
        check_effects, check_effect_timing, check_sounds, check_actions, check_footprints,
        check_rsli,
        check_control, check_efficiency,
        check_motion, check_playback, check_ground, check_sensors, check_hit_test,
        check_collision, check_lake_and_buoys,
        check_combat, check_ownership,
        check_capture, check_repair, check_chassis, check_weapons, check_firing,
        check_moving_parts,
        check_targeting, check_target_marks, check_ai_fight, check_turrets, check_packages,
        check_target_panel,
        check_wingman,
        check_builder,
        check_units, check_loading, check_search, check_construction,
        check_controls, check_player_input, check_turret_channels,
        check_behaviour, check_research, check_descriptions, check_saves,
        check_vocabulary, check_resources, check_briefing, check_briefing_screen,
        check_progression, check_outcome,
        check_hud_top,
        check_hud_radar,
        check_hud_screens,
        check_settings,
        check_research_streams, check_atmosphere_events,
        check_varset_types, check_profiles, check_walker,
    )
    for fn in checks:
        fn(check, game)
    failed = [n for n, ok, _ in results if not ok]
    print(f"\n{len(results) - len(failed)}/{len(results)} checks passed")
    for n in failed:
        print(f"  failed: {n}")
    return 1 if failed else 0
