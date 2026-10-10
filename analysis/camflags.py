"""Who hands a camera's flag setter a constant that carries one bit.

`ICamera`'s slot 13 (`Terrain.dll:0x10084a70`) stores its one argument in the camera's flag
word, `+0x164`, and slot 20 (`0x10084ad0`) reads it back.  docs/10 had two of the word's bits
open: 8, which stops a camera drawing the sky's pass and the world, and `0x200`, which keeps
the weather's pass from it.  A caller sets a bit one of three ways -- it pushes a constant,
it ORs one into what slot 20 answered, or it loads one into a register -- so this lists every
call through `+0x34` in the install with such a constant since the call before it, and how
many values the call pushes.  The setter takes its receiver and one value: a call that pushes
three or four is another interface's slot 13.

    uv run --group analysis python analysis/camflags.py          # bit 8
    uv run --group analysis python analysis/camflags.py 0x200    # the control

The control is the bit whose writer is published: `CBuilding::Render` ORs `0x200` into the
view's flags at `Terrain.dll:0x100599f9`.  An `or ah, 2` is that bit, so a byte register's
high half is shifted before it is tested.

What this cannot see is a value that is neither a constant nor a mask: the mirror's camera is
handed the scene camera's whole word (`Terrain.dll:0x10083f0e`-`0x10083f20`), which the
listing of two-push calls after a slot-20 call shows and the constant search does not.

See ../docs/09-method.md.  This recovers which calls the engine makes; no code is copied out
of it.
"""

from __future__ import annotations

import re
import sys
from collections import Counter

import slotcalls

SLOT = 13
CONSTANT = re.compile(
    r"(push|or \w+,|or (?:dword|byte|word) ptr \[[^\]]+\],|mov e..,) (0x[0-9a-f]+|\d+)$")


def main(argv: list[str]) -> int:
    bit = int(argv[1], 0) if len(argv) > 1 else 8
    call = re.compile(r"call dword ptr \[e.. \+ " + f"{4 * SLOT:#x}" + r"\]$")
    total, hits, arity = 0, [], Counter()
    for name, module in slotcalls.cov.load().items():
        for at, lines in slotcalls.listing(module):
            for i, line in enumerate(lines):
                if not call.search(line):
                    continue
                total += 1
                back = []
                for earlier in reversed(lines[:i]):
                    body = earlier.split("  ", 1)[1]
                    if body.startswith(("call ", "ret")) or len(back) > 24:
                        break
                    back.append(earlier)
                found = []
                for earlier in reversed(back):
                    body = earlier.split("  ", 1)[1]
                    m = CONSTANT.match(body)
                    if not m:
                        continue
                    value = int(m.group(2), 0)
                    if re.match(r"or [abcd]h,", body):
                        value <<= 8
                    if value & bit and value < 0x10000:
                        found.append(earlier)
                if not found:
                    continue
                pushes = sum(e.split("  ", 1)[1].startswith("push ") for e in back)
                arity[pushes] += 1
                hits.append(f"  {name:13s} {line}   in {at:#x}, {pushes} pushed: "
                            + "; ".join(found))
    print(f"{total} calls at +{4 * SLOT:#x} in all, {len(hits)} with a constant carrying "
          f"{bit:#x} since the call before:")
    print("\n".join(hits))
    print("by the values pushed: "
          + ", ".join(f"{n} push {k}" for k, n in sorted(arity.items())))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
