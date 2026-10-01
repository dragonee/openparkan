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


def fields(functions, interface: int) -> set[str]:
    """Displacements a `QueryInterface` for `interface` stores its answer at."""
    ask = f"mov edx, {interface:#x}"
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


def main(argv: list[str]) -> int:
    interface = int(argv[1], 0) if len(argv) > 1 else 0x16
    slot = int(argv[2], 0) if len(argv) > 2 else 16
    offset = f"{4 * slot:#x}"
    call = re.compile(r"call dword ptr \[(e..) \+ " + offset + r"\]$")
    ask = f"mov edx, {interface:#x}"
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
