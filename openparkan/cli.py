"""Command line entry point: ``python -m openparkan <command>``."""

from __future__ import annotations

import argparse
import sys
from collections import Counter
from pathlib import Path

from . import (
    behaviour,
    briefing,
    control,
    controls,
    descriptions,
    effects,
    font,
    gamedir,
    landmesh,
    mission,
    research,
    resources,
    rsli,
    save,
    settings,
    sky,
    texm,
    units,
    verify,
    viewer,
)
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
    path = _archive_path(game, args.archive)
    if rsli.is_rsli(path):
        archive = rsli.RsLiArchive.open(path)
        print(f"# {archive.source} -- {len(archive)} entries, RsLi, "
              f"{'presorted' if archive.presorted else 'unsorted'}")
        for e in archive:
            print(f"{e.storage:<8} {e.name:<34} {e.size:10d}  @{e.offset} "
                  f"({e.packed} packed)")
        return 0
    archive = NResArchive.open(path)
    print(f"# {archive.source} -- {len(archive)} entries, version 0x{archive.version:x}")
    for e in archive:
        if args.type and e.tag != args.type:
            continue
        print(f"{e.tag:<8} {e.name:<34} {e.size:10d}  @{e.offset}")
    return 0


def cmd_extract(args, game: Path) -> int:
    path = _archive_path(game, args.archive)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    if rsli.is_rsli(path):
        archive = rsli.RsLiArchive.open(path)
        for e in archive:
            (out / e.name).write_bytes(archive.read(e))
        print(f"extracted {len(archive)} members to {out}")
        return 0
    archive = NResArchive.open(path)
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


def _members(path: Path):
    """(name, bytes) for every member, whichever container this is."""
    if rsli.is_rsli(path):
        archive = rsli.RsLiArchive.open(path)
        for entry in archive:
            yield entry.name, archive.read(entry)
        return
    archive = NResArchive.open(path)
    for entry in archive:
        yield entry.name or f"unnamed_{entry.index}", archive.read(entry)


def cmd_textures(args, game: Path) -> int:
    path = _archive_path(game, args.archive)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    ok = failed = 0
    for name, blob in _members(path):
        try:
            tex = texm.decode(blob)
        except texm.UnsupportedTexture as exc:
            failed += 1
            if args.verbose:
                print(f"  skip {name:<24} {exc}", file=sys.stderr)
            continue
        stem = name.rsplit(".", 1)[0] or name
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


def cmd_controls(args, game: Path) -> int:
    """The input tables: what each key sends, and where."""
    keys = controls.scancodes(game)
    actions = controls.commands(game)
    if args.bindings:
        for path in sorted(game.glob("*.man")):
            bound = controls.bindings(path)
            print(f"{path.name}  ({len(bound)} bindings)")
            for b in bound:
                label = keys.get(b.key, "") or b.key
                print(f"  {label:<12} {b.command:<32} {actions.get(b.command, '')}")
        return 0

    for name in controls.TABLES:
        path = game / name
        if not path.exists():
            continue
        rows = controls.table(path)
        print(f"{name}  ({len(rows)} rows)")
        for r in rows:
            edge = "down" if r.pressed else "up  "
            chord = r.key if r.modifier == controls.NO_MODIFIER \
                else f"{r.modifier}+{r.key}"
            extra = f"  {r.state}" if r.state != "0" else ""
            ramp = f"  ramp {r.ramp:g}/{r.ramp_time}" if r.ramp or r.ramp_time else ""
            print(f"  {r.device:<5} {chord:<24} {edge}  {r.target:<18} "
                  f"{r.command:<14} {r.value:6.2f} {r.index:3d}{extra}{ramp}"
                  f"   {r.note}")
        print()

    schemes = controls.build_schemes(game)
    print(f"{controls.BUILD_SCHEMES}  ({len(schemes)} schemes, header says "
          f"{controls.BUILD_SCHEME_DECLARED})")
    for scheme in schemes:
        print(f"  {scheme.name:<16} {len(scheme.members)}  "
              + ", ".join(m.rsplit("\\", 1)[-1] for m in scheme.members))
    return 0


def cmd_control(args, game: Path) -> int:
    """List controllers, or show one .ctl in full."""
    names = frozenset(
        path.name.lower() for path in game.iterdir()
        if path.suffix.lower() in (".rlb", ".lib") and is_nres(path)
    )
    found: list[tuple[str, str, control.Controller]] = []
    for path in sorted(game.glob("*.rlb")) + sorted(game.glob("*.lib")):
        if not is_nres(path):
            continue
        archive = NResArchive.open(path)
        for entry in archive:
            if not entry.tag.upper().startswith("CTL"):
                continue
            if args.name and entry.name.lower() != args.name.lower():
                continue
            found.append((path.name, entry.name,
                          control.parse(archive.read(entry), names)))
    if not found:
        print(f"no such controller: {args.name}" if args.name
              else "no controllers found", file=sys.stderr)
        return 2

    if not args.name:
        refs = sum(len(c.named) for _l, _n, c in found)
        parts = sum(len(c.components) for _l, _n, c in found)
        print(f"{len(found)} controllers, {parts} components, {refs} named references")
        for lib, name, c in found:
            mark = " bare" if c.bare else ""
            print(f"  {lib:<14} {name:<26} parts {len(c.components):3d}  "
                  f"named {len(c.named):3d}{mark}")
        return 0

    for lib, name, c in found:
        print(f"{lib}/{name}   counts {c.counts}"
              f"{'   (frame only)' if c.bare else ''}")
        for at, triple in zip(control.TRIPLE_AT, c.triples, strict=True):
            print(f"  +{at:<4d} {triple[0]:12g} {triple[1]:12g} {triple[2]:12g}")
        print(f"  +92   {c.scale}   +96 {c.pair[0]:g}, {c.pair[1]:g}   "
              f"+104 {c.mode}   +116 {c.flags}")
        print(f"  bounds {c.bounds[0]:g}, {c.bounds[1]:g}   "
              f"cone {c.cone:.5f}   payload {c.payload:g}")
        for k in c.components:
            label = f"  {k.label}" if k.label else ""
            entries = f"  {len(k.entries)} entries" if k.entries else ""
            print(f"  part +{k.offset:<6d} type {k.type_id:2d}  {k.size:4d} bytes  "
                  f"{str(k.resource):<34}{label}{entries}")
        for r in c.references:
            if r.resource:
                print(f"  ref  +{r.offset:<6d} {str(r.resource):<40} {list(r.values)}")
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
        if args.floats:
            live = emitter.live_floats()
            aim = emitter.direction
            if aim:
                print(f"        direction +{effects.DIRECTION_AT} "
                      f"({aim[0]:g}, {aim[1]:g}, {aim[2]:g})")
            if live:
                print("        live  " + "  ".join(
                    f"+{at}={value:g}" for at, value in sorted(live.items())))
            else:
                print("        live  none -- this class reads nothing")
    return 0


def cmd_font(args, game: Path) -> int:
    """The game font: its glyph table, its atlas, and the palette behind it."""
    archive = rsli.RsLiArchive.open(game / "gamefont.rlb")
    glyphs = font.parse_font(archive.read_name("ARIALTEX.TFT"))
    palette = font.parse_palette(archive.read_name("PAL.PAL"))
    atlas = texm.decode(glyphs.atlas, palette.raw)
    drawn = glyphs.drawn
    print(f"ARIALTEX.TFT  {len(drawn)} glyphs of {font.GLYPH_COUNT} records, "
          f"{len(glyphs.rows)} rows, atlas {atlas.width}x{atlas.height} "
          f"format {atlas.fmt}")
    print(f"PAL.PAL       {sum(1 for c in palette.colours if any(c))} colours "
          f"and a {font.TABLE_SIDE}x{font.TABLE_SIDE} '{font.PALETTE_TAG.decode()}' "
          f"mixing table")
    if args.out:
        write_png(Path(args.out), atlas.width, atlas.height, texm.to_rgb(atlas))
        print(f"wrote {args.out}")
    if args.glyphs:
        for code in drawn:
            g = glyphs.glyphs[code]
            label = chr(code) if 32 <= code < 127 else f"\\x{code:02x}"
            print(f"  {code:>3} {label:<6} u {g.u0:.5f}..{g.u1:.5f}  v {g.v0:.5f}"
                  f"  {g.width(atlas.width):>2}px  advance {g.advance}")
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
            print(f"{path.name}/{entry.name:<24} kind {record.kind}"
                  f"  damage {record.damage:g}  radius {record.radius:g}"
                  f"  placement {record.placement}  -> "
                  + ", ".join(str(r) for r in record.effects))
    print(f"\n{shown} explosion definitions")
    return 0


def cmd_unit(args, game: Path) -> int:
    """Describe a unit assembly whole, or list them all."""
    workshop = units.Workshop(game)
    every = sorted(game.glob("UNITS/**/*.dat"))
    if not args.name:
        for path in every:
            sheet = workshop.describe(path)
            if sheet.chassis is None:
                continue
            turret = sheet.turret.name if sheet.turret else "-"
            print(f"{str(path.relative_to(game)):<36} {sheet.role:<9} "
                  f"{sheet.chassis.code:<7} {turret:<24} {sheet.firepower:6.0f} dmg/s")
        return 0
    want = args.name.lower().replace("\\", "/")
    found = [p for p in every
             if p.stem.lower() == want or str(p.relative_to(game)).lower().endswith(want)]
    if not found:
        print(f"no unit assembly matches {args.name!r}")
        return 1
    for path in found:
        print("\n".join(units.render(workshop.describe(path))))
        print()
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


def cmd_resources(args, game: Path) -> int:
    """List every resource descriptor, or print the text one library holds."""
    if args.text is not None:
        texts = resources.TextResources.open(game)
        pattern = args.text.lower()
        hits = [(n, i) for n, i in sorted(texts.names.items(), key=lambda kv: kv[1])
                if pattern in n.lower()]
        print(f"# {len(hits)} of {len(texts.names)} names match {args.text!r}")
        for name, ident in hits:
            body = (texts.table.get(ident) or "").replace("\n", " ")
            print(f"  {name:<14} {ident:>4}  {body}")
        return 0

    total = bound = 0
    print("# file                                        role              "
          "library              type       names")
    for path in sorted(game.rglob("*.cfg")):
        for d in resources.descriptors(path):
            total += 1
            library = resources.locate(game, d.library)
            where = library.name if library else f"MISSING {d.library}"
            bound += len(d)
            rel = path.relative_to(game).as_posix()
            print(f"  {rel:<44} {d.role:<17} {where:<20} "
                  f"{d.kind:<10} {len(d):>4}")
    print(f"\n  {total} descriptors binding {bound} names")
    return 0


def cmd_briefing(args, game: Path) -> int:
    """The camera flythrough a campaign mission opens on, and what it says."""
    paths = briefing.briefings(game)
    if not paths:
        print("no briefings found")
        return 1
    texts = resources.TextResources.open(game)

    if not args.mission:
        print(f"{len(paths)} briefings\n")
        for path in paths:
            stops = briefing.waypoints(path)
            spoken = sum(w.speaks for w in stops)
            msgs = path.parent / briefing.MESSAGES
            extra = f", {len(briefing.messages(msgs))} messages" if msgs.exists() else ""
            print(f"  {_mission_name(game, path.parent):<34} {len(stops):3} waypoints "
                  f"{sum(w.seconds for w in stops):6.1f}s  {spoken} spoken{extra}")
        return 0

    want = args.mission.lower().replace("\\", "/")
    match = [p for p in paths
             if want in _mission_name(game, p.parent).lower()]
    if len(match) != 1:
        print(f"{'no' if not match else 'more than one'} briefing matches "
              f"{args.mission!r}; try one of")
        for p in paths:
            print(f"  {_mission_name(game, p.parent)}")
        return 1

    path = match[0]
    sounds = _briefing_sounds(game, path.parent)
    stops = briefing.waypoints(path)
    print(f"# {_mission_name(game, path.parent)} -- {len(stops)} waypoints, "
          f"{sum(w.seconds for w in stops):.1f}s of camera\n")
    for i, w in enumerate(stops):
        cx, cy, cz = w.camera
        tx, ty, tz = w.target
        holds = ",".join(n for n, on in (
            ("text", w.wait_for_text), ("sound", w.wait_for_sound),
            ("time", w.wait_for_time), ("click", w.wait_for_click)) if on)
        print(f"  {i:3} {w.edge:<7}{w.wait:<11}{w.edge_time:5.1f}s "
              f"({cx:7.1f},{cy:7.1f},{cz:6.1f}) -> ({tx:7.1f},{ty:7.1f},{tz:6.1f})  "
              f"{holds}")
        if w.sound_id:
            print(f"      voice {sounds.get(w.sound_id, w.sound_id + ' (unbound)')}")
        if w.text_id:
            body = texts.get(w.text_id)
            if body is None:
                print(f"      text  {w.text_id}: no string in the shipped table")
            else:
                flat = " ".join(body.split())
                if len(flat) > 160:
                    flat = flat[:157] + "..."
                print(f'      "{flat}"')
    return 0


def _mission_name(game: Path, directory: Path) -> str:
    return directory.relative_to(game).as_posix().replace("MISSIONS/", "")


def _briefing_sounds(game: Path, directory: Path) -> dict[str, str]:
    cfg = directory / "mission.cfg"
    if not cfg.exists():
        return {}
    for d in resources.descriptors(cfg):
        if d.role == briefing.BRIEFING_ROLE:
            return dict(d.bindings)
    return {}


def cmd_settings(args, game: Path) -> int:
    """The engine's own configuration, and the progress it keeps."""
    registry_path = game / settings.COMPONENTS_FILE
    if registry_path.exists():
        print(f"# {settings.COMPONENTS_FILE} -- the component registry")
        for row in settings.registry(registry_path):
            print(f"  {row.cid}  {row.name:<24} {row.dll:<14} {row.function}")

    for name in (settings.BEHAVIOUR_FILE, settings.AREALMAP_FILE):
        path = game / name
        if not path.exists():
            continue
        table = settings.switches(path)
        print(f"\n# {name} -- {len(table)} switches")
        for key, value in table.items():
            shared = " (shared)" if key in settings.LOGGING else ""
            print(f"  {key:<20} {value}{shared}")

    path = game / settings.DISPLAY_FILE
    if path.exists():
        blocks = settings.sections(path)
        print(f"\n# {settings.DISPLAY_FILE} -- written by the game, "
              f"{sum(len(v) for v in blocks.values())} keys")
        for head, table in blocks.items():
            print(f"  [{head}]")
            for key, value in table.items():
                print(f"    {key:<22} {value}")

    done = settings.completed(game)
    if done:
        missions = {settings.dispatcher_key(game, d): d
                    for d in gamedir.missions(game)}
        print(f"\n# {settings.DISPATCHER_FILE[-1]} -- {len(done)} of "
              f"{len(missions)} missions completed on this install")
        for key in done:
            directory = missions.get(key)
            where = (directory.relative_to(game).as_posix() if directory
                     else f"{key}  (no such mission)")
            print(f"  {where}")
    return 0


def cmd_research(args, game: Path) -> int:
    """The research tree: what unlocks what."""
    paths = research.trees(game)
    if not paths:
        print("no .trf archives found")
        return 1
    chosen = paths[0]
    if args.file:
        want = args.file.lower()
        match = [p for p in paths if p.name.lower() == want or p.stem.lower() == want]
        if not match:
            print(f"no such tree: {args.file}; {len(paths)} available")
            return 1
        chosen = match[0]
    elif any(p.name == "auto.trf" for p in paths):
        chosen = next(p for p in paths if p.name == "auto.trf")

    tree = research.read(chosen)
    leaves = sum(1 for i in tree.items if i.leaf)
    print(f"{chosen.name}: {len(tree)} items, {tree.edges} prerequisites, "
          f"{len(tree.roots)} with none, {leaves} unlocking nothing")

    if args.item:
        found = tree.find(args.item)
        if not found:
            print(f"nothing matches {args.item!r}")
            return 1
        for item in found:
            print(f"\n{item.index}  {item.name}"
                  f"{f'  [{item.code}]' if item.code else ''}  ({item.kind})")
            print(f"  values   {', '.join(f'{v:g}' for v in item.values)}")
            print(f"  parts    {', '.join(item.parts) or '-'}")
            print(f"  requires {', '.join(tree[r].name for r in item.requires) or '-'}")
            print(f"  unlocks  {', '.join(tree[u].name for u in item.unlocks) or '-'}")
            print(f"  tail     {', '.join(str(b) for b in item.tail) or '-'}"
                  f"   (six fields at record +0x22..+0x27, unnamed)")
        return 0

    if args.hubs:
        gates = sorted((i for i in tree.items if i.unlocks),
                       key=lambda i: (-len(i.unlocks), i.index))
        for item in gates:
            print(f"  {len(item.unlocks):3}  {item.name}")
        return 0

    print()
    for line in research.render(tree, args.category):
        print(line)
    return 0


def cmd_saves(args, game: Path) -> int:
    """What each save game is, what it refers to, and what it holds."""
    catalogue = {}
    library = game / descriptions.LIBRARY
    if library.exists():
        catalogue = descriptions.read(library)
    index = {}
    try:
        index = {x.filename.lower(): x for x in save.slots(game)}
    except OSError:
        pass
    paths = save.saves(game)
    if not paths:
        print("no saves found")
        return 1
    for path in paths:
        try:
            s = save.read(path)
        except save.SaveFormatError as exc:
            print(f"{path.name}: {exc}")
            continue
        label = index.get(path.name.lower())
        kind = "campaign" if s.campaign else "single"
        print(f"{path.name}  {path.stat().st_size:>7} bytes  {kind}"
              f"{f'  {label.name!r}' if label and label.name else ''}")
        print(f"  mission  {s.mission}")
        print(f"  map      {s.map}")
        print(f"  trees    {', '.join(s.trees) or '-'}")
        print(f"  body     {len(s.blobs)} length-prefixed blobs, "
              + ", ".join(f"{b.size}" for b in s.blobs))
        print(f"  refers to {len(s.references)} archive members, "
              f"{len(s.members)} distinct")

        world = [r for r in s.references if r.field == save.MEMBER_AT[1]]
        parts = [r for r in s.references if r.field == save.MEMBER_AT[0]]
        scenery = [r for r in world if r.member.startswith(save.SCENERY)]
        print(f"  world    {len(world)} objects -- {len(scenery)} scenery, "
              f"{len(world) - len(scenery)} built")
        print(f"  parts    {len(parts)} records", end="")
        if catalogue:
            known = [r for r in parts if r.member in catalogue]
            kinds = Counter(catalogue[r.member].kind for r in known)
            print(f", {len(known)} named in {descriptions.LIBRARY} -- "
                  + ", ".join(f"{n} {k}" for k, n in kinds.most_common()))
        else:
            print()

        if args.members:
            print("\n    world objects:")
            for r in world:
                print(f"      {r.member}")
            print("    parts:")
            for name, count in Counter(r.member for r in parts).most_common():
                part = catalogue.get(name)
                label = f"  {part.name}" if part else ""
                print(f"      {name:16} x{count}{label}")
        print()
    return 0


def cmd_behaviour(args, game: Path) -> int:
    """The mission AI scripts, as pseudo-code."""
    paths = behaviour.scripts(game)
    if not paths:
        print("no .scr files found")
        return 1
    if not args.script:
        table = behaviour.variables(game)
        print(f"{len(paths)} scripts, {len(table)} variables in {behaviour.VARSET}\n")
        for path in paths:
            script = behaviour.read(path)
            print(f"  {path.name:16} {len(script.handlers):3} handlers "
                  f"{script.nodes:4} nodes   {', '.join(script.problems) or '-'}")
        return 0

    want = args.script.lower()
    match = [p for p in paths if p.name.lower() == want or p.stem.lower() == want]
    if not match:
        print(f"no such script: {args.script}")
        return 1
    script = behaviour.read(match[0])
    table = behaviour.variables(game)
    if args.handler and script.handler(args.handler) is None:
        print(f"{match[0].name} has no handler {args.handler!r}; it has "
              + ", ".join(h.name for h in script.handlers))
        return 1
    print(f"{match[0].name}: {len(script.handlers)} handlers, {script.nodes} nodes")
    print("# fnN, opN and tagN are numbered, not named -- the shipped files say "
          "what they take, not what they do\n")
    for line in behaviour.render(script, table, args.handler or ""):
        print(line)
    return 0


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

    p = sub.add_parser("controls", help="the input tables and key bindings")
    p.add_argument("--bindings", action="store_true",
                   help="list the .man key bindings instead of the tables")
    p.set_defaults(fn=cmd_controls)

    p = sub.add_parser("control", help="list .ctl controllers, or show one")
    p.add_argument("name", nargs="?", help="a .ctl member name; default is a listing")
    p.set_defaults(fn=cmd_control)

    p = sub.add_parser("behaviour", help="the .scr mission AI, as pseudo-code")
    p.add_argument("script", nargs="?", help="a .scr name; default is a listing")
    p.add_argument("handler", nargs="?", help="one handler; default is all of them")
    p.set_defaults(fn=cmd_behaviour)

    sub.add_parser(
        "settings", help="the engine's own .ini files and the progress it keeps"
    ).set_defaults(fn=cmd_settings)

    p = sub.add_parser(
        "briefing", help="a campaign mission's opening flythrough, and its script")
    p.add_argument("mission", nargs="?", help="part of a mission's path")
    p.set_defaults(fn=cmd_briefing)

    p = sub.add_parser(
        "resources", help="the .cfg resource descriptors, and the text they reach")
    p.add_argument("--text", metavar="PATTERN",
                   help="print the game's own text whose name contains PATTERN")
    p.set_defaults(fn=cmd_resources)

    p = sub.add_parser("saves", help="what each save game is, and what it refers to")
    p.add_argument("--members", action="store_true",
                   help="list every archive member the save names")
    p.set_defaults(fn=cmd_saves)

    p = sub.add_parser("research", help="the research tree from a .trf archive")
    p.add_argument("item", nargs="?", help="show one item by name instead of the tree")
    p.add_argument("--file", help="which .trf to read; default auto.trf")
    p.add_argument("--category", type=int, help="only roots in this category")
    p.add_argument("--hubs", action="store_true",
                   help="list the items that gate the most, commonest first")
    p.set_defaults(fn=cmd_research)

    p = sub.add_parser("effects", help="list effects, or describe one")
    p.add_argument("name", nargs="?", help="an FXID name; default is a listing")
    p.add_argument("--floats", action="store_true",
                   help="show the block floats the engine actually reads")
    p.set_defaults(fn=cmd_effects)

    p = sub.add_parser("font", help="the game font and its palette")
    p.add_argument("--out", help="write the glyph atlas to this PNG")
    p.add_argument("--glyphs", action="store_true", help="list every glyph")
    p.set_defaults(fn=cmd_font)

    sub.add_parser(
        "explosions", help="every .exp and the effects it sets off"
    ).set_defaults(fn=cmd_explosions)

    p = sub.add_parser("unit", help="describe a unit assembly whole, or list them")
    p.add_argument("name", nargs="?", help="a .dat name or path under UNITS, e.g. w_b_trk1")
    p.set_defaults(fn=cmd_unit)

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
