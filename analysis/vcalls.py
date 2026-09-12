"""Every call made through one C++ object's vtable, across the whole install.

Two entries in `../TODO.md` are negatives -- nobody asks a material for its
second animation track, nobody reads its class byte -- and a negative is only
worth publishing if the search that produced it can be shown to find the
things that *are* there.  This is that search, and it runs in two passes.

The first follows the pointer the way the compiler actually moves it, which is
the part every earlier attempt got wrong:

* into **object fields** -- `Terrain.dll` keeps the manager in three of them
  and `AniMesh.dll` in one, so a scan that assumes a single stored global sees
  a third of the call sites;
* through **stack locals**, because a pointer loaded once is routinely parked
  in a local and re-read at the call;
* and across **call boundaries**, because `Terrain.dll` hands the manager to a
  five-way factory which hands it on to five constructors.  A scan that stops
  at the module's own frame stops one call short of most of the uses.

Note that `ebp` is not always a frame pointer: `AniMesh.dll` uses it as the
object pointer, so `[ebp + 0x14c]` there is a field and not a local.  Reading
it as a local loses a real call site silently.

That pass has one gap it cannot close on its own.  A constructor stores the
manager into its own object, and following *that* field means tainting every
object with a field at the same offset -- which taints the whole module and
answers nothing.  So the second pass drops the taint entirely and enumerates
**every** indirect call at the slot's offset in the modules that can hold the
object, reporting where each receiver came from and how many arguments the
call passes.  A slot's arity is in its own `ret n`, and a call that passes a
different number is not a call to that slot.

    uv run --group analysis python analysis/vcalls.py

The controls are in the output: the same first pass must find the call sites
already published for slots 3 and 6, and it does.

See ../docs/09-method.md.  This recovers which calls the engine makes; no code
is copied out of it.
"""

from __future__ import annotations

import sys
from collections import defaultdict
from pathlib import Path

import capstone
import pefile

X = capstone.x86

GAME = Path("/Users/dragonee/Kod/parkan/Parkan Iron Strategy")

#: The object this asks about, and where its class is implemented.
FACTORY = "LoadMatManager"
OWNER = "World3D.dll"
VTABLE = 0x100209e4
SLOTS = 11

#: Slots the write-ups call a negative, and so must be enumerated exhaustively.
CLAIMED_EMPTY = (5, 9)

#: Registers a call may destroy.
VOLATILE = ("eax", "ecx", "edx")

#: How far back a push still counts as an argument to the next call.
ARGS = 12

#: How far back the receiver of an indirect call is looked for.
BACK = 12

#: Instructions that leave a pending argument list alone.
QUIET = frozenset((
    "mov", "lea", "xor", "test", "cmp", "and", "or", "add", "sub", "sar",
    "shr", "shl", "inc", "dec", "movzx", "movsx", "nop", "fld", "fstp"))


class Image:
    """A PE file, decoded with a sweep that restarts where capstone stalls."""

    def __init__(self, path: Path):
        self.path = path
        self.name = path.name
        self.raw = path.read_bytes()
        self.pe = pefile.PE(str(path))
        self.base = self.pe.OPTIONAL_HEADER.ImageBase
        self.text = next(s for s in self.pe.sections if b".text" in s.Name)
        self.va0 = self.base + self.text.VirtualAddress
        self.data = self.text.get_data()
        self.md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
        self.md.detail = True
        self._code: list | None = None

    def code(self) -> list:
        """Every instruction in `.text`, in address order.

        A linear sweep desynchronises on jump tables and inline data and then
        stops; restarting a byte later is what keeps the rest visible.
        """
        if self._code is None:
            out, off = [], 0
            while off < len(self.data):
                moved = False
                for ins in self.md.disasm(self.data[off:], self.va0 + off):
                    moved = True
                    off = ins.address - self.va0 + ins.size
                    out.append(ins)
                if not moved:
                    off += 1
            self._code = out
        return self._code

    def offset(self, va: int) -> int | None:
        """The file offset of a virtual address, or None where it has no bytes."""
        rva = va - self.base
        for s in self.pe.sections:
            span = max(s.Misc_VirtualSize, s.SizeOfRawData)
            if s.VirtualAddress <= rva < s.VirtualAddress + span:
                if rva - s.VirtualAddress >= s.SizeOfRawData:
                    return None
                return s.PointerToRawData + (rva - s.VirtualAddress)
        return None

    def dword(self, va: int) -> int:
        off = self.offset(va)
        return 0 if off is None else int.from_bytes(self.raw[off:off + 4], "little")

    def imports(self, want: str) -> int | None:
        """The IAT slot an imported name is called through."""
        self.pe.parse_data_directories(directories=[
            pefile.DIRECTORY_ENTRY["IMAGE_DIRECTORY_ENTRY_IMPORT"]])
        for d in getattr(self.pe, "DIRECTORY_ENTRY_IMPORT", []):
            for imp in d.imports:
                if imp.name and imp.name.decode() == want:
                    return imp.address
        return None

    def arity(self, va: int) -> int | None:
        """Stack arguments, from the function's own ``ret n``.

        Every `ret` in a stdcall function pops the same amount, so the first
        one at or after the entry answers it -- guessing a function length
        instead reports `?` for the long ones.
        """
        for ins in self.code():
            if ins.address >= va and ins.mnemonic == "ret":
                return (int(ins.op_str, 0) if ins.op_str else 0) // 4
        return None


def _base(ins, op) -> str | None:
    return ins.reg_name(op.mem.base) if op.mem.base else None


def follow(img: Image, fields: set[int], args: dict[int, set[int]],
           thiscall: set[int]):
    """Follow the pointer through one module, one pass.

    ``fields`` are object offsets known to hold it, ``args`` maps a function to
    the ``[ebp + n]`` slots that receive it, ``thiscall`` the functions that
    receive it in ``ecx``.  Returns the virtual calls made on it, where it was
    stored, and what it was passed to.
    """
    calls: list[tuple[int, int]] = []
    stores: list[tuple[int, int]] = []
    passed: dict[int, set[int]] = defaultdict(set)
    ecx_args: set[int] = set()

    tag: dict[str, str] = {}
    local: dict[int, str] = {}
    pushes: list[str | None] = []

    for ins in img.code():
        ops = ins.operands
        m = ins.mnemonic

        if m == "ret":
            tag.clear()
            local.clear()
            pushes.clear()
            continue

        if ins.address in args:
            for n in args[ins.address]:
                local[n] = "mgr"
        if ins.address in thiscall:
            tag["ecx"] = "mgr"

        if m == "push":
            op = ops[0]
            pushes.append(tag.get(ins.reg_name(op.reg))
                          if op.type == X.X86_OP_REG else None)
            del pushes[:-ARGS]
            continue

        if m == "call":
            op = ops[0]
            if op.type == X.X86_OP_MEM and op.mem.index == 0 and op.mem.base:
                if tag.get(_base(ins, op)) == "vt":
                    calls.append((ins.address, op.mem.disp // 4))
            elif op.type == X.X86_OP_IMM:
                # The last push is the callee's [ebp + 8].
                for n, t in enumerate(reversed(pushes)):
                    if t == "mgr":
                        passed[op.imm].add(8 + n * 4)
                if tag.get("ecx") == "mgr":
                    ecx_args.add(op.imm)
            pushes.clear()
            for r in VOLATILE:
                tag.pop(r, None)
            continue

        if m == "mov" and len(ops) == 2:
            dst, src = ops
            if dst.type == X.X86_OP_REG:
                d = ins.reg_name(dst.reg)
                new = None
                if src.type == X.X86_OP_REG:
                    new = tag.get(ins.reg_name(src.reg))
                elif src.type == X.X86_OP_MEM and src.mem.index == 0:
                    b = _base(ins, src)
                    if b and tag.get(b) == "mgr" and src.mem.disp == 0:
                        new = "vt"
                    # The field test comes before the local test: `ebp` is the
                    # object pointer in AniMesh, not a frame pointer.
                    elif b and src.mem.disp in fields and tag.get(b) != "vt":
                        new = "mgr"
                    elif b == "ebp":
                        new = local.get(src.mem.disp)
                if new:
                    tag[d] = new
                else:
                    tag.pop(d, None)
                continue
            if dst.type == X.X86_OP_MEM and dst.mem.index == 0 \
                    and src.type == X.X86_OP_REG:
                t = tag.get(ins.reg_name(src.reg))
                b = _base(ins, dst)
                if b == "ebp" and dst.mem.disp < 0:
                    if t:
                        local[dst.mem.disp] = t
                    else:
                        local.pop(dst.mem.disp, None)
                elif t == "mgr" and b:
                    stores.append((ins.address, dst.mem.disp))
            continue

        for op in ops:
            if op.type == X.X86_OP_REG and op.access & capstone.CS_AC_WRITE:
                tag.pop(ins.reg_name(op.reg), None)

    return calls, stores, passed, ecx_args


def seeds(img: Image) -> set[int]:
    """The object fields a module stores the factory's result in."""
    slot = img.imports(FACTORY)
    if slot is None:
        return set()
    thunk = next((i.address for i in img.code() if i.mnemonic == "jmp"
                  and i.operands[0].type == X.X86_OP_MEM
                  and i.operands[0].mem.base == 0
                  and i.operands[0].mem.disp == slot), None)
    after = {i.address + i.size for i in img.code()
             if i.mnemonic == "call" and i.operands[0].type == X.X86_OP_IMM
             and i.operands[0].imm in (thunk, slot)}
    out: set[int] = set()
    live = False
    for ins in img.code():
        if ins.address in after:
            live = True
        elif live and ins.mnemonic == "mov" \
                and ins.operands[0].type == X.X86_OP_MEM \
                and ins.operands[1].type == X.X86_OP_REG \
                and ins.reg_name(ins.operands[1].reg) == "eax":
            out.add(ins.operands[0].mem.disp)
            live = False
        elif live and ins.mnemonic not in QUIET and ins.mnemonic != "push":
            live = False
    return out


def every_call(img: Image, disp: int):
    """Every indirect call at ``disp``, with its receiver and argument count.

    No taint: this is the exhaustive pass, so nothing about the receiver is
    assumed.  Arguments are the pushes since the last call or branch.
    """
    code = img.code()
    rows = []
    run = 0
    frame = False
    for n, ins in enumerate(code):
        # The prologue's own pushes are not arguments: `push ebp; mov ebp, esp`
        # sets the frame up and the `push ecx` after it allocates a local.
        # Counting them turned a three-argument thiscall into a five-argument
        # one, which is exactly the slot being asked about.
        if ins.mnemonic == "mov" and ins.op_str == "ebp, esp":
            run, frame = 0, True
            continue
        if ins.mnemonic == "push":
            if not frame:
                run += 1
            frame = False
            continue
        frame = False
        if ins.mnemonic == "call":
            op = ins.operands[0]
            if op.type == X.X86_OP_MEM and op.mem.index == 0 and op.mem.base \
                    and op.mem.disp == disp:
                field = receiver(img, code, n)
                rows.append((ins.address, field, run,
                             widest(img, code, n, field) if field is not None else 0))
            run = 0
            continue
        if ins.mnemonic not in QUIET:
            run = 0
    return rows


def bounds(code: list, n: int) -> tuple[int, int]:
    """The enclosing function, as the nearest `push ebp; mov ebp, esp` around."""
    def prologue(k: int) -> bool:
        return code[k].mnemonic == "push" and code[k].op_str == "ebp" \
            and k + 1 < len(code) and code[k + 1].mnemonic == "mov" \
            and code[k + 1].op_str == "ebp, esp"
    lo = next((k for k in range(n, -1, -1) if prologue(k)), 0)
    hi = next((k for k in range(n + 1, len(code)) if prologue(k)), len(code))
    return lo, hi


def widest(img: Image, code: list, n: int, field: int) -> int:
    """The largest vtable offset called on the same field in this function.

    An object is not the one we are looking for if the code calls a slot it
    does not have.  This is what separates a receiver that merely sits at the
    same offset from the real thing, and it is sharper than the argument count:
    `Terrain.dll` calls `+0x84` on the object at `[arg1 + 0xc]`, so that object
    has at least 34 slots against this vtable's eleven.
    """
    lo, hi = bounds(code, n)
    out = 0
    for k in range(lo, hi):
        ins = code[k]
        if ins.mnemonic != "call":
            continue
        op = ins.operands[0]
        if op.type != X.X86_OP_MEM or not op.mem.base or op.mem.index:
            continue
        if receiver(img, code, k) == field:
            out = max(out, op.mem.disp)
    return out


def receiver(img: Image, code: list, n: int) -> int | None:
    """The field offset the call's object was loaded from, if it was one."""
    call = code[n]
    vt = call.reg_name(call.operands[0].mem.base)
    for prev in reversed(code[max(0, n - BACK):n]):
        if prev.mnemonic == "mov" and prev.operands[0].type == X.X86_OP_REG \
                and prev.reg_name(prev.operands[0].reg) == vt \
                and prev.operands[1].type == X.X86_OP_MEM \
                and prev.operands[1].mem.disp == 0 and prev.operands[1].mem.base:
            obj = prev.reg_name(prev.operands[1].mem.base)
            for p2 in reversed(code[max(0, n - BACK - BACK):code.index(prev)]):
                if p2.mnemonic == "mov" \
                        and p2.operands[0].type == X.X86_OP_REG \
                        and p2.reg_name(p2.operands[0].reg) == obj:
                    src = p2.operands[1]
                    # `[ebp + n]` is this function's own argument, not a field
                    # of the object -- counting it as one invents receivers.
                    if src.type == X.X86_OP_MEM and src.mem.base \
                            and p2.reg_name(src.mem.base) not in ("ebp", "esp"):
                        return src.mem.disp
                    return None
            return None
    return None


def main() -> int:
    owner = Image(GAME / OWNER)
    entries = [owner.dword(VTABLE + n * 4) for n in range(SLOTS)]
    print(f"{FACTORY} -> {OWNER} vtable {VTABLE:08x}, {SLOTS} slots\n")

    callers: dict[int, list[str]] = defaultdict(list)
    holders: list[tuple[Image, set[int]]] = []
    for path in sorted(GAME.glob("*.dll")):
        img = Image(path)
        if img.imports(FACTORY) is None:
            continue
        fields = seeds(img)
        args: dict[int, set[int]] = {}
        this: set[int] = set()
        for _ in range(12):
            calls, stores, passed, ecx = follow(img, fields, args, this)
            grew = False
            for fn, slots in passed.items():
                if not slots <= args.get(fn, set()):
                    args.setdefault(fn, set()).update(slots)
                    grew = True
            if not ecx <= this:
                this |= ecx
                grew = True
            if not grew:
                break
        where = fields | {f for _, f in stores}
        holders.append((img, where))
        stored = ", ".join(sorted({f"+0x{f:x}" for _, f in stores})) or "nothing"
        print(f"{img.name}: held in "
              f"{', '.join(f'+0x{f:x}' for f in sorted(fields))}; "
              f"constructors store it into {stored}; "
              f"{len(args)} functions take it as an argument")
        for addr, idx in sorted(calls):
            callers[idx].append(f"{img.name}:{addr:08x}")

    print("\nslot  entry     args  callers")
    for n, va in enumerate(entries):
        take = owner.arity(va)
        who = ", ".join(callers.get(n, [])) or "--"
        print(f" {n:2}   {va:08x}   {'?' if take is None else take:>2}   {who}")

    for slot in CLAIMED_EMPTY:
        disp = slot * 4
        take = owner.arity(entries[slot])
        print(f"\nevery indirect call at +0x{disp:02x}, no taint "
              f"(slot {slot} takes {take}):")
        for img, where in holders:
            rows = every_call(img, disp)
            live = [r for r in rows if r[1] in where]
            print(f"   {img.name}: {len(rows)} calls, "
                  f"{len(live)} on an offset the manager is stored at")
            for addr, field, pushed, span in live:
                if span > (SLOTS - 1) * 4:
                    verdict = (f"receiver also takes +0x{span:x}, "
                               f"so it has more than {SLOTS} slots")
                elif pushed != take:
                    verdict = f"passes {pushed} arguments, not {take}"
                else:
                    verdict = "MATCHES -- look at it"
                print(f"      {addr:08x}  receiver +0x{field:x}  {verdict}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
