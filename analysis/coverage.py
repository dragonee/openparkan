"""A ledger of the game's code: what has been read, and what has not.

The install is **15800 functions over 3.4 MB** of x86.  Reading all of it is a
finite job, but only if there is a ledger, because nothing in a stripped
release build tells you whether you have been somewhere before.  This builds
that ledger and answers three questions:

* **how much is done** -- per module and overall, in functions and in bytes;
* **which entries are wrong** -- every address the notes publish is checked
  against the binary it claims, which is not something prose can do;
* **what to read next** -- the unread functions of a module, ranked by the
  evidence that they touch the game's own data rather than the CRT's.

Two things make the job smaller than 15800.  Masking the addresses the loader
relocates makes a library function linked into two DLLs compare equal, which
collapses the set to **11717 distinct bodies**; and the weight is lopsided --
3468 functions above 256 bytes hold 74% of all the code there is.

Function boundaries come from four sources, because no single one is enough in
a build with no symbols: direct call targets, the export table, the addresses
data sections point at, and the end of a padding run.  `int3` and `nop` are
both used as padding here, in different modules.  A pointer out of a data
section is only believed when it lands after padding or on a call target --
otherwise a dword that happens to look like an address splits a real function
in two and invents one that was never there.

    uv sync --group analysis
    uv run --group analysis python analysis/coverage.py                 # the ledger
    uv run --group analysis python analysis/coverage.py --check         # is it true?
    uv run --group analysis python analysis/coverage.py --unknown Terrain.dll
    uv run --group analysis python analysis/coverage.py --closure World3D.dll:0x10014790

Needs Python 3.11 for `tomllib`.  See ../docs/09-method.md: this recovers
format facts, and no code is copied out of the binaries.
"""

from __future__ import annotations

import argparse
import hashlib
import re
import sys
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path

import capstone
import pefile
import tomllib

X = capstone.x86

GAME = Path("/Users/dragonee/Kod/parkan/Parkan Iron Strategy")
LEDGER = Path(__file__).with_name("known.toml")

#: The game's own modules.  `libogg`, `libvorbis`, `winmm` and the installer
#: helper are third-party and are nobody's job to read.
MODULES = (
    "World3D.dll", "Terrain.dll", "AniMesh.dll", "iron3d.dll", "Effect.dll",
    "Control.dll", "MisLoad.dll", "ai.dll", "Behavior.dll", "ArealMap.dll",
    "Ngi32.dll", "services.dll", "Net.dll", "Wizard.dll", "Joystick.dll",
    "iron_3d.exe",
)

#: Padding between functions.  Both appear, in different modules.
PADDING = re.compile(rb"(\xcc{2,}|\x90{2,})")

#: A string worth counting as evidence that a function touches game data.
PRINTABLE = re.compile(rb"[\x20-\x7e]{4,}\x00")

#: `function` an entry point; `site` one instruction inside one; `datum` data
#: outside `.text`; `table` data inside it, which is where this compiler puts
#: its jump tables.
KINDS = ("function", "site", "datum", "table")


@dataclass
class Function:
    at: int
    size: int
    group: str              # hash of the relocation-masked body
    callers: int = 0
    strings: list[str] = field(default_factory=list)

    @property
    def end(self) -> int:
        return self.at + self.size


@dataclass
class Entry:
    """One line of the ledger."""
    module: str
    at: int
    kind: str
    name: str
    doc: str = ""


class Module:
    """One binary, cut into functions."""

    def __init__(self, path: Path):
        self.path = path
        self.name = path.name
        self.pe = pefile.PE(str(path))
        self.base = self.pe.OPTIONAL_HEADER.ImageBase
        self.raw = path.read_bytes()
        text = next(s for s in self.pe.sections if b".text" in s.Name)
        self.text = text
        self.va0 = self.base + text.VirtualAddress
        self.data = bytes(text.get_data()[:text.Misc_VirtualSize])
        self.end = self.va0 + len(self.data)
        self.md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
        self.md.detail = True
        self._functions: list[Function] | None = None
        self._calls: dict[int, set[int]] | None = None

    # --- addresses -------------------------------------------------------
    def in_text(self, va: int) -> bool:
        return self.va0 <= va < self.end

    def section_of(self, va: int) -> str | None:
        rva = va - self.base
        for s in self.pe.sections:
            span = max(s.Misc_VirtualSize, s.SizeOfRawData)
            if s.VirtualAddress <= rva < s.VirtualAddress + span:
                return s.Name.rstrip(b"\0").decode("latin-1", "replace")
        return None

    def masked(self) -> bytes:
        """`.text` with every relocated dword blanked.

        The same library function linked into two DLLs differs only in the
        addresses the loader fixes up; blanking them is what lets the two
        compare equal.  Unmasked, cross-module matching finds 2% of the code
        where the truth is 8%.
        """
        out = bytearray(self.data)
        self.pe.parse_data_directories(directories=[
            pefile.DIRECTORY_ENTRY["IMAGE_DIRECTORY_ENTRY_BASERELOC"]])
        for block in getattr(self.pe, "DIRECTORY_ENTRY_BASERELOC", []):
            for r in block.entries:
                if r.type != 3:
                    continue
                off = r.rva - self.text.VirtualAddress
                if 0 <= off < len(out):
                    out[off:off + 4] = b"\0\0\0\0"
        return bytes(out)

    # --- code ------------------------------------------------------------
    def sweep(self):
        """Every instruction, restarting a byte later where capstone stalls."""
        off = 0
        while off < len(self.data):
            moved = False
            for ins in self.md.disasm(self.data[off:], self.va0 + off):
                moved = True
                off = ins.address - self.va0 + ins.size
                yield ins
            if not moved:
                off += 1

    def functions(self) -> list[Function]:
        if self._functions is not None:
            return self._functions

        targets: dict[int, int] = defaultdict(int)     # call target -> callers
        strings: dict[int, list[str]] = defaultdict(list)
        boundary: set[int] = set()
        ends: dict[int, str] = {}
        edges: dict[int, set[int]] = defaultdict(set)
        for ins in self.sweep():
            boundary.add(ins.address)
            ends[ins.address + ins.size] = ins.mnemonic
            if ins.mnemonic == "call" and ins.operands[0].type == X.X86_OP_IMM:
                t = ins.operands[0].imm
                if self.in_text(t):
                    targets[t] += 1
                    edges[ins.address].add(t)
            for op in ins.operands:
                if op.type == X.X86_OP_IMM and not self.in_text(op.imm):
                    text = self.cstring(op.imm)
                    if text:
                        strings[ins.address].append(text)

        starts = set(targets) | {self.va0}
        for m in PADDING.finditer(self.data):
            if m.end() < len(self.data):
                starts.add(self.va0 + m.end())
        self.pe.parse_data_directories(directories=[
            pefile.DIRECTORY_ENTRY["IMAGE_DIRECTORY_ENTRY_EXPORT"]])
        exported = getattr(self.pe, "DIRECTORY_ENTRY_EXPORT", None)
        if exported:
            starts.update(self.base + e.address for e in exported.symbols)
        starts |= self._pointed_at(starts, ends)
        starts = {s for s in starts if self.in_text(s) and s in boundary}

        masked = self.masked()
        ordered = sorted(starts)
        out: list[Function] = []
        for i, s in enumerate(ordered):
            e = ordered[i + 1] if i + 1 < len(ordered) else self.end
            body = masked[s - self.va0:e - self.va0].rstrip(b"\xcc\x90")
            if not body:
                continue
            fn = Function(s, len(body),
                          hashlib.sha1(body).hexdigest(), targets.get(s, 0))
            for at, found in strings.items():
                if s <= at < s + len(body):
                    fn.strings.extend(found)
            out.append(fn)
        self._functions = out
        self._calls = edges
        return out

    def _pointed_at(self, starts: set[int], ends: dict[int, str]) -> set[int]:
        """Addresses a data section names -- vtables, and handler tables.

        A dword that merely looks like an address would cut a real function in
        two and invent a second one, so one is only believed where a function
        could begin: after padding, after the `ret` or `jmp` that ended the
        last one, or somewhere already known to be an entry.  Requiring
        padding alone is not enough -- `MisLoad.dll`'s ten record getters are
        packed with none between them, reached only through a vtable, and a
        scan that wants padding sees two of the ten.
        """
        out: set[int] = set()
        for s in self.pe.sections:
            if b".text" in s.Name:
                continue
            blob = s.get_data()
            points = [self.in_text(int.from_bytes(blob[o:o + 4], "little"))
                      for o in range(0, len(blob) - 4, 4)]
            for i, ok in enumerate(points):
                if not ok:
                    continue
                # A vtable is a run of pointers; a dword that happens to hold a
                # code address is on its own.  Without this the relaxation
                # below turns every `ret` a table points past into a function
                # and iron3d.dll grows from 4000 of them to 10000.
                if not (i and points[i - 1]) and not (i + 1 < len(points)
                                                      and points[i + 1]):
                    continue
                va = int.from_bytes(blob[i * 4:i * 4 + 4], "little")
                if va in out:
                    continue
                before = self.data[va - self.va0 - 1:va - self.va0]
                if va in starts or before in (b"\xcc", b"\x90") \
                        or ends.get(va) in ("ret", "jmp"):
                    out.add(va)
        return out

    def calls(self) -> dict[int, set[int]]:
        self.functions()
        return self._calls or {}

    def cstring(self, va: int, limit: int = 64) -> str | None:
        rva = va - self.base
        for s in self.pe.sections:
            if not s.VirtualAddress <= rva < s.VirtualAddress + s.SizeOfRawData:
                continue
            off = s.PointerToRawData + (rva - s.VirtualAddress)
            m = PRINTABLE.match(self.raw, off, off + limit)
            return m.group()[:-1].decode("latin-1") if m else None
        return None

    def function_at(self, va: int) -> Function | None:
        for fn in self.functions():
            if fn.at <= va < fn.end:
                return fn
        return None


def ledger() -> list[Entry]:
    if not LEDGER.exists():
        return []
    raw = tomllib.loads(LEDGER.read_text())
    out = []
    for row in raw.get("entry", []):
        out.append(Entry(row["module"], int(str(row["at"]), 16),
                         row.get("kind", "function"), row["name"],
                         row.get("doc", "")))
    return out


def load(names=MODULES) -> dict[str, Module]:
    return {n: Module(GAME / n) for n in names}


def library(mods: dict[str, Module]) -> set[str]:
    """Bodies that appear in more than one module -- the statically linked CRT.

    Two modules is enough to say so: the engine's own modules are separate
    codebases and share no function byte for byte, so a body in two of them
    came out of a library.  It catches about half of a small module.

    It does not catch all of it, and the reason is worth knowing -- the copies
    are compiled separately, so inlining can make two instances of the same
    CRT function differ.  `MisLoad.dll`'s float formatter is unique to it and
    still sorts to the top of any ranking by strings, on `1#INF` and `1#QNAN`.
    Read the strings, not just the rank.
    """
    where: dict[str, set[str]] = defaultdict(set)
    for name, mod in mods.items():
        for fn in mod.functions():
            where[fn.group].add(name)
    return {g for g, v in where.items() if len(v) >= 2}


# --- reports -------------------------------------------------------------
def report(mods: dict[str, Module], entries: list[Entry]) -> None:
    known: dict[str, set[int]] = defaultdict(set)
    for e in entries:
        if e.kind in ("function", "site"):
            known[e.module].add(e.at)

    groups: dict[str, list[tuple[str, Function]]] = defaultdict(list)
    for name, mod in mods.items():
        for fn in mod.functions():
            groups[fn.group].append((name, fn))

    read_groups = set()
    for name, mod in mods.items():
        for at in known[name]:
            fn = mod.function_at(at)
            if fn:
                read_groups.add(fn.group)

    print(f"{'module':16} {'funcs':>6} {'bytes':>9} {'read':>5} {'bytes':>8} {'%':>5}")
    tf = tb = rf = rb = 0
    for name, mod in mods.items():
        fns = mod.functions()
        done = [f for f in fns if f.group in read_groups]
        b = sum(f.size for f in fns)
        db = sum(f.size for f in done)
        print(f"{name:16} {len(fns):6} {b:9} {len(done):5} {db:8} "
              f"{100 * db / b if b else 0:4.1f}%")
        tf += len(fns)
        tb += b
        rf += len(done)
        rb += db
    print(f"{'TOTAL':16} {tf:6} {tb:9} {rf:5} {rb:8} "
          f"{100 * rb / tb if tb else 0:4.1f}%")
    shared = sum(1 for g, v in groups.items() if len({n for n, _ in v}) > 1)
    print(f"\n{len(groups)} distinct bodies, {tf - len(groups)} duplicates; "
          f"{shared} of them appear in more than one module and are library "
          f"code.")


def check(mods: dict[str, Module], entries: list[Entry]) -> int:
    """Every published address, against the binary it claims."""
    bad = 0
    for e in sorted(entries, key=lambda x: (x.module, x.at)):
        mod = mods.get(e.module)
        if mod is None:
            print(f"  FAIL {e.module}:{e.at:#x}  no such module")
            bad += 1
            continue
        where = mod.section_of(e.at)
        if e.kind in ("datum", "table"):
            if where is None:
                print(f"  FAIL {e.module}:{e.at:#x}  in no section  ({e.name})")
                bad += 1
            elif (where == ".text") != (e.kind == "table"):
                want = "inside .text" if e.kind == "table" else "outside .text"
                print(f"  FAIL {e.module}:{e.at:#x}  {e.kind} should be {want}, "
                      f"found in {where}  ({e.name})")
                bad += 1
            continue
        if not mod.in_text(e.at):
            print(f"  FAIL {e.module}:{e.at:#x}  not in .text "
                  f"(section {where})  ({e.name})")
            bad += 1
            continue
        fn = mod.function_at(e.at)
        if fn is None:
            print(f"  FAIL {e.module}:{e.at:#x}  no instruction there  ({e.name})")
            bad += 1
        elif e.kind == "function" and fn.at != e.at:
            print(f"  FAIL {e.module}:{e.at:#x}  mid-function, starts at "
                  f"{fn.at:#x}  ({e.name})")
            bad += 1
    print(f"\n{len(entries) - bad}/{len(entries)} ledger entries check out")
    return bad


def unknown(mod: Module, entries: list[Entry], limit: int,
            lib: set[str]) -> None:
    """The unread functions of one module, worth-reading first.

    Ranked by strings and then by size, because a function that names a file,
    a tag, a class or an error message is handling the game's own data.
    Library bodies are dropped rather than demoted: they are somebody else's
    code and reading them tells us nothing about the formats.
    """
    known = {e.at for e in entries if e.module == mod.name}
    read = {f.group for f in (mod.function_at(a) for a in known) if f}
    all_fns = mod.functions()
    rest = [f for f in all_fns if f.group not in read and f.group not in lib]
    rest.sort(key=lambda f: (-len(set(f.strings)), -f.size))
    print(f"{mod.name}: {len(rest)} unread of {len(all_fns)}, "
          f"{sum(1 for f in all_fns if f.group in lib)} of them library\n")
    print(f"{'address':10} {'size':>6} {'callers':>7}  strings")
    for fn in rest[:limit]:
        sample = ", ".join(sorted(set(fn.strings))[:3])
        print(f"{fn.at:#010x} {fn.size:6} {fn.callers:7}  {sample[:78]}")


def closure(mod: Module, at: int, entries: list[Entry],
            lib: set[str]) -> None:
    """What one entry point reaches, and how much of it has been read."""
    named = {e.at: e.name for e in entries if e.module == mod.name}
    edges = mod.calls()
    out: dict[int, int] = {}
    frontier = [(at, 0)]
    while frontier:
        va, depth = frontier.pop()
        fn = mod.function_at(va)
        if fn is None or fn.at in out:
            continue
        out[fn.at] = depth
        for site, targets in edges.items():
            if fn.at <= site < fn.end:
                for t in targets:
                    frontier.append((t, depth + 1))
    mine = [f for f in mod.functions() if f.at in out and f.group not in lib]
    done = sum(1 for f in mine if f.at in named)
    print(f"{mod.name}:{at:#x} reaches {len(out)} functions, "
          f"{len(out) - len(mine)} of them library; of the other {len(mine)} "
          f"({sum(f.size for f in mine)} bytes), {done} named")
    for a, depth in sorted(out.items(), key=lambda kv: (kv[1], kv[0])):
        fn = mod.function_at(a)
        mark = named.get(a, "[lib]" if fn.group in lib else "")
        print(f"  {'  ' * min(depth, 6)}{a:#010x} {fn.size:6}  {mark}")


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--check", action="store_true",
                    help="validate every ledger entry against its binary")
    ap.add_argument("--unknown", metavar="MODULE",
                    help="list a module's unread functions, best first")
    ap.add_argument("--closure", metavar="MODULE:ADDR",
                    help="what one entry point reaches")
    ap.add_argument("-n", type=int, default=40, help="how many rows")
    args = ap.parse_args(argv)

    entries = ledger()
    mods = load()
    if args.check:
        return 1 if check(mods, entries) else 0
    if args.closure:
        name, _, at = args.closure.partition(":")
        closure(mods[name], int(at, 16), entries, library(mods))
        return 0
    if args.unknown:
        unknown(mods[args.unknown], entries, args.n, library(mods))
        return 0
    report(mods, entries)
    return 0


if __name__ == "__main__":
    sys.exit(main())
