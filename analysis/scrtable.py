"""The behaviour interpreter's function table, out of ai.dll, against the scripts.

A `.scr` call node carries a function id in `head[0]`, and `ai.dll`'s node
executor calls `table[id]` with no arguments of its own
(`ai.dll:0x100122c9`).  The table is written by code, not the linker: the
SuperAI constructor allocates `count * 4` bytes, where `count` is the script's
first word, and stores one code address per slot starting at `0x1000128a`.
This finds that run, cuts each handler at the next function start, and reads
**which operands each handler fetches** -- the handlers are unoptimised, so an
argument is always

    mov  r, [a + b + 0x14]      ; the current node's operand pointer
    mov  r', [r + 4k]           ; operand k, a varset index

-- and then checks the count against what the 58 shipped scripts pass.  If the
id were not the slot, the counts would not line up.

    uv run --group analysis python analysis/scrtable.py

Recovers facts only; see ../docs/09-method.md.  Nothing here is imported by
the library.
"""

from __future__ import annotations

import sys
from collections import defaultdict
from pathlib import Path

import capstone

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import coverage  # noqa: E402

from openparkan import behaviour  # noqa: E402

X = capstone.x86

#: Where the constructor starts writing the table, and how far to look.
FIRST_STORE = 0x1000128A
SPAN = 0x300

#: The operand pointer is loaded from this displacement off the current node.
OPERANDS = 0x14

#: The slot the executor writes a call's result from, on the SuperAI.
RESULT = 0x50


def table(mod: coverage.Module) -> dict[int, int]:
    """Slot -> handler address, from the run of stores the constructor makes."""
    out: dict[int, int] = {}
    base = None
    code = mod.data[FIRST_STORE - mod.va0:FIRST_STORE - mod.va0 + SPAN]
    for ins in mod.md.disasm(code, FIRST_STORE):
        ops = ins.operands
        if ins.mnemonic != "mov" or len(ops) != 2:
            continue
        dst, src = ops
        if dst.type != X.X86_OP_MEM or src.type != X.X86_OP_IMM:
            continue
        if not mod.in_text(src.imm) or dst.mem.index or not dst.mem.base:
            continue
        if base is None:
            base = dst.mem.base
        if dst.mem.base == base and dst.mem.disp % 4 == 0:
            out[dst.mem.disp // 4] = src.imm
    return out


def handler(mod: coverage.Module, at: int, end: int) -> tuple[int, bool]:
    """How many operands a handler reads, and whether it writes the result slot."""
    pointer: set[int] = set()
    top = -1
    writes = False
    for ins in mod.md.disasm(mod.data[at - mod.va0:end - mod.va0], at):
        ops = ins.operands
        if ins.mnemonic == "mov" and len(ops) == 2:
            dst, src = ops
            if dst.type == X.X86_OP_MEM and dst.mem.disp == RESULT and dst.mem.base \
                    and not dst.mem.index:
                writes = True
            if dst.type == X.X86_OP_REG and src.type == X.X86_OP_MEM:
                m = src.mem
                if m.base and m.index and m.scale == 1 and m.disp == OPERANDS:
                    pointer = {dst.reg}
                    continue
                if m.base in pointer and not m.index and m.disp >= 0 and m.disp % 4 == 0:
                    top = max(top, m.disp // 4)
        pointer = set() if ins.mnemonic not in ("mov",) else pointer
    return top + 1, writes


def corpus(game: Path) -> dict[int, tuple[set[int], set[bool], int]]:
    """Function id -> (argument counts, writes-a-destination, uses)."""
    out: dict[int, tuple[set[int], set[bool], int]] = {}
    seen: dict[int, list] = defaultdict(lambda: [set(), set(), 0])
    for path in behaviour.scripts(game):
        for h in behaviour.read(path).handlers:
            for n in h.nodes:
                if n.calls:
                    row = seen[n.function]
                    row[0].add(len(n.operands))
                    row[1].add(n.destination != behaviour.NULL)
                    row[2] += 1
    for k, (a, w, u) in seen.items():
        out[k] = (a, w, u)
    return out


def main() -> int:
    mod = coverage.Module(coverage.GAME / "ai.dll")
    slots = table(mod)
    starts = sorted(f.at for f in mod.functions())
    used = corpus(coverage.GAME)
    print(f"{len(slots)} stores, slots {min(slots)}..{max(slots)}, "
          f"{len(set(slots.values()))} distinct handlers; "
          f"{len(used)} ids used by the scripts, {min(used)}..{max(used)}\n")
    print(f"{'id':>3} {'handler':>10} {'reads':>5} {'writes':>6}   "
          f"{'scripts pass':>12} {'dest':>5} {'uses':>4}")
    exact = 0
    written = quiet = silent = 0
    arities: list[int] = []
    void: list[int] = []
    for slot in sorted(slots):
        at = slots[slot]
        end = next((s for s in starts if s > at), mod.end)
        reads, writes = handler(mod, at, end)
        arities.append(reads)
        if not writes:
            void.append(slot)
        if slot in used:
            counts, dests, uses = used[slot]
            exact += reads == max(counts)
            if True in dests:
                written += 1
                quiet += writes
            if not writes:
                silent += dests == {False}
            passes = "/".join(str(c) for c in sorted(counts))
            dest = "/".join("y" if d else "n" for d in sorted(dests))
            mark = "" if reads == max(counts) else "  <-"
            print(f"{slot:3} {at:#010x} {reads:5} {'y' if writes else 'n':>6}   "
                  f"{passes:>12} {dest:>5} {uses:4}{mark}")
        else:
            print(f"{slot:3} {at:#010x} {reads:5} {'y' if writes else 'n':>6}   "
                  f"{'unused':>12}")
    print(f"\n{exact} of {len(used)} used ids: the handler reads exactly as many "
          f"operands as the scripts' longest call passes")
    never = sum(1 for s in used if s in void)
    print(f"{quiet} of {written} ids whose calls write a destination have a handler "
          f"that writes the result slot, and {silent} of the {never} used handlers "
          f"that never write it are never given a destination")
    library = (tuple(arities) == behaviour.ARGUMENTS
               and tuple(void) == behaviour.VOID_FUNCTIONS)
    print(f"openparkan.behaviour.ARGUMENTS and VOID_FUNCTIONS "
          f"{'match' if library else 'DO NOT match'} the binary")
    ok = (len(slots) == behaviour.FUNCTION_TABLE and exact >= len(used) - 2
          and quiet == written and library)
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
