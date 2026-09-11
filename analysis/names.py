"""Recover the engine's own name-to-number tables from a DLL.

The game parses several of its data files as **text** -- ``hero.tbl`` names a
command as ``MCMD_LEFT``, a component class as ``CICLS_TURRET``, a key as
``SCAN_A`` -- so every binary that reads one of those files has to carry a
resolver that turns each name into the number the engine actually dispatches
on.  Those resolvers are tables of the developers' own vocabulary, and they
are the cheapest naming evidence in the whole installation.

A resolver is a chain of string compares.  Three shapes turn up, and all four
matter -- missing them is how one pass lost six entries and another read a
whole family as a table of zeros:

1. the ordinary case ::

       push offset "MCMD_LEFT"
       push esi
       call strcmp
       add  esp, 8
       test eax, eax
       jne  next
       mov  eax, 9          ; <- the value
       ret

2. a case that returns the zero ``strcmp`` already left in ``eax`` ::

       jne  next
       pop  esi
       ret                  ; <- the value is 0

3. a case that writes the value to an out-parameter rather than returning
   it, which is what ``iron3d.dll``'s ``CMD_`` resolver does ::

       jne  next
       mov  dword ptr [edi], 0x2ed    ; <- the value
       mov  al, 1                     ; and a success flag

4. the chain's last case, written branchlessly ::

       call strcmp
       neg  eax
       sbb  eax, eax        ; 0 on a match, -1 otherwise
       and  eax, M
       add  eax, K          ; <- K on a match, M+K (the default) otherwise

Run it::

    uv run --group analysis python analysis/names.py World3D.dll
    uv run --group analysis python analysis/names.py Net.dll --prefix GMSG

What it prints is evidence, not a result.  Check a table against the shipped
files before writing anything down: the scan codes were confirmed by matching
the real IBM PC set-1 numbers and by ``ScanCode.dsc`` listing its first 56
entries in code order.
"""

from __future__ import annotations

import argparse
import collections
import re
import sys
from pathlib import Path

import capstone
import pefile

#: A name worth resolving: a prefix, an underscore, then more capitals.
#: One character after the underscore is enough -- ``SCAN_A`` is a key, and
#: requiring two lost all 26 letters on the first pass.
NAME = re.compile(r"[A-Z][A-Z0-9]{1,9}_[A-Z0-9_]{1,30}$")

#: How many instructions after the push a case is allowed to take.
WINDOW = 14


class Image:
    """A PE file with the two lookups this needs."""

    def __init__(self, path: Path):
        self.path = path
        self.raw = path.read_bytes()
        self.pe = pefile.PE(str(path))
        self.base = self.pe.OPTIONAL_HEADER.ImageBase
        self.text = next(s for s in self.pe.sections if b".text" in s.Name)

    def string_at(self, va: int, limit: int = 64) -> str | None:
        for sec in self.pe.sections:
            lo = self.base + sec.VirtualAddress
            if not lo <= va < lo + sec.Misc_VirtualSize:
                continue
            off = sec.PointerToRawData + (va - lo)
            end = self.raw.find(b"\0", off, off + limit)
            if end < 0:
                return None
            text = self.raw[off:end]
            if text and all(32 <= b < 127 for b in text):
                return text.decode("latin-1")
        return None


def sweep(image: Image) -> dict[str, int]:
    """Every ``(name, value)`` pair the image's resolvers hand out.

    Disassembly restarts a byte later whenever capstone stalls: a linear sweep
    over ``.text`` desynchronises on jump tables and inline data, and without
    the restart everything past the first one is invisible.
    """
    data = image.text.get_data()
    va0 = image.base + image.text.VirtualAddress
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)

    found: dict[str, int] = {}
    pending: str | None = None
    compared = tested = branchless = guarded = False
    window = 0
    off = 0
    while off < len(data):
        moved = False
        for ins in md.disasm(data[off:], va0 + off):
            moved = True
            off = ins.address - va0 + ins.size
            text = f"{ins.mnemonic} {ins.op_str}"

            match = re.fullmatch(r"push (0x[0-9a-f]+)", text)
            if match:
                name = image.string_at(int(match.group(1), 16))
                if name and NAME.fullmatch(name):
                    pending = name
                    compared = tested = branchless = guarded = False
                    window = WINDOW
                continue

            if not pending:
                continue
            # A case is a handful of instructions.  Without a window, a pushed
            # string keeps matching against whatever ret or mov turns up next,
            # which reads whole families as tables of zeros.
            window -= 1
            if window <= 0:
                pending = None
                continue
            if ins.mnemonic == "call":
                compared = True
                continue
            # A resolver tests the compare's result.  Without this, any pushed
            # string followed by a call and a guarded ret reads as a case
            # returning zero -- which is how 36 VOICE_ names in iron3d.dll,
            # pushed as arguments to a sound lookup, came out as a table of
            # zeros on an earlier pass.
            if compared and text in ("test eax, eax", "neg eax"):
                tested = True
                continue
            # The chain's final case needs no test when the case and the
            # default are both zero, so the compiler elides it: MCMD_DUMMY is
            # `call strcmp; xor eax, eax; ret`.
            if compared and text == "xor eax, eax":
                found.setdefault(pending, 0)
                pending = None
                continue
            if not tested:
                continue
            match = re.fullmatch(
                r"mov dword ptr \[e[a-z][a-z]\], (0x[0-9a-f]+|\d+)", text)
            if match:                                    # shape 4
                found.setdefault(pending, int(match.group(1), 0))
                pending = None
                continue
            match = re.fullmatch(r"mov eax, (0x[0-9a-f]+|\d+)", text)
            if match:                                    # shape 1
                found.setdefault(pending, int(match.group(1), 0))
                pending = None
            elif text == "xor eax, eax":                 # shape 1, value zero
                found.setdefault(pending, 0)
                pending = None
            elif branchless and text.startswith("add eax, "):
                found.setdefault(pending, int(text.split(", ")[1], 0))  # shape 3
                pending = None
            elif ins.mnemonic == "ret" and guarded:      # shape 2
                found.setdefault(pending, 0)
                pending = None
            elif text == "sbb eax, eax":
                branchless = True
            elif ins.mnemonic == "jne":
                guarded = True
        if not moved:
            off += 1
    return found


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("binary", help="a DLL or EXE in the game directory")
    ap.add_argument("--game", help="path to the installation")
    ap.add_argument("--prefix", help="show only this family")
    args = ap.parse_args(argv)

    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
    from openparkan import gamedir

    path = Path(args.game or gamedir.find()) / args.binary
    pairs = sweep(Image(path))

    families: dict[str, dict[str, int]] = collections.defaultdict(dict)
    for name, value in pairs.items():
        families[name.split("_")[0]][name] = value

    wanted = [args.prefix] if args.prefix else sorted(
        families, key=lambda k: -len(families[k]))
    for prefix in wanted:
        table = families.get(prefix)
        if not table:
            print(f"no {prefix}_ names in {path.name}")
            continue
        print(f"=== {prefix}_ ({len(table)}) ===")
        for name, value in sorted(table.items(), key=lambda kv: (kv[1], kv[0])):
            print(f"  {name:32s} {value}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
