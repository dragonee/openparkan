"""Command line entry point: ``python -m openparkan <command>``."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from . import gamedir, landmesh, texm, verify, viewer
from .nres import NResArchive, is_nres
from .png import write_png


def _archive_path(game: Path, name: str) -> Path:
    p = Path(name)
    return p if p.exists() else game / name


def cmd_info(args, game: Path) -> int:
    maps = gamedir.maps(game)
    missions = gamedir.missions(game)
    print(f"install : {game}")
    print(f"maps    : {len(maps)}")
    print(f"missions: {len(missions)}")
    archives = sorted(
        p for p in game.rglob("*")
        if p.is_file() and p.suffix.lower() in {".lib", ".rlb", ".dlb", ".res", ".trf"}
    )
    good = sum(1 for p in archives if is_nres(p))
    print(f"archives: {good} NRes, {len(archives) - good} other")
    return 0


def cmd_ls(args, game: Path) -> int:
    archive = NResArchive.open(_archive_path(game, args.archive))
    print(f"# {archive.source} -- {len(archive)} entries, version 0x{archive.version:x}")
    for e in archive:
        if args.type and e.tag != args.type:
            continue
        print(f"{e.tag:<8} {e.name:<34} {e.size:10d}  @{e.offset}")
    return 0


def cmd_extract(args, game: Path) -> int:
    archive = NResArchive.open(_archive_path(game, args.archive))
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    n = 0
    for e in archive:
        if args.type and e.tag != args.type:
            continue
        name = e.name or f"unnamed_{e.index}"
        # Members of the per-map files all share a name; disambiguate by type.
        target = out / ("{}.{}".format(name, e.tag.lstrip("#")) if args.by_type else name)
        target.write_bytes(archive.read(e))
        n += 1
    print(f"extracted {n} members to {out}")
    return 0


def cmd_textures(args, game: Path) -> int:
    archive = NResArchive.open(_archive_path(game, args.archive))
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    ok = failed = 0
    for e in archive:
        try:
            tex = texm.decode(archive.read(e))
        except texm.UnsupportedTexture as exc:
            failed += 1
            if args.verbose:
                print(f"  skip {e.name:<24} {exc}", file=sys.stderr)
            continue
        stem = e.name.rsplit(".", 1)[0] or f"unnamed_{e.index}"
        if args.alpha:
            write_png(out / (stem + ".png"), tex.width, tex.height, tex.rgba, alpha=True)
        else:
            write_png(out / (stem + ".png"), tex.width, tex.height, texm.to_rgb(tex))
        ok += 1
    print(f"wrote {ok} PNGs to {out} ({failed} skipped)")
    return 0


def cmd_maps(args, game: Path) -> int:
    for d in gamedir.maps(game):
        mesh = landmesh.load(d / "Land.msh")
        (lo, _, minz), (hi, _, maxz) = mesh.bounds()
        width = mesh.bounds()[1][0] - mesh.bounds()[0][0]
        depth = mesh.bounds()[1][1] - mesh.bounds()[0][1]
        layers = ",".join(n for n in mesh.layer1_names if n)
        print(
            f"{d.name:<16} {mesh.vertex_count:5d} verts {mesh.face_count:5d} tris  "
            f"{width:6.0f} x {depth:<6.0f}  z {minz:7.1f}..{maxz:<7.1f}  {layers}"
        )
    return 0


def cmd_heightmap(args, game: Path) -> int:
    """Rasterise a map top-down: the quickest way to eyeball a terrain parse."""
    mesh = landmesh.load(game / "DATA" / "MAPS" / args.map / "Land.msh")
    n = args.size
    (minx, miny, minz), (maxx, maxy, maxz) = mesh.bounds()
    depth = [None] * (n * n)
    shade = [0.0] * (n * n)
    for tri in mesh.faces:
        p = [mesh.positions[i] for i in tri]
        sx = [(v[0] - minx) / (maxx - minx) * (n - 1) for v in p]
        sy = [(1 - (v[1] - miny) / (maxy - miny)) * (n - 1) for v in p]
        den = (sy[1] - sy[2]) * (sx[0] - sx[2]) + (sx[2] - sx[1]) * (sy[0] - sy[2])
        if abs(den) < 1e-9:
            continue
        nz = [mesh.normals[i] for i in tri]
        for py in range(max(0, int(min(sy))), min(n, int(max(sy)) + 2)):
            for px in range(max(0, int(min(sx))), min(n, int(max(sx)) + 2)):
                l1 = ((sy[1] - sy[2]) * (px - sx[2]) + (sx[2] - sx[1]) * (py - sy[2])) / den
                l2 = ((sy[2] - sy[0]) * (px - sx[2]) + (sx[0] - sx[2]) * (py - sy[2])) / den
                l3 = 1 - l1 - l2
                if l1 < -0.002 or l2 < -0.002 or l3 < -0.002:
                    continue
                i = py * n + px
                z = l1 * p[0][2] + l2 * p[1][2] + l3 * p[2][2]
                if depth[i] is None or z > depth[i]:
                    depth[i] = z
                    lx = l1 * nz[0][0] + l2 * nz[1][0] + l3 * nz[2][0]
                    ly = l1 * nz[0][1] + l2 * nz[1][1] + l3 * nz[2][1]
                    lz = l1 * nz[0][2] + l2 * nz[1][2] + l3 * nz[2][2]
                    shade[i] = max(0.12, -0.45 * lx + 0.42 * ly + 0.79 * lz)
    span = (maxz - minz) or 1.0
    ramp = (
        (0.10, (48, 78, 132)),
        (0.42, (66, 112, 58)),
        (0.72, (116, 98, 62)),
        (1.01, (196, 194, 188)),
    )
    out = bytearray(n * n * 3)
    for i, z in enumerate(depth):
        if z is None:
            continue
        t = (z - minz) / span
        base = next(c for edge, c in ramp if t < edge)
        k = shade[i] * 1.35
        out[i * 3 : i * 3 + 3] = bytes(min(255, int(c * k)) for c in base)
    write_png(args.out, n, n, bytes(out))
    print(f"wrote {args.out} ({n}x{n})")
    return 0


def cmd_verify(args, game: Path) -> int:
    return verify.run(game)


def cmd_viewer(args, game: Path) -> int:
    names = args.maps or [d.name for d in gamedir.maps(game)]
    resolver = viewer.TextureResolver(game, max_size=args.texture_size)
    payloads = []
    for name in names:
        path = game / "DATA" / "MAPS" / name / "Land.msh"
        if not path.exists():
            print(f"no such map: {name}", file=sys.stderr)
            return 2
        mesh = landmesh.load(path)
        payloads.append(viewer.build_map_payload(mesh, resolver, name))
        print(f"  packed {name:<16} {mesh.vertex_count:5d} verts {mesh.face_count:5d} tris")
    html = viewer.build_html(payloads, resolver.pool, args.title)
    Path(args.out).write_text(html, encoding="utf-8")
    print(f"wrote {args.out} ({len(html.encode()) / 1e6:.1f} MB, {len(payloads)} maps)")
    return 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="openparkan", description=__doc__)
    ap.add_argument("--game", help="path to the Parkan: Iron Strategy directory")
    sub = ap.add_subparsers(dest="cmd", required=True)

    sub.add_parser("info", help="summarise the installation").set_defaults(fn=cmd_info)

    p = sub.add_parser("ls", help="list the members of an NRes archive")
    p.add_argument("archive")
    p.add_argument("--type", help="only entries with this FourCC tag")
    p.set_defaults(fn=cmd_ls)

    p = sub.add_parser("extract", help="write archive members out as files")
    p.add_argument("archive")
    p.add_argument("--out", default="extracted")
    p.add_argument("--type")
    p.add_argument("--by-type", action="store_true", help="suffix each file with its type tag")
    p.set_defaults(fn=cmd_extract)

    p = sub.add_parser("textures", help="decode a Texm archive to PNGs")
    p.add_argument("archive", nargs="?", default="Textures.lib")
    p.add_argument("--out", default="textures")
    p.add_argument("--alpha", action="store_true", help="keep the alpha channel (RGBA PNG)")
    p.add_argument("--verbose", action="store_true")
    p.set_defaults(fn=cmd_textures)

    sub.add_parser("maps", help="list maps and their terrain statistics").set_defaults(fn=cmd_maps)

    p = sub.add_parser("heightmap", help="render a map top-down to a PNG")
    p.add_argument("map")
    p.add_argument("--out", default="terrain.png")
    p.add_argument("--size", type=int, default=800)
    p.set_defaults(fn=cmd_heightmap)

    sub.add_parser(
        "verify", help="re-derive every claim in docs/ from the installed data files"
    ).set_defaults(fn=cmd_verify)

    p = sub.add_parser("viewer", help="build a self-contained 3D terrain viewer")
    p.add_argument("maps", nargs="*", help="map names; default is every map")
    p.add_argument("--out", default="terrain-viewer.html")
    p.add_argument("--title", default="Parkan Terrain Viewer")
    p.add_argument("--texture-size", type=int, default=64,
                   help="downsample terrain textures to at most this many pixels")
    p.set_defaults(fn=cmd_viewer)

    args = ap.parse_args(argv)
    try:
        game = gamedir.find(args.game)
    except gamedir.GameNotFound as exc:
        print(str(exc), file=sys.stderr)
        return 2
    return args.fn(args, game)
