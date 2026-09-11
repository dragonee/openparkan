"""Check ``Comp.ini``'s registry against the DLLs it names.

The component registry is a table of ``CID  DLL  Function`` rows, and
``World3D.dll`` turns each one into an entry point at load time: ``LoadLibraryA``
on the DLL, ``GetProcAddress`` on the function (the reader is at
``0x10014790``, and its own error string calls it ``LoadComponentAddr``).

This is the reproducer for the one claim in `docs/22-settings.md` the library
cannot make on its own, because checking it means reading a PE export table::

    uv run --group analysis python analysis/registry.py
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from pe import Binary


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--game", help="path to the installation")
    args = ap.parse_args(argv)

    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
    from openparkan import gamedir, settings

    game = Path(args.game or gamedir.find())
    rows = settings.registry(game / settings.COMPONENTS_FILE)

    tables: dict[str, dict[str, int]] = {}
    missing = 0
    for row in rows:
        if row.dll not in tables:
            path = next((p for p in game.iterdir()
                         if p.name.lower() == row.dll.lower()), None)
            tables[row.dll] = Binary(str(path)).exports() if path else {}
        address = tables[row.dll].get(row.function)
        missing += address is None
        print(f"  {row.cid}  {row.name:<24} {row.dll:<14} {row.function:<18} "
              + (f"{address:#010x}" if address else "MISSING"))
    print(f"\n{len(rows) - missing}/{len(rows)} rows resolve to a real export")
    return 1 if missing else 0


if __name__ == "__main__":
    raise SystemExit(main())
