# /// script
# requires-python = ">=3.10"
# dependencies = ["pillow"]
# ///
"""Frames, contact sheets, briefing pairs and the message box's text out of a playthrough video.

The mechanical half of `/playthrough` (`.claude/commands/playthrough.md`): what a recording shows,
cut out and laid side by side so it can be read and set against the engine. Needs `ffmpeg` and
`ffprobe` on the path, `tesseract` for `ocr`, and a release build of the engine for `pairs`.

    uv run analysis/playthrough.py probe VIDEO
    uv run analysis/playthrough.py frame VIDEO 110            # a whole frame, to find the game
    uv run analysis/playthrough.py --crop 2872:2154:964:3 sheets VIDEO --every 5
    uv run analysis/playthrough.py grab VIDEO 106 --width 1400
    uv run analysis/playthrough.py burst VIDEO 572 579 --fps 4
    uv run analysis/playthrough.py fade VIDEO 33.8 35.2       # a briefing's first shot fading in
    uv run analysis/playthrough.py pairs VIDEO --mission MISSIONS/CAMPAIGN/CAMPAIGN.02/Mission.01 \\
        --offset 29.5 6 10 14
    uv run analysis/playthrough.py ocr VIDEO --box 1100:330:1850:3 --from 100 --to 600

Every output goes under the work directory (`--work`, by default `.playthrough/<video stem>` in
the repository, which git ignores): the frames are the game's own and stay out of the repository.
`--crop W:H:X:Y` is the game's picture inside the video's frame, in the video's own pixels; given
once, it is kept in the work directory and used by every later call. Times are seconds, or
`m:ss`/`h:mm:ss`.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from difflib import SequenceMatcher
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFont

REPO = Path(__file__).resolve().parent.parent
ENGINE = REPO / "engine" / "target" / "release" / "parkan"
FONT = "/System/Library/Fonts/Menlo.ttc"


def seconds(text: str) -> float:
    """`95.5`, `1:35.5` or `0:01:35.5` as seconds."""
    total = 0.0
    for part in text.split(":"):
        total = total * 60 + float(part)
    return total


def stamp(t: float) -> str:
    return f"{int(t // 60):02d}:{t % 60:04.1f}"


def font(size: int) -> ImageFont.ImageFont:
    try:
        return ImageFont.truetype(FONT, size)
    except OSError:
        return ImageFont.load_default()


def ffmpeg(*args: str) -> None:
    subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", *args], check=True)


class Work:
    """The work directory, and the crop kept in it."""

    def __init__(self, video: Path, work: Path | None, crop: str | None):
        stem = re.sub(r"[^A-Za-z0-9._-]+", "_", video.stem)
        # A long title keeps its start and its end, where a download puts the video's id.
        if len(stem) > 80:
            stem = stem[:56] + "_" + stem[-23:]
        self.dir = work or REPO / ".playthrough" / stem
        self.dir.mkdir(parents=True, exist_ok=True)
        saved = self.dir / "crop"
        if crop:
            if not re.fullmatch(r"\d+:\d+:\d+:\d+", crop):
                sys.exit(f"--crop wants W:H:X:Y, not {crop}")
            saved.write_text(crop + "\n")
        self.crop = crop or (saved.read_text().strip() if saved.exists() else None)
        self.video = video

    def filter(self, width: int | None, *before: str) -> str:
        """The filter chain: `before`, the crop, then a scale to `width`."""
        chain = list(before)
        if self.crop:
            chain.append(f"crop={self.crop}")
        if width:
            chain.append(f"scale={width}:-2")
        return ",".join(chain) or "null"

    def path(self, *parts: str) -> Path:
        p = self.dir.joinpath(*parts)
        p.parent.mkdir(parents=True, exist_ok=True)
        return p


def tile(files: list[Path], labels: list[str], cols: int, out: Path, width: int = 480) -> None:
    """`files` in a grid of `cols`, each `width` wide and labelled in its top left."""
    first = Image.open(files[0])
    height = round(first.height * width / first.width)
    rows = (len(files) + cols - 1) // cols
    sheet = Image.new("RGB", (width * cols, height * rows), "black")
    draw = ImageDraw.Draw(sheet)
    ink = font(max(14, width // 22))
    for i, (f, label) in enumerate(zip(files, labels, strict=True)):
        x, y = (i % cols) * width, (i // cols) * height
        sheet.paste(Image.open(f).convert("RGB").resize((width, height)), (x, y))
        box = draw.textbbox((x + 4, y + 2), label, font=ink)
        draw.rectangle([x, y, box[2] + 4, box[3] + 2], fill="black")
        draw.text((x + 4, y + 2), label, fill="yellow", font=ink)
    sheet.save(out, quality=85)


def probe(args: argparse.Namespace, _: Work) -> None:
    entries = "format=duration:stream=codec_type,width,height,r_frame_rate"
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", entries, "-of", "json", str(args.video)],
        check=True, capture_output=True, text=True,
    ).stdout
    info = json.loads(out)
    print(f"duration {stamp(float(info['format']['duration']))}")
    for s in info["streams"]:
        if s.get("codec_type") == "video":
            print(f"video {s['width']}x{s['height']} at {s['r_frame_rate']} fps")


def frame(args: argparse.Namespace, w: Work) -> None:
    """A whole frame, uncropped, to find where the game's picture stands in the video."""
    out = w.path(f"frame_{args.time}.png")
    ffmpeg("-ss", str(seconds(args.time)), "-i", str(args.video), "-frames:v", "1", str(out))
    print(out)


def sheets(args: argparse.Namespace, w: Work) -> None:
    """One frame every `--every` seconds, sixteen to a contact sheet, each stamped with its time."""
    start = seconds(args.start)
    frames = w.path("frames", "x").parent
    shutil.rmtree(frames)
    frames.mkdir()
    span = ["-ss", str(start)] + (["-to", str(seconds(args.end))] if args.end else [])
    ffmpeg(*span, "-i", str(args.video), "-vf", w.filter(480, f"fps=1/{args.every}"), "-q:v", "3",
           str(frames / "f_%05d.jpg"))
    files = sorted(frames.glob("f_*.jpg"))
    per = args.cols * args.cols
    for n in range(0, len(files), per):
        chunk = files[n:n + per]
        labels = [stamp(start + (n + i) * args.every) for i in range(len(chunk))]
        out = w.path("sheets", f"sheet_{n // per:03d}.jpg")
        tile(chunk, labels, args.cols, out)
        print(out)


def grab(args: argparse.Namespace, w: Work) -> None:
    t = seconds(args.time)
    out = w.path(f"grab_{t:.2f}.jpg")
    ffmpeg("-ss", str(t), "-i", str(args.video), "-frames:v", "1", "-vf", w.filter(args.width),
           "-q:v", "2", str(out))
    print(out)


def burst(args: argparse.Namespace, w: Work) -> None:
    """Every frame at `--fps` from one time to another, tiled: a capture, a death, a cut."""
    t0, t1 = seconds(args.start), seconds(args.end)
    frames = w.path("burst", "x").parent
    shutil.rmtree(frames)
    frames.mkdir()
    ffmpeg("-ss", str(t0), "-i", str(args.video), "-t", str(t1 - t0), "-vf",
           w.filter(args.width, f"fps={args.fps}"), "-q:v", "3", str(frames / "b_%04d.jpg"))
    files = sorted(frames.glob("b_*.jpg"))
    out = w.path(f"burst_{t0:.1f}-{t1:.1f}.jpg")
    tile(files, [stamp(t0 + i / args.fps) for i in range(len(files))], args.cols, out, args.width)
    print(out)


def fade(args: argparse.Namespace, w: Work) -> None:
    """The mean brightness of the picture's middle and of the subtitle band, frame by frame: a
    briefing's black lead-in, its first shot fading in, and the subtitle changing."""
    t0, t1 = seconds(args.start), seconds(args.end)
    frames = w.path("fade", "x").parent
    shutil.rmtree(frames)
    frames.mkdir()
    ffmpeg("-ss", str(t0), "-i", str(args.video), "-t", str(t1 - t0), "-vf",
           w.filter(240, f"fps={args.fps}"), str(frames / "f_%04d.png"))

    def mean(im: Image.Image) -> float:
        data = im.tobytes()
        return sum(data) / max(1, len(data))

    print("time      picture  subtitles")
    for i, f in enumerate(sorted(frames.glob("f_*.png"))):
        im = Image.open(f).convert("L")
        width, height = im.size
        picture = mean(im.crop((0, int(height * 0.2), width, int(height * 0.8))))
        band = mean(im.crop((0, int(height * 0.9), width, height)))
        print(f"{t0 + i / args.fps:8.2f}  {picture:7.1f}  {band:9.2f}")


def pairs(args: argparse.Namespace, w: Work) -> None:
    """The video at `t + offset` beside the engine's briefing at `t`, six pairs to a sheet."""
    if not ENGINE.exists():
        sys.exit(f"no engine at {ENGINE}: cd engine && cargo build --release -p parkan")
    out_dir = w.path("pairs", "x").parent
    shots = []
    for t in args.times:
        video_shot = out_dir / f"v_{t}.jpg"
        engine_shot = out_dir / f"e_{t}.png"
        ffmpeg("-ss", str(float(t) + args.offset), "-i", str(args.video), "-frames:v", "1", "-vf",
               w.filter(640), "-q:v", "2", str(video_shot))
        briefing = ["--briefing-at", str(t), "--size", "640x480"]
        subprocess.run([str(ENGINE), "--mission", args.mission, "--screenshot", str(engine_shot),
                        *briefing], check=True, capture_output=True)
        shots.append((t, video_shot, engine_shot))
    for n in range(0, len(shots), 6):
        files, labels = [], []
        for t, v, e in shots[n:n + 6]:
            files += [v, e]
            labels += [f"VIDEO t={t}", f"ENGINE t={t}"]
        out = w.path(f"pairs_{n // 6:02d}.jpg")
        tile(files, labels, 4, out, 640)
        print(out)


def ocr(args: argparse.Namespace, w: Work) -> None:
    """The text in one box of the video's frame, once every `--every` seconds, printed only when
    it changes. The box is in the video's own pixels, not the crop's."""
    if not shutil.which("tesseract"):
        sys.exit("ocr wants tesseract on the path")
    if not re.fullmatch(r"\d+:\d+:\d+:\d+", args.box):
        sys.exit(f"--box wants W:H:X:Y, not {args.box}")
    start = seconds(args.start)
    frames = w.path("ocr", "x").parent
    shutil.rmtree(frames)
    frames.mkdir()
    span = ["-ss", str(start)] + (["-to", str(seconds(args.end))] if args.end else [])
    ffmpeg(*span, "-i", str(args.video), "-vf", f"fps=1/{args.every},crop={args.box}", "-q:v", "2",
           str(frames / "m_%05d.jpg"))

    def read(f: Path) -> tuple[int, list[str]]:
        r, g, b = Image.open(f).convert("RGB").split()
        # Keep the light text, drop the green panel under it, and read it at twice the size.
        least = ImageChops.darker(ImageChops.darker(r, g), b)
        mask = least.point(lambda v: 0 if v > args.threshold else 255)
        mask = mask.resize((mask.width * 2, mask.height * 2))
        png = f.with_suffix(".png")
        mask.save(png)
        text = subprocess.run(["tesseract", str(png), "-", "--psm", "6"], capture_output=True,
                              text=True).stdout
        n = int(re.search(r"(\d+)", f.stem).group(1))
        return n - 1, [line.strip() for line in text.splitlines() if len(line.strip()) > 3]

    with ThreadPoolExecutor(8) as pool:
        found = sorted(pool.map(read, sorted(frames.glob("m_*.jpg"))))
    # The same text read again with the recognition's noise is not a new line: it is compared,
    # letters only, with what was printed in the last `--hold` seconds, a message's time up.
    def letters(text: str) -> str:
        return re.sub(r"[^a-z]", "", text.lower())

    shown: list[tuple[float, str]] = []
    for i, lines in found:
        t, text = start + i * args.every, " | ".join(lines)
        key = letters(text)
        if not key:
            continue
        shown = [(when, k) for when, k in shown if t - when <= args.hold]
        if all(SequenceMatcher(None, key, k).ratio() < args.same for _, k in shown):
            print(f"{stamp(t)}  {text}")
            shown.append((t, key))


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    p.add_argument("--work", type=Path, help="work directory (default .playthrough/<video stem>)")
    p.add_argument("--crop", help="the game's picture in the video, W:H:X:Y, kept for later calls")
    sub = p.add_subparsers(dest="command", required=True)

    def command(name: str, run, help: str) -> argparse.ArgumentParser:
        c = sub.add_parser(name, help=help)
        c.add_argument("video", type=Path)
        c.set_defaults(run=run)
        return c

    command("probe", probe, "duration, resolution and frame rate")
    c = command("frame", frame, "one whole, uncropped frame")
    c.add_argument("time")
    c = command("sheets", sheets, "contact sheets, one frame every N seconds")
    c.add_argument("--every", type=float, default=5.0)
    c.add_argument("--from", dest="start", default="0")
    c.add_argument("--to", dest="end")
    c.add_argument("--cols", type=int, default=4)
    c = command("grab", grab, "one cropped frame")
    c.add_argument("time")
    c.add_argument("--width", type=int, default=1400)
    c = command("burst", burst, "every frame at --fps between two times, tiled")
    c.add_argument("start")
    c.add_argument("end")
    c.add_argument("--fps", type=float, default=4.0)
    c.add_argument("--width", type=int, default=360)
    c.add_argument("--cols", type=int, default=7)
    c = command("fade", fade, "brightness per frame: where a briefing's first shot fades in")
    c.add_argument("start")
    c.add_argument("end")
    c.add_argument("--fps", type=float, default=10.0)
    c = command("pairs", pairs, "the video beside the engine's briefing at the same times")
    c.add_argument("--mission", required=True, help="e.g. MISSIONS/CAMPAIGN/CAMPAIGN.02/Mission.01")
    c.add_argument("--offset", type=float, required=True, help="video time of briefing time 0")
    c.add_argument("times", nargs="+")
    c = command("ocr", ocr, "the text in one box of the frame, whenever it changes")
    c.add_argument("--box", required=True, help="W:H:X:Y in the video's own pixels")
    c.add_argument("--every", type=float, default=1.0)
    c.add_argument("--from", dest="start", default="0")
    c.add_argument("--to", dest="end")
    c.add_argument("--threshold", type=int, default=150, help="a text pixel's least channel")
    c.add_argument("--same", type=float, default=0.75,
                   help="how alike two readings are to count as one line (0 to 1)")
    c.add_argument("--hold", type=float, default=20.0,
                   help="seconds a printed line is remembered: a message box's time up")

    args = p.parse_args()
    if not shutil.which("ffmpeg"):
        sys.exit("playthrough wants ffmpeg on the path")
    work = Work(args.video, args.work, args.crop)
    args.run(args, work)


if __name__ == "__main__":
    main()
