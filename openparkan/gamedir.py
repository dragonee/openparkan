"""Locating a Parkan: Iron Strategy installation."""

from __future__ import annotations

import os
from pathlib import Path

ENV_VAR = "PARKAN_DIR"

#: Files that must all be present for a directory to be a Parkan install.
MARKERS = ("Textures.lib", "MISSIONS", "DATA")

_DEFAULT_GUESSES = (
    Path(__file__).resolve().parent.parent.parent / "Parkan Iron Strategy",
    Path.home() / "Library/Application Support/Steam/steamapps/common/Parkan Iron Strategy",
    Path("C:/Program Files (x86)/Steam/steamapps/common/Parkan Iron Strategy"),
)


class GameNotFound(FileNotFoundError):
    pass


def looks_like_install(path: Path) -> bool:
    return all((path / m).exists() for m in MARKERS)


def find(explicit: str | os.PathLike | None = None) -> Path:
    """Resolve the game directory from an explicit path, ``$PARKAN_DIR``, or a guess."""
    candidates = []
    if explicit:
        candidates.append(Path(explicit))
    if os.environ.get(ENV_VAR):
        candidates.append(Path(os.environ[ENV_VAR]))
    candidates.extend(_DEFAULT_GUESSES)
    for c in candidates:
        c = c.expanduser()
        if looks_like_install(c):
            return c
    raise GameNotFound(
        "could not find a Parkan install (looked for {}).\n"
        "Pass --game /path/to/'Parkan Iron Strategy' or set {}.".format(", ".join(MARKERS), ENV_VAR)
    )


def maps(game: Path) -> list[Path]:
    """Every map directory that contains a terrain mesh."""
    root = game / "DATA" / "MAPS"
    found = (d for d in root.iterdir() if (d / "Land.msh").exists())
    return sorted(found, key=lambda p: p.name.lower())


def missions(game: Path) -> list[Path]:
    """Every mission directory, single, multiplayer and campaign."""
    root = game / "MISSIONS"
    out = [d for d in root.iterdir() if d.is_dir() and (d / "data.tma").exists()]
    campaign = root / "CAMPAIGN"
    if campaign.exists():
        for c in sorted(campaign.iterdir()):
            if c.is_dir():
                out.extend(d for d in sorted(c.iterdir()) if (d / "data.tma").exists())
    return sorted(out, key=lambda p: str(p).lower())
