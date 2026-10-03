"""Who calls one slot of an interface whose methods take `this` on the stack.

`vcalls.py` follows a stored pointer and `slotcalls.py` a `QueryInterface` answer; both
discriminate by where the receiver came from.  Some interfaces can be told by how they are
*called* instead: the effect manager's methods are `__stdcall` with the receiver as their
first stack argument, so a call of its draw -- slot 3, which ends `ret 0x10` -- is four pushes,
the receiver last, straight before `call [vtable + 0xc]`, with the vtable loaded from the
register that was pushed.  A `__thiscall` slot at the same offset on another class pushes one
argument fewer and never pushes its receiver.

    uv run --group analysis python analysis/stdcalls.py 0xc 4        # the manager's draw
    uv run --group analysis python analysis/stdcalls.py 0x14 3       # the control

For every module of the install it prints how many indirect calls there are at the offset and
which of them have the shape, with the straight-line run of instructions before each.  The
pushes are counted only since the last call or branch, so an argument set up across a branch
is missed; and a module built without optimisation loads its receiver twice, once to push and
once for the vtable, and matches nothing -- `Terrain.dll` is one, and its callers are found by
what they push (docs/11-effects.md, "Who passes the draw's pass argument").

The control is the listener's setter, slot 5 of a three-argument interface: the search finds
its one call, `World3D.dll:0x10013c34`, which an earlier pass that looked for the stored
pointer had published as absent (docs/11-effects.md, "How a sound is heard").

See ../docs/09-method.md.  This recovers which calls the engine makes; no code is copied out
of it.
"""

from __future__ import annotations

import sys

import capstone
from pe import Binary

from openparkan import gamedir

#: How many instructions of the run before a call are printed.
SHOWN = 12


def calls(binary: Binary, offset: int, arguments: int):
    """Every indirect call at `offset`, and those with the stdcall shape and their runs."""
    run: list = []
    total = 0
    shaped = []
    for ins in binary.instructions():
        if ins.mnemonic == "call":
            op = ins.operands[0]
            if (op.type == capstone.x86.X86_OP_MEM and op.mem.disp == offset
                    and op.mem.base != 0 and op.mem.index == 0):
                total += 1
                vtable = ins.reg_name(op.mem.base)
                receiver = None
                for before in reversed(run):
                    text = before.op_str
                    if (before.mnemonic == "mov" and text.startswith(vtable + ", dword ptr [")
                            and "+" not in text and "-" not in text):
                        receiver = text.split("[")[1].rstrip("]")
                        break
                pushes = [i for i in run if i.mnemonic == "push"]
                if receiver and len(pushes) == arguments and pushes[-1].op_str == receiver:
                    shaped.append((ins.address, list(run[-SHOWN:])))
            run = []
        elif ins.mnemonic.startswith("j") or ins.mnemonic.startswith("ret"):
            run = []
        else:
            run.append(ins)
    return total, shaped


def main(argv: list[str]) -> int:
    offset = int(argv[1], 0) if len(argv) > 1 else 0xC
    arguments = int(argv[2], 0) if len(argv) > 2 else 4
    game = gamedir.find()
    modules = sorted(p for p in game.iterdir() if p.suffix.lower() in (".dll", ".exe"))
    every = hits = 0
    for path in modules:
        try:
            binary = Binary(str(path))
        except Exception:  # not an x86 image with a .text this loader reads
            continue
        total, shaped = calls(binary, offset, arguments)
        every += total
        hits += len(shaped)
        print(f"{path.name:24s} {total:4d} calls at +{offset:#x}, {len(shaped)} of the shape")
        for at, run in shaped:
            print(f"  {at:#x}: " + "; ".join(f"{i.mnemonic} {i.op_str}" for i in run))
    print(f"{every} calls at +{offset:#x} in all, {hits} with {arguments} pushes ending in "
          f"the receiver: read each")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
