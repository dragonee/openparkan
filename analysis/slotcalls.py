"""Who calls one slot of an interface that objects hand out by `QueryInterface`.

`vcalls.py` follows a pointer from where it is stored; it cannot see one a caller asked an
object for.  This is the search for that case, and it was written for a negative that turned
out false: docs/07 published "no caller of `ILifeSystem` slot 16 has been found", and the
slot has three callers in `iron3d.dll`, each on the `ILifeSystem` its unit or building record
keeps at `+0x48`.

It works in two passes over every module, function by function:

* **every** indirect call at the slot's offset, `call [reg + 4 * slot]`, with the count per
  module -- the whole population, so a reader can see how small it is (116 sites for slot 16
  across the install);
* of those, the ones whose receiver is an `ILifeSystem`: the call sits in a function that
  asks `QueryInterface` for the interface's id (`mov edx, ID` before a `call [reg]`), or the
  receiver was loaded from a field known to hold the interface.

The fields are found, not assumed: a `lea reg, [obj + N]` pushed just before
`mov edx, ID` is where a `QueryInterface` answer is stored, and the script lists every such
`N` per module before using them.

The control is the same search asked for a slot whose callers are published: slot 10, the
owner word's setter, which `MBehaviour::Capture` calls three times (`Behavior.dll:0x10008e40`)
and a unit record's bind once (`iron3d.dll:0x10074d99`).

    uv run --group analysis python analysis/slotcalls.py            # ILifeSystem slot 16
    uv run --group analysis python analysis/slotcalls.py 0x16 10    # the control

Neither pass sees a receiver kept some other way, and the field finder wants a `lea`: an
unoptimised module makes the address with an `add`, and a method reads the field at the offset
of the interface it was entered on.  `ICamera2`'s slot 23 had "no caller" for both reasons.
What found it was the slot's own arity -- it takes its value in a register and ends in a bare
`ret` -- so `--bare` lists the calls at a slot's offset that push nothing since the call
before them:

    uv run --group analysis python analysis/slotcalls.py --bare 23  # 3 of 38, ICamera2's one
    uv run --group analysis python analysis/slotcalls.py --bare 24  # the control: 8 of 65

See ../docs/09-method.md.  This recovers which calls the engine makes; no code is copied out
of it.
"""

from __future__ import annotations

import re
import sys
from collections import defaultdict

import capstone
import coverage as cov

#: How far back a receiver's load, or a stored answer's `lea`, is looked for.
BACK = 10


def listing(module) -> list[tuple[int, list[str]]]:
    """Each function's instructions as text, restarting where the sweep stalls."""
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
    out = []
    for f in module.functions():
        data = module.data[f.at - module.va0:f.end - module.va0]
        lines, pos = [], 0
        while pos < len(data):
            moved = False
            for ins in md.disasm(data[pos:], f.at + pos):
                moved = True
                pos = ins.address - f.at + ins.size
                lines.append(f"{ins.address:#x}  {ins.mnemonic} {ins.op_str}")
            if not moved:
                pos += 1
        out.append((f.at, lines))
    return out


def asks_for(interface: int) -> str:
    """The load of an interface's id as the listing prints it.

    capstone writes an immediate below ten in decimal, with no `0x`: asked as `0x8`, the
    camera's interface matched nothing, and the search answered "no caller" for a slot that
    has dozens.
    """
    return f"mov edx, {interface:#x}" if interface > 9 else f"mov edx, {interface}"


def fields(functions, interface: int) -> set[str]:
    """Displacements a `QueryInterface` for `interface` stores its answer at."""
    ask = asks_for(interface)
    found = set()
    for _, lines in functions:
        for i, line in enumerate(lines):
            if not line.endswith(ask):
                continue
            for back in lines[max(i - 3, 0):i]:
                # A local's `[esp + N]` is not a field: the function is listed for asking.
                m = re.search(r"lea e.., \[e(?!sp).. \+ (0x[0-9a-f]+)\]$", back)
                if m:
                    found.add(m.group(1))
    return found


def bare(slot: int) -> int:
    """Every call at `slot`'s offset that pushes nothing since the call before it.

    A push ahead of a branch that the call shares is not seen: read what is listed.
    """
    call = re.compile(r"call dword ptr \[e.. \+ " + f"{4 * slot:#x}" + r"\]$")
    total, kept = 0, []
    for name, module in cov.load().items():
        for at, lines in listing(module):
            for i, line in enumerate(lines):
                if not call.search(line):
                    continue
                total += 1
                pushes = 0
                for back in reversed(lines[:i]):
                    body = back.split("  ", 1)[1]
                    if body.startswith(("call ", "ret")):
                        break
                    pushes += body.startswith("push ")
                if not pushes:
                    kept.append(f"  {name:13s} {line}   in {at:#x}")
    print(f"{total} calls at +{4 * slot:#x} in all, {len(kept)} of them with nothing pushed:")
    print("\n".join(kept))
    return 0


def main(argv: list[str]) -> int:
    if len(argv) > 2 and argv[1] == "--bare":
        return bare(int(argv[2], 0))
    interface = int(argv[1], 0) if len(argv) > 1 else 0x16
    slot = int(argv[2], 0) if len(argv) > 2 else 16
    offset = f"{4 * slot:#x}"
    call = re.compile(r"call dword ptr \[(e..) \+ " + offset + r"\]$")
    ask = asks_for(interface)
    total = 0
    hits = defaultdict(list)
    for name, module in cov.load().items():
        functions = listing(module)
        # `iron3d.dll` keeps the answer in its records through a helper (`0x1007e3c0`), whose
        # `lea edx, [esi + 0x48]` this finds.
        held = fields(functions, interface)
        count = 0
        for at, lines in functions:
            asks = any(line.endswith(ask) for line in lines)
            for i, line in enumerate(lines):
                m = call.search(line)
                if not m:
                    continue
                count += 1
                vtable = m.group(1)
                receiver = None
                for back in reversed(lines[max(i - BACK, 0):i]):
                    load = re.search(r"mov " + vtable + r", dword ptr \[(e..)\]$", back)
                    if load:
                        receiver = load.group(1)
                        break
                field = None
                if receiver:
                    for back in reversed(lines[max(i - BACK, 0):i]):
                        load = re.search(
                            r"mov " + receiver + r", dword ptr \[e.. \+ (0x[0-9a-f]+)\]$", back)
                        if load:
                            field = load.group(1)
                            break
                if field in held:
                    hits[name].append(f"  {line}   in {at:#x}, receiver from +{field}")
                elif asks:
                    hits[name].append(f"  {line}   in {at:#x}, which asks for {interface:#x}")
        total += count
        print(f"{name:14s} {count:4d} calls at +{offset}; "
              f"interface {interface:#x} is kept at {sorted(held) or 'no field'}")
        for line in hits[name]:
            print(line)
    print(f"{total} calls at +{offset} in all, {sum(len(v) for v in hits.values())} of them "
          f"on a pointer that may be interface {interface:#x}: read each")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
