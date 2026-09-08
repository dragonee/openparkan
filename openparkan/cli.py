"""Command line entry point: ``python -m openparkan <command>``."""

from __future__ import annotations

import argparse
import sys
from collections import Counter
from pathlib import Path

from . import effects, gamedir, landmesh, mission, sky, texm, verify, viewer
from .nres import NotAnNResArchive, NResArchive, is_nres
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


def cmd_missions(args, game: Path) -> int:
    for d in gamedir.missions(game):
        m = mission.load(d / "data.tma")
        kinds = Counter(o.kind_name for o in m.objects)
        print(
            f"{str(d.relative_to(game)):<42} {m.map_name:<16} "
            f"{len(m.clans)} clans  {len(m.objects):4d} objects  "
            f"({kinds['building']}b/{kinds['unit']}u/"
            f"{kinds['vegetation'] + kinds['rock']}s)  {m.title}"
        )
    return 0


def cmd_sky(args, game: Path) -> int:
    """Print a mission's atmosphere: its day cycle, keyframe by keyframe."""
    folders = gamedir.missions(game) if not args.mission else [
        Path(args.mission) if (Path(args.mission) / "sky.ske").exists()
        else game / "MISSIONS" / args.mission
    ]
    shown = 0
    for folder in folders:
        path = folder / "sky.ske"
        if not path.exists():
            continue
        atmosphere = sky.load(path)
        shown += 1
        label = str(folder.relative_to(game))
        print(f"{label}  {len(atmosphere)} keyframes, {atmosphere.sections} section(s)")
        if atmosphere.textures:
            print(f"  sky.wea      {', '.join(t for t in atmosphere.textures if t)}")
        if not args.frames:
            frame = atmosphere.brightest()
            if frame:
                r, g, b, _ = frame.sky
                print(f"  brightest    {frame.hour:02d}:{frame.minute:02d}  "
                      f"sky #{r:02x}{g:02x}{b:02x}  light {frame.light:.2f}")
            continue
        for frame in atmosphere.keyframes:
            r, g, b, _ = frame.sky
            extra = []
            if frame.name:
                extra.append(frame.name)
            if frame.sounds:
                extra.append(", ".join(frame.sounds))
            print(f"    {frame.hour:02d}:{frame.minute:02d}  sky #{r:02x}{g:02x}{b:02x}  "
                  f"light {frame.light:5.2f}  section {frame.section}"
                  + (f"  {' | '.join(extra)}" if extra else ""))
    if not shown:
        print("no sky.ske found", file=sys.stderr)
        return 2
    return 0


def cmd_effects(args, game: Path) -> int:
    """List effects, or describe one and everything it reaches."""
    library = effects.EffectLibrary(game / "effects.rlb")
    if not args.name:
        kinds: dict[int, int] = {}
        for effect in library:
            for emitter in effect.emitters:
                kinds[emitter.kind] = kinds.get(emitter.kind, 0) + 1
        print(f"{len(library)} effects, "
              f"{sum(len(e.emitters) for e in library)} emitters")
        print("  emitter types " + ", ".join(
            f"{k}x{n}" for k, n in sorted(kinds.items())))
        for effect in sorted(library, key=lambda e: e.name.lower()):
            drawn = len(effect.materials)
            heard = len(effect.sounds)
            print(f"  {effect.name:<26} {len(effect.emitters):2d} emitters"
                  f"  {drawn} drawn, {heard} heard")
        return 0

    effect = library.get(args.name)
    if effect is None:
        print(f"no such effect: {args.name}", file=sys.stderr)
        return 2
    print(f"{effect.name}  {len(effect.emitters)} emitters")
    for i, emitter in enumerate(effect.emitters):
        flag = " flagged" if emitter.flagged else ""
        what = str(emitter.resource) if emitter.resource else "-"
        span = emitter.audible_range
        heard = f"   audible {span[0]:g}..{span[1]:g}" if span else ""
        print(f"  {i:2d}  type {emitter.kind:2d}{flag:8s}  "
              f"{len(emitter.body):3d} bytes  {what}{heard}")
    return 0


def cmd_explosions(args, game: Path) -> int:
    """Every .exp in the installation, and the effects it sets off."""
    shown = 0
    for path in sorted(game.glob("*.rlb")) + sorted(game.glob("*.lib")):
        try:
            archive = NResArchive.open(path)
        except NotAnNResArchive:
            continue
        for entry in archive:
            if not entry.name.lower().endswith(".exp"):
                continue
            record = effects.parse_explosion(archive.read(entry), entry.name)
            shown += 1
            print(f"{path.name}/{entry.name:<24} magnitude {record.magnitude:5.1f}"
                  f"  flags {record.flags}  -> "
                  + ", ".join(str(r) for r in record.effects))
    print(f"\n{shown} explosion definitions")
    return 0


def cmd_mission(args, game: Path) -> int:
    d = Path(args.mission)
    if not (d / "data.tma").exists():
        d = game / "MISSIONS" / args.mission
    if not (d / "data.tma").exists():
        print(f"no mission at {args.mission}", file=sys.stderr)
        return 2
    m = mission.load(d / "data.tma")
    print(f"{m.source}")
    print(f"  description  {m.title}")
    if m.description and m.description != m.title:
        print(f"  (in data.tma) {m.description!r}")
    print(f"  map          {m.map_name}  ({m.map_path})")
    print(f"  viewpoints   {len(m.viewpoints)}")
    if m.routes:
        pts = ", ".join(f"#{r.id}:{len(r.points)}pt" for r in m.routes)
        print(f"  routes       {len(m.routes)}  ({pts})")
    print(f"  clans        {len(m.clans)}")
    for c in m.clans:
        allies = [n for n, v in c.relations.items() if v and n != c.name]
        print(
            f"    [{c.index}] {c.name:<10} base=({c.base[0]:.0f}, {c.base[1]:.0f})  "
            f"ai={c.ai_script.rsplit(chr(92), 1)[-1]}"
            + (f"  zones={len(c.zones)}" if c.zones else "")
            + (f"  allied={allies}" if allies else "")
        )
    print(f"  objects      {len(m.objects)}")
    for kind, group in sorted(
        _group(m.objects, lambda o: o.kind_name).items(), key=lambda kv: -len(kv[1])
    ):
        print(f"    {kind:<12} {len(group):4d}  {_top_names(group)}")
    if args.list:
        print("\n  id          clan  kind        position                    definition")
        for o in m.objects:
            x, y, z = o.position
            print(
                f"    {o.logical_id:<11} {str(o.clan_id):<5} {o.kind_name:<11} "
                f"({x:7.1f},{y:7.1f},{z:6.1f})  {o.path}"
            )
    cfg = d / "mission.cfg"
    if cfg.exists():
        blocks = mission.load_cfg(cfg)
        for name in ("primary_objectives", "bonus_objectives"):
            if blocks.get(name):
                print(f"\n  {name.replace('_', ' ')}:")
                for v in blocks[name].values():
                    print(f"    {v}")
    return 0


def _group(items, key):
    out = {}
    for it in items:
        out.setdefault(key(it), []).append(it)
    return out


def _top_names(objs, limit: int = 4) -> str:
    names = Counter(o.path.replace(chr(92), "/").rsplit("/", 1)[-1] for o in objs)
    return ", ".join(f"{n}x{c}" if c > 1 else n for n, c in names.most_common(limit))


def cmd_verify(args, game: Path) -> int:
    return verify.run(game)


def cmd_viewer(args, game: Path) -> int:
    names = args.maps or [d.name for d in gamedir.maps(game)]
    resolver = viewer.TextureResolver(game, max_size=args.texture_size)
    payloads = []
    index_of = {}
    for name in names:
        path = game / "DATA" / "MAPS" / name / "Land.msh"
        if not path.exists():
            print(f"no such map: {name}", file=sys.stderr)
            return 2
        mesh = landmesh.load(path)
        index_of[name] = len(payloads)
        payloads.append(viewer.build_map_payload(mesh, resolver, name))
        print(f"  packed {name:<16} {mesh.vertex_count:5d} verts {mesh.face_count:5d} tris")

    missions = []
    models = None if args.no_geometry else viewer.ModelLibrary(game, resolver)
    if not args.no_missions:
        for d in gamedir.missions(game):
            m = mission.load(d / "data.tma")
            if m.map_name in index_of:
                missions.append(viewer.build_mission_payload(m, index_of[m.map_name], models))
        print(f"  packed {len(missions)} missions, "
              f"{sum(len(x['objects']) for x in missions)} placed objects")
        if models:
            print(f"  packed {len(models.models)} object meshes, "
                  f"{sum(x['tris'] for x in models.models)} triangles")

    html = viewer.build_html(
        payloads, resolver.pool, missions, models.models if models else [], args.title
    )
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

    sub.add_parser("missions", help="list every mission with its map and contents").set_defaults(
        fn=cmd_missions
    )

    p = sub.add_parser("mission", help="describe one mission in detail")
    p.add_argument("mission", help="a mission directory, or a name under MISSIONS/")
    p.add_argument("--list", action="store_true", help="list every placed object")
    p.set_defaults(fn=cmd_mission)

    sub.add_parser(
        "verify", help="re-derive every claim in docs/ from the installed data files"
    ).set_defaults(fn=cmd_verify)

    p = sub.add_parser("sky", help="show a mission's atmosphere and its day cycle")
    p.add_argument("mission", nargs="?", help="mission directory; default is every mission")
    p.add_argument("--frames", action="store_true", help="list every keyframe")
    p.set_defaults(fn=cmd_sky)

    p = sub.add_parser("effects", help="list effects, or describe one")
    p.add_argument("name", nargs="?", help="an FXID name; default is a listing")
    p.set_defaults(fn=cmd_effects)

    sub.add_parser(
        "explosions", help="every .exp and the effects it sets off"
    ).set_defaults(fn=cmd_explosions)

    p = sub.add_parser("viewer", help="build a self-contained 3D terrain viewer")
    p.add_argument("maps", nargs="*", help="map names; default is every map")
    p.add_argument("--out", default="terrain-viewer.html")
    p.add_argument("--title", default="Parkan Terrain Viewer")
    p.add_argument("--texture-size", type=int, default=64,
                   help="downsample terrain textures to at most this many pixels")
    p.add_argument("--no-missions", action="store_true",
                   help="terrain only; omit mission object placement")
    p.add_argument("--no-geometry", action="store_true",
                   help="draw objects as markers instead of their real meshes")
    p.set_defaults(fn=cmd_viewer)

    args = ap.parse_args(argv)
    try:
        game = gamedir.find(args.game)
    except gamedir.GameNotFound as exc:
        print(str(exc), file=sys.stderr)
        return 2
    return args.fn(args, game)
