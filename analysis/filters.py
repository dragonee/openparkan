"""Who fills a face query's filter, and with what: every block handed to a query, off the stack.

A face query -- a segment through a mesh, the push-out, the world's first-face search -- takes
an eight-dword filter whose last two words are a required and an excluded mask on a mesh
triangle's flags.  docs/07 could say what read those words (three places in `AniMesh.dll`) but
not who fills them: the round's hit test stores all eight words inline, so listing the callers of
`Control.dll`'s six-word constructor is not a list of filters, and a scan for the constants (2,
16, 4, `0x24`) separates nothing (docs/09, "Searches that do not discriminate").

This searches the other way, from the write.  It follows `esp` through every function in the
install, so that each `[esp + k]` names the same slot wherever it is written, and then for a call
site reports where each argument came from; an argument that is the address of a stack block is
printed with every store into the block's eight words -- a constant, a register and where that
register was set, or one of the small constructors' pushed arguments.

    uv run --group analysis python analysis/filters.py check      # is the tracker to be believed
    uv run --group analysis python analysis/filters.py queries    # every call at a face-query slot
    uv run --group analysis python analysis/filters.py receivers  # how each hands over its object
    uv run --group analysis python analysis/filters.py closure    # how those slots were found
    uv run --group analysis python analysis/filters.py at Control.dll 0x1001da6b
    uv run --group analysis python analysis/filters.py into AniMesh.dll:0x10008120:1
    uv run --group analysis python analysis/filters.py sink 0x18:3:2

`check` is the tracker's own control: the depth it carries must be zero at every `ret`, and it
says for how many functions it is (16048 of 17081).  A call through a pointer pops what its
callee says, which the caller does not show; the tracker takes every push since the last call
or label, less the register saves, and where a `ret` then disagrees the function is marked,
its calls are listed apart with every filter-shaped block they push, and they are read by hand.

`closure` starts from the three readers of a triangle's flags and walks outwards: each
reader's direct callers, and every call at the offset of each table slot that holds it.  A
caller handing on its own argument is followed in turn.  It follows too much -- an unrelated
interface's slot at the same offset passes arguments on too -- so what it returns is a
superset, read for the tables in `AniMesh.dll` and `Terrain.dll` that hold a reader; those
give `QUERIES`, the face-query slots.

`queries` is the enumeration: every call at one of those slots, sorted by where its filter
argument came from -- a stack block filled as a filter is (its eight words printed), a copy of
the caller's own filter, the caller's argument handed on, an address that is not on the stack,
or something else.  The controls are the round's inline filter (`Control.dll:0x1001da6b`) and
the collision pair's three (`0x1001dd24`, `0x1001de0b`, `0x1001e007`), which it must list.

What it cannot see is a filter filled somewhere other than the stack and passed as a loaded
pointer.  A static one shows as a pushed address (one does: `Terrain.dll:0x100a6038`); one
kept in an object would not, and the sweep for such a fill does not discriminate (docs/09).
Nor does it see a block filled by a constructor it does not know, and `Terrain.dll`'s, stored
through a local, are not known: `receivers` sorts the rest by how each call hands over its
object, which leaves few enough to read (docs/07, "The filters the first enumeration could
not see").

See ../docs/09-method.md.  This recovers which calls the engine makes and what data they pass;
no code is copied out of it.
"""

from __future__ import annotations

import sys
from dataclasses import dataclass, field

import capstone
import coverage as cov

X = capstone.x86

SAVED = ("ebx", "ebp", "esi", "edi")
REGS = ("eax", "ebx", "ecx", "edx", "esi", "edi", "ebp")
SUB = {"al": "eax", "ah": "eax", "ax": "eax", "bl": "ebx", "bh": "ebx", "bx": "ebx",
       "cl": "ecx", "ch": "ecx", "cx": "ecx", "dl": "edx", "dh": "edx", "dx": "edx",
       "si": "esi", "di": "edi", "bp": "ebp"}

#: The small constructors a filter is made with, by how many words they store.  Found by body:
#: `mov [eax + 4k], arg k` for k in order and nothing else.
CTORS = (4, 6, 8)


@dataclass
class Call:
    at: int
    target: str                 # "0x1000..." for a direct call, "[reg + 0x28]" otherwise
    direct: int | None
    ecx: tuple
    edx: tuple
    args: list                  # origins of the pushed arguments, first argument first
    arg_at: list                # the address of each push
    pops: int                   # bytes the tracker took the callee to pop
    known: bool                 # whether that count was read from the callee's `ret`
    base: str = ""              # the register the slot is read through, for a call through one
    receiver: tuple = ()        # where that register's table was loaded from: the object called


@dataclass
class Frame:
    """One function, with every stack operand named by its distance from the entry `esp`."""
    at: int
    calls: list = field(default_factory=list)
    stores: dict = field(default_factory=dict)      # frame address -> [(at, origin)]
    held: dict = field(default_factory=dict)        # base -> displacement -> [(at, origin)]
    ok: bool = True                                 # depth zero at every `ret`
    rets: int = 0
    lines: list = field(default_factory=list)


class Tracker:
    def __init__(self, module):
        self.m = module
        self.md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
        self.md.detail = True
        self._pop: dict[int, int | None] = {}
        self._frames: dict[int, Frame] = {}
        self.ctors = self._constructors()

    # --- what a callee pops ----------------------------------------------
    def body(self, at: int):
        f = self.m.function_at(at)
        if f is None or f.at != at:
            end = at + 0x4000
        else:
            end = f.end
        data = self.m.data[at - self.m.va0:end - self.m.va0]
        pos = 0
        while pos < len(data):
            moved = False
            for ins in self.md.disasm(data[pos:], at + pos):
                moved = True
                pos = ins.address - at + ins.size
                yield ins
            if not moved:
                pos += 1

    def callee_pop(self, at: int, depth: int = 0) -> int | None:
        if at in self._pop:
            return self._pop[at]
        self._pop[at] = None
        out = None
        for n, ins in enumerate(self.body(at)):
            if ins.mnemonic == "ret":
                out = ins.operands[0].imm if ins.operands else 0
                break
            if n == 0 and ins.mnemonic == "jmp" and ins.operands[0].type == X.X86_OP_IMM \
                    and self.m.in_text(ins.operands[0].imm) and depth < 4:
                out = self.callee_pop(ins.operands[0].imm, depth + 1)
                break
        self._pop[at] = out
        return out

    def _constructors(self) -> dict[int, int]:
        """Functions that store their k arguments into `this + 4k` and return: address -> k."""
        out = {}
        for f in self.m.functions():
            if f.size > 0x60:
                continue
            stores, pops = [], None
            fine = True
            for ins in self.body(f.at):
                if ins.mnemonic == "ret":
                    pops = ins.operands[0].imm if ins.operands else 0
                    break
                if ins.mnemonic != "mov":
                    fine = False
                    break
                a, b = ins.operands
                if a.type == X.X86_OP_MEM and ins.reg_name(a.mem.base) == "eax" \
                        and b.type == X.X86_OP_REG:
                    stores.append(a.mem.disp)
            if fine and pops and stores == [4 * k for k in range(pops // 4)] \
                    and pops // 4 in CTORS:
                out[f.at] = pops // 4
        return out

    # --- one function -----------------------------------------------------
    def frame(self, at: int) -> Frame:
        if at in self._frames:
            return self._frames[at]
        fr = Frame(at)
        ins_list = list(self.body(at))
        f = self.m.function_at(at)
        end = f.end if f else at + 0x4000
        depth = 0
        ebp_frame = (len(ins_list) > 1 and ins_list[0].mnemonic == "push"
                     and ins_list[0].op_str == "ebp" and ins_list[1].mnemonic == "mov"
                     and ins_list[1].op_str == "ebp, esp")
        reg: dict[str, tuple] = {r: ("in", r) for r in REGS}
        tables: dict[str, tuple] = {}   # a register holding a table -> what it was read out of
        written: set[str] = set()
        stack: list = []            # (depth after the push, at, origin, is a save)
        label_depth: dict[int, int] = {}
        dead = False                # the last instruction left the function
        since = 0                   # index into `stack` of the first push since a call or label

        def slot(op, ins) -> int | None:
            """The frame address of a memory operand, where it is on the stack."""
            base = ins.reg_name(op.mem.base) if op.mem.base else None
            if op.mem.index:
                return None
            if base == "esp":
                return op.mem.disp - depth
            if base == "ebp" and ebp_frame:
                return op.mem.disp - 4
            if base in reg and reg[base][0] == "local":
                return reg[base][1] + op.mem.disp
            return None

        def origin(op, ins) -> tuple:
            if op.type == X.X86_OP_IMM:
                return ("imm", op.imm & 0xffffffff)
            if op.type == X.X86_OP_REG:
                name = ins.reg_name(op.reg)
                return reg.get(name, ("reg", name))
            if op.type == X.X86_OP_MEM:
                a = slot(op, ins)
                if a is not None:
                    return ("frame", a)
                if not op.mem.base and not op.mem.index:
                    return ("abs", op.mem.disp & 0xffffffff)
                base = ins.reg_name(op.mem.base) if op.mem.base else "?"
                if base in reg and reg[base][0] in ("frame", "in"):
                    return ("field", reg[base], op.mem.disp)
                return ("mem", ins.address)
            return ("?", ins.address)

        for i, ins in enumerate(ins_list):
            if ins.address >= end:
                break
            a = ins.address
            if a in label_depth:
                if dead:
                    depth = label_depth[a]
                    stack = [s for s in stack if s[0] <= depth]
                since = len(stack)
            dead = False
            m = ins.mnemonic
            ops = ins.operands
            if m in ("jmp", "je", "jne", "jb", "jbe", "ja", "jae", "jl", "jle", "jg", "jge",
                     "js", "jns", "jp", "jnp", "jo", "jno", "jecxz", "loop") \
                    and ops and ops[0].type == X.X86_OP_IMM:
                t = ops[0].imm
                if at <= t < end and t not in label_depth:
                    label_depth[t] = depth
                if m == "jmp":
                    if not (at <= t < end):
                        fr.rets += 1         # a tail call: the callee returns for us
                    dead = True
                continue
            if m == "jmp":
                dead = True                  # a jump table: its targets were not seen
                continue
            if m == "ret":
                fr.rets += 1
                if depth != 0:
                    fr.ok = False
                dead = True
                continue
            if m == "push":
                depth += 4
                o = origin(ops[0], ins)
                save = False
                if ops[0].type == X.X86_OP_REG:
                    name = ins.reg_name(ops[0].reg)
                    save = (name in SAVED and name not in written) or (name == "ecx" and i == 0)
                stack.append((depth, a, o, save))
                continue
            if m == "pop":
                depth -= 4
                stack = [s for s in stack if s[0] <= depth]
                since = min(since, len(stack))
                if ops[0].type == X.X86_OP_REG:
                    name = ins.reg_name(ops[0].reg)
                    reg[name] = ("pop", a)
                continue
            if m in ("sub", "add") and ops[0].type == X.X86_OP_REG \
                    and ins.reg_name(ops[0].reg) == "esp" and ops[1].type == X.X86_OP_IMM:
                depth += ops[1].imm if m == "sub" else -ops[1].imm
                stack = [s for s in stack if s[0] <= depth]
                since = min(since, len(stack))
                continue
            if ebp_frame and (m == "leave" or (m == "mov" and ins.op_str == "esp, ebp")):
                depth = 0 if m == "leave" else 4
                stack = [s for s in stack if s[0] <= depth]
                since = min(since, len(stack))
                continue
            if m == "call":
                direct = ops[0].imm if ops[0].type == X.X86_OP_IMM else None
                pops, known = None, False
                if direct is not None and self.m.in_text(direct):
                    pops = self.callee_pop(direct)
                    known = pops is not None
                pending = [s for s in stack[since:] if not s[3]]
                if pops is None:
                    nxt = ins_list[i + 1] if i + 1 < len(ins_list) else None
                    cdecl = (nxt is not None and nxt.mnemonic == "add"
                             and nxt.op_str.startswith("esp,"))
                    pops = 0 if cdecl else 4 * len(pending)
                nargs = pops // 4 if known else len(pending)
                taken = [s for s in stack if not s[3]][-nargs:] if nargs else []
                taken = taken[::-1]
                target = f"{direct:#x}" if direct is not None else ins.op_str.replace(
                    "dword ptr ", "")
                base = ""
                if ops[0].type == X.X86_OP_MEM and ops[0].mem.base:
                    base = ins.reg_name(ops[0].mem.base)
                fr.calls.append(Call(a, target, direct, reg["ecx"], reg["edx"],
                                     [t[2] for t in taken], [t[1] for t in taken], pops, known,
                                     base, tables.get(base, ())))
                depth -= pops
                stack = [s for s in stack if s[0] <= depth]
                since = len(stack)
                reg["eax"] = ("ret", a)
                reg["ecx"] = ("clobbered", a)
                reg["edx"] = ("clobbered", a)
                continue
            # --- data movement ------------------------------------------------
            if m == "lea" and ops[0].type == X.X86_OP_REG:
                name = ins.reg_name(ops[0].reg)
                s = slot(ops[1], ins)
                if s is not None:
                    reg[name] = ("local", s)
                else:
                    base = ins.reg_name(ops[1].mem.base) if ops[1].mem.base else None
                    src = reg.get(base, ("?", a)) if base and not ops[1].mem.index else ("?", a)
                    if src[0] in ("frame", "in", "abs", "ret", "field", "pop", "mem"):
                        reg[name] = ("lea", src, ops[1].mem.disp)
                    else:
                        reg[name] = ("expr", a)
                written.add(name)
                continue
            if m == "mov" and len(ops) == 2:
                if ops[0].type == X.X86_OP_REG:
                    name = ins.reg_name(ops[0].reg)
                    if name in REGS:
                        tables.pop(name, None)
                        if ops[1].type == X.X86_OP_MEM and ops[1].mem.base \
                                and not ops[1].mem.disp and not ops[1].mem.index:
                            source = ins.reg_name(ops[1].mem.base)
                            if source in reg:
                                tables[name] = reg[source]
                        reg[name] = origin(ops[1], ins)
                        written.add(name)
                    elif name in SUB:
                        reg[SUB[name]] = ("expr", a)
                        written.add(SUB[name])
                    continue
                if ops[0].type == X.X86_OP_MEM:
                    s = slot(ops[0], ins)
                    if s is not None:
                        fr.stores.setdefault(s, []).append((a, origin(ops[1], ins), ops[0].size))
                    elif ops[0].size == 4 and not ops[0].mem.index:
                        # a store off the stack: into an object's field, or a static
                        base = ins.reg_name(ops[0].mem.base) if ops[0].mem.base else None
                        key = ("abs",) if base is None else reg.get(base, ("?",))[:2]
                        if key[0] in ("abs", "in", "frame", "ret", "pop", "mem", "field"):
                            fr.held.setdefault(key, {}).setdefault(
                                ops[0].mem.disp & 0xffffffff, []).append(
                                (a, origin(ops[1], ins)))
                    continue
            if m == "xor" and len(ops) == 2 and ops[0].type == X.X86_OP_REG \
                    and ops[1].type == X.X86_OP_REG and ops[0].reg == ops[1].reg:
                name = ins.reg_name(ops[0].reg)
                if name in REGS:
                    reg[name] = ("imm", 0)
                    written.add(name)
                continue
            if m == "or" and len(ops) == 2 and ops[0].type == X.X86_OP_REG \
                    and ops[1].type == X.X86_OP_IMM and (ops[1].imm & 0xffffffff) == 0xffffffff:
                name = ins.reg_name(ops[0].reg)
                if name in REGS:
                    reg[name] = ("imm", 0xffffffff)
                    written.add(name)
                continue
            if m in ("and", "or", "add", "sub", "xor", "fstp", "fst", "fistp", "inc", "dec",
                     "mov", "movsd", "movss", "sete", "setne") and ops \
                    and ops[0].type == X.X86_OP_MEM:
                s = slot(ops[0], ins)
                if s is not None:
                    fr.stores.setdefault(s, []).append(
                        (a, ("op", f"{m} {ins.op_str.split(',')[-1].strip()}"), ops[0].size))
                continue
            # anything else that writes a register makes it an expression
            for op in ops[:1]:
                if op.type == X.X86_OP_REG:
                    name = ins.reg_name(op.reg)
                    name = SUB.get(name, name)
                    if name in REGS and m not in ("cmp", "test", "push"):
                        old = reg.get(name)
                        reg[name] = ("expr", a, m, ins.op_str, old)
                        written.add(name)
        self._frames[at] = fr
        return fr


def show(o) -> str:
    kind = o[0]
    if kind == "imm":
        return f"{o[1]:#x}"
    if kind == "local":
        return f"&local[{o[1]:+#x}]"
    if kind == "frame":
        return f"arg{o[1] // 4}" if o[1] >= 4 else f"local[{o[1]:+#x}]"
    if kind == "abs":
        return f"[{o[1]:#x}]"
    if kind == "in":
        return f"in:{o[1]}"
    if kind == "field":
        return f"{show(o[1])}->{o[2]:#x}"
    if kind == "lea":
        return f"&({show(o[1])}){o[2]:+#x}"
    if kind == "expr":
        if len(o) > 2:
            return f"expr@{o[1]:#x}({o[2]} {o[3]}; was {show(o[4]) if o[4] else '?'})"
        return f"expr@{o[1]:#x}"
    if kind == "op":
        return f"({o[1]})"
    return f"{kind}@{o[1]:#x}" if isinstance(o[1], int) else f"{kind}:{o[1]}"


def block(tr: Tracker, fr: Frame, base: int, before: int, words: int = 8) -> list[str]:
    """The stores into the dwords at `base`, and any constructor run on it, before a call."""
    out = []
    for c in fr.calls:
        if c.direct in tr.ctors and c.ecx == ("local", base) and c.at < before:
            k = tr.ctors[c.direct]
            out.append(f"      ctor{k} at {c.at:#x}: " + ", ".join(show(a) for a in c.args))
    for w in range(words):
        a = base + 4 * w
        hits = [s for s in fr.stores.get(a, []) if s[0] < before]
        # a store that is not dword-sized or not aligned is another variable's
        text = "; ".join(f"{show(o)} @{at:#x}" + ("" if size == 4 else f" ({size}b)")
                         for at, o, size in hits) or "-"
        out.append(f"      +{4 * w:#04x}: {text}")
    return out


def brief(o) -> str:
    kind = o[0]
    if kind in ("imm", "local", "frame", "abs", "field", "in"):
        return show(o)
    if kind == "expr":
        return f"expr@{o[1]:#x}"
    if kind == "op":
        return "float" if o[1].startswith("f") else f"({o[1]})"
    return show(o)


def row(tr: Tracker, fr: Frame, base: int, before: int, count: int = 8) -> str:
    """The block at `base` on one line: what each word last held before the call."""
    covered = {}
    for c in fr.calls:
        if c.direct in tr.ctors and c.ecx == ("local", base) and c.at < before:
            for k, a in enumerate(c.args):
                covered[k] = (c.at, a)
    out = []
    for w in range(count):
        hits = [(at, o) for at, o, size in fr.stores.get(base + 4 * w, []) if at < before]
        if w in covered:
            hits.append(covered[w])
        hits.sort()
        if not hits:
            out.append("-")
        else:
            seen = []
            for _, o in hits:
                text = brief(o)
                if text not in seen:
                    seen.append(text)
            out.append("|".join(seen))
    return "[" + ", ".join(out) + "]"


def shaped(tr: Tracker, fr: Frame, base: int, before: int) -> bool:
    """Whether the eight words at `base` are filled the way a filter is, before a call.

    Every one of the last four words -- the two flag masks and the two triangle masks -- holds
    a whole dword moved in as an integer, and so does the first, the classes; or one of the small
    constructors ran on the block and the words it leaves are moved in.  A float stored with
    `fstp` is a vector's or a matrix's, and a byte or a word is another variable's.
    """
    covered = 0
    for c in fr.calls:
        if c.direct in tr.ctors and c.ecx == ("local", base) and c.at < before:
            covered = max(covered, tr.ctors[c.direct])

    def moved(word: int) -> bool:
        if word < covered:
            return True
        return any(at < before and size == 4 and o[0] != "op"
                   for at, o, size in fr.stores.get(base + 4 * word, []))

    return all(moved(w) for w in (0, 4, 5, 6, 7))


def describe(tr: Tracker, fr: Frame, c: Call, words: int = 8) -> list[str]:
    out = [f"{tr.m.name}:{c.at:#x} in {fr.at:#x}: call {c.target}  ecx={show(c.ecx)} "
           f"edx={show(c.edx)}"
           f"  [{len(c.args)} pushed{'' if c.known else ', guessed'}"
           f"{'' if fr.ok else '; DEPTH NOT VERIFIED'}]"]
    for n, (o, at) in enumerate(zip(c.args, c.arg_at, strict=True), 1):
        out.append(f"    arg{n} = {show(o)}  (pushed at {at:#x})")
        if o[0] == "local":
            out += block(tr, fr, o[1], c.at, words)
    for name, o in (("ecx", c.ecx), ("edx", c.edx)):
        if o[0] == "local":
            out.append(f"    {name} = {show(o)}")
            out += block(tr, fr, o[1], c.at, words)
    return out


def vtables(tr: Tracker) -> set[int]:
    """Where a table of code pointers starts: every address outside `.text` that code stores
    or loads as an immediate, and that holds a code pointer.  A constructor installs a vtable
    with `mov dword ptr [reg], table`, and that immediate is the table's first slot."""
    cached = getattr(tr, "_vtables", None)
    if cached is not None:
        return cached
    m = tr.m
    out = set()
    for ins in m.sweep():
        for op in ins.operands:
            if op.type == X.X86_OP_IMM:
                v = op.imm & 0xffffffff
                if not m.in_text(v) and m.section_of(v) and m.in_text(m.dword(v)):
                    out.add(v)
    tr._vtables = out
    return out


#: The readers of a mesh triangle's flags, as (module, function, which pushed argument is the
#: filter): the triangle visitor's two callers and the push-out, all in `AniMesh.dll`.
SEEDS = (
    ("AniMesh.dll", 0x10010a50, 3),     # IJointMesh slot 10: a segment through one node's faces
    ("AniMesh.dll", 0x10013ef0, 2),     # IMesh2 slot 6: a segment through every node's
    ("AniMesh.dll", 0x1000d410, 3),     # interface 0x25 slot 3, the push-out: its segment filter
    ("AniMesh.dll", 0x1000d410, 4),     # ... and the filter its gathered faces are tested by
)

#: Modules that hold no face query: a function there that passes an argument on at a matching
#: offset is another interface's, and is listed and not followed.
FOREIGN = ("World3D.dll", "Ngi32.dll", "services.dll", "Net.dll", "Wizard.dll", "Joystick.dll",
           "iron_3d.exe")


def closure(extra: list[str]) -> int:
    """Every block handed to a reader of the triangle masks, through however many hands.

    From the readers outwards: who calls each one directly, and who calls the slot each sits in
    (the offset in every table that holds it, with as many pushed arguments as it pops).  A
    caller that hands on its own argument, or a copy of one, is a reader in turn; one that hands
    on a block it filled is a site, and its eight words are printed.
    """
    loaded = trackers()
    sinks: list[tuple[str, int, int]] = list(SEEDS)
    for spec in extra:
        name, target, index = spec.split(":")
        sinks.append((name, int(target, 0), int(index)))
    done: set = set()
    signatures: dict[tuple[int, int, int], list[str]] = {}
    sites: dict[tuple[str, int, int], str] = {}
    held: dict[tuple[str, int], str] = {}
    skipped: set[str] = set()

    def classify(name, tr, fr, c, o, via):
        if o[0] == "local":
            text = row(tr, fr, o[1], c.at)
            if shaped(tr, fr, o[1], c.at) and "->0x1c" not in text:
                sites.setdefault((name, fr.at, o[1]), f"{text}  -> {via} at {c.at:#x}")
                return
            copied = [w for w in text.strip("[]").split(", ") if "->0x1c" in w]
            if copied and copied[0].startswith("arg"):
                follow(name, fr.at, int(copied[0].split("->")[0].split("|")[0][3:]), via, c.at)
                return
            if any(fr.stores.get(o[1] + 4 * w) for w in (6, 7)):
                held.setdefault((name, c.at), f"a block not filled as a filter: {text} -> {via}")
            return
        if o[0] == "frame" and o[1] >= 4:
            follow(name, fr.at, o[1] // 4, via, c.at)
            return
        if o[0] in ("imm", "abs", "lea", "field"):
            held.setdefault((name, c.at), f"not on the stack: {show(o)} -> {via}")

    def follow(name, at, index, via, where):
        if name in FOREIGN:
            skipped.add(f"{name}:{at:#x} hands on its argument {index} at {where:#x} ({via})")
            return
        if (name, at, index) not in done and (name, at, index) not in sinks:
            sinks.append((name, at, index))

    while sinks:
        name, target, index = sinks.pop(0)
        if (name, target, index) in done:
            continue
        done.add((name, target, index))
        tr = loaded[name]
        m = tr.m
        pops = tr.callee_pop(target)
        label = f"{name}:{target:#x} arg {index}"
        # the tables that hold it
        starts = vtables(tr)
        needle = target.to_bytes(4, "little")
        for sec in m.pe.sections:
            if b".text" in sec.Name:
                continue
            blob = sec.get_data()
            base = m.base + sec.VirtualAddress
            i = blob.find(needle)
            while i >= 0:
                if i % 4 == 0 and pops is not None:
                    at = base + i
                    start = max((v for v in starts if v <= at), default=None)
                    if start is not None and at - start < 0x200:
                        signatures.setdefault((at - start, pops // 4, index), []).append(
                            f"{label} (table {start:#x} slot {(at - start) // 4})")
                i = blob.find(needle, i + 1)
        # its direct callers
        for f in m.functions():
            fr = tr.frame(f.at)
            for c in fr.calls:
                if c.direct == target and len(c.args) >= index:
                    classify(name, tr, fr, c, c.args[index - 1], label)
        # every call at a signature's offset, in every module
        for offset, nargs, k in list(signatures):
            if (offset, nargs, k) in done:
                continue
            done.add((offset, nargs, k))
            needle_s = f"+ {offset:#x}]"
            via = f"slot +{offset:#x}/{nargs}"
            for other, otr in loaded.items():
                for f in otr.m.functions():
                    fr = otr.frame(f.at)
                    for c in fr.calls:
                        if c.direct is not None or not c.target.startswith("[e") \
                                or not c.target.endswith(needle_s):
                            continue
                        if fr.ok:
                            if len(c.args) == nargs and k <= nargs:
                                classify(other, otr, fr, c, c.args[k - 1], via)
                        else:
                            for o in c.args[:8]:
                                if o[0] == "local" and shaped(otr, fr, o[1], c.at):
                                    sites.setdefault(
                                        (other, fr.at, o[1]),
                                        f"{row(otr, fr, o[1], c.at)}  -> {via} at {c.at:#x} "
                                        f"(DEPTH NOT VERIFIED)")
    print(f"readers and the hands a filter passes through: "
          f"{len([d for d in done if isinstance(d[0], str)])}")
    for d in sorted(d for d in done if isinstance(d[0], str)):
        print(f"  {d[0]}:{d[1]:#x} argument {d[2]}")
    print(f"slots searched: {len(signatures)}")
    for (offset, nargs, k), names in sorted(signatures.items()):
        print(f"  +{offset:#x}, {nargs} pushed, argument {k}: {'; '.join(sorted(set(names)))}")
    print(f"blocks filled and handed on: {len(sites)}")
    for (name, at, base), text in sorted(sites.items()):
        print(f"  {name}:{at:#x} local[{base:+#x}] {text}")
    print(f"other origins: {len(held)}")
    for (name, at), text in sorted(held.items()):
        print(f"  {name}:{at:#x} {text}")
    print(f"not followed: {len(skipped)}")
    for line in sorted(skipped):
        print(f"  {line}")
    return 0


#: The face-query slots, as (offset, pushed arguments, which is the filter, what it is).  Each
#: is a slot the closure reaches from the readers through a table in `AniMesh.dll` or
#: `Terrain.dll`: the mesh's, the building's and the landscape's interface 0x18, the world's
#: object container (table `Terrain.dll:0x1009a630`), the mesh's interface 0x20 and 0x25.
QUERIES = (
    (0x18, 3, 2, "interface 0x18 slot 6, a segment's first face"),
    (0x1c, 3, 2, "slot 7: the walk-face query on a mesh, the segment query on the world"),
    (0x20, 3, 2, "slot 8"),
    (0x28, 4, 3, "slot 10: a segment through one node (interface 0x20), the world's ground query"),
    (0x30, 5, 2, "the world's slot 12"),
    (0xc, 5, 3, "interface 0x25 slot 3, the push-out: its segment filter"),
    (0xc, 5, 4, "interface 0x25 slot 3, the push-out: its face filter"),
)


def queries(verbose: bool) -> int:
    """Every call at a face-query slot, sorted by where its filter argument came from."""
    loaded = trackers()
    grand = {"block": 0, "copy": 0, "param": 0, "held": 0, "other": 0}
    for offset, nargs, index, what in QUERIES:
        needle = f"+ {offset:#x}]"
        total = 0
        kinds: dict[str, list[str]] = {"block": [], "copy": [], "param": [], "held": [],
                                       "other": [], "unverified": []}
        for name, tr in loaded.items():
            for f in tr.m.functions():
                fr = tr.frame(f.at)
                for c in fr.calls:
                    if c.direct is not None or not c.target.startswith("[e") \
                            or not c.target.endswith(needle):
                        continue
                    total += 1
                    head = f"{name}:{c.at:#x} in {fr.at:#x}"
                    if not fr.ok:
                        parts = [f"arg{n}={show(o)} {row(tr, fr, o[1], c.at)}"
                                 for n, o in enumerate(c.args, 1)
                                 if o[0] == "local" and shaped(tr, fr, o[1], c.at)]
                        if parts:
                            kinds["unverified"].append(head + ": " + "; ".join(parts))
                        continue
                    if len(c.args) != nargs:
                        continue
                    o = c.args[index - 1]
                    if o[0] == "local":
                        text = row(tr, fr, o[1], c.at)
                        if "->0x1c" in text:
                            kind = "copy"
                        elif shaped(tr, fr, o[1], c.at):
                            kind = "block"
                        else:
                            kind = "other"
                        kinds[kind].append(f"{head}: {show(o)} {text}")
                    elif o[0] == "frame" and o[1] >= 4:
                        kinds["param"].append(f"{head}: {show(o)}")
                    elif o[0] in ("abs", "lea", "field") or (
                            o[0] == "imm" and tr.m.section_of(o[1])
                            and not tr.m.in_text(o[1])):
                        kinds["held"].append(f"{head}: {show(o)}")
                    else:
                        kinds["other"].append(f"{head}: {brief(o)} ecx={brief(c.ecx)} "
                                              f"edx={brief(c.edx)}")
        print(f"##### +{offset:#x}, {nargs} pushed, filter argument {index} ({what}): "
              f"{total} calls at the offset in all")
        for kind, lines in kinds.items():
            if kind in grand:
                grand[kind] += len(lines)
            print(f"=== {kind}: {len(lines)}")
            if kind != "other" or verbose:
                for line in lines:
                    print("  " + line)
    print("in all: " + ", ".join(f"{k} {v}" for k, v in grand.items()))
    return 0


def passed(c: Call) -> str:
    """How a call through a slot hands over its object: in `ecx`, on the stack, or not seen.

    A face query takes its object in `ecx`.  A call through `[ecx + k]` has its table in `ecx`, so
    whatever it calls is not given an object there; the same goes for one that pushes last the
    pointer it read its table out of.
    """
    if c.base == "ecx":
        return "stack"
    if c.receiver:
        if c.args and c.args[0] == c.receiver:
            return "stack"
        if c.ecx == c.receiver:
            return "ecx"
    return "unseen"


def receivers() -> int:
    """Every call at a face-query slot, by what its filter argument is and how its object goes.

    The control is the filter blocks and their copies: all of those must say `ecx`.
    """
    loaded = trackers()
    for offset, nargs, index, what in QUERIES:
        needle = f"+ {offset:#x}]"
        counts: dict[tuple[str, str], int] = {}
        listed = []
        for name, tr in loaded.items():
            for f in tr.m.functions():
                fr = tr.frame(f.at)
                for c in fr.calls:
                    if c.direct is not None or not c.target.startswith("[e") \
                            or not c.target.endswith(needle) or not fr.ok \
                            or len(c.args) != nargs:
                        continue
                    o = c.args[index - 1]
                    if o[0] == "imm" and not (tr.m.section_of(o[1]) and not tr.m.in_text(o[1])):
                        kind = "constant"
                    elif o[0] == "local":
                        text = row(tr, fr, o[1], c.at)
                        kind = "filled" if "->0x1c" in text or shaped(tr, fr, o[1], c.at) \
                            else "local"
                    elif o[0] == "frame" and o[1] >= 4:
                        kind = "param"
                    else:
                        kind = "loaded"
                    how = passed(c)
                    counts[(kind, how)] = counts.get((kind, how), 0) + 1
                    if kind in ("loaded", "local") and how != "stack":
                        listed.append(f"  {name}:{c.at:#x} in {fr.at:#x} {how}: {show(o)} "
                                      f"[{', '.join(brief(a) for a in c.args)}] "
                                      f"ecx={brief(c.ecx)} edx={brief(c.edx)}")
        print(f"##### +{offset:#x}, argument {index} of {nargs} ({what})")
        for key in sorted(counts):
            print(f"  {key[0]:9s} object {key[1]:7s} {counts[key]:4d}")
        print("\n".join(listed))
    return 0


def trackers(names=cov.MODULES):
    return {name: Tracker(mod) for name, mod in cov.load(names).items()}


def main(argv: list[str]) -> int:
    cmd = argv[1] if len(argv) > 1 else "check"
    if cmd == "check":
        total = bad = 0
        for name, tr in trackers().items():
            n = wrong = 0
            for f in tr.m.functions():
                fr = tr.frame(f.at)
                if not fr.rets:
                    continue
                n += 1
                wrong += not fr.ok
            total += n
            bad += wrong
            print(f"{name:14s} {n:5d} functions with a ret, depth zero at every one on "
                  f"{n - wrong:5d}; constructors {sorted(set(tr.ctors.values()))} "
                  f"x {len(tr.ctors)}")
        print(f"{total} functions, {total - bad} verified, {bad} not")
        return 0
    if cmd == "receivers":
        return receivers()
    if cmd == "bad":
        name = argv[2]
        tr = trackers((name,))[name]
        for f in tr.m.functions():
            fr = tr.frame(f.at)
            if fr.rets and not fr.ok:
                print(f"{f.at:#x} size {f.size}")
        return 0
    if cmd == "into":
        # who hands a function its k-th argument, and what tables hold the function:
        # `into Terrain.dll:0x100250c0:3 ...`
        loaded: dict[str, Tracker] = {}
        for spec in argv[2:]:
            name, target, index = spec.split(":")
            target, index = int(target, 0), int(index)
            if name not in loaded:
                loaded.update(trackers((name,)))
            tr = loaded[name]
            m = tr.m
            print(f"== {name}:{target:#x} argument {index}; it pops {tr.callee_pop(target)}")
            needle = target.to_bytes(4, "little")
            starts = vtables(tr)
            for sec in m.pe.sections:
                if b".text" in sec.Name:
                    continue
                blob = sec.get_data()
                base = m.base + sec.VirtualAddress
                i = blob.find(needle)
                while i >= 0:
                    if i % 4 == 0:
                        at = base + i
                        start = max((v for v in starts if v <= at), default=None)
                        where = (f"table {start:#x} slot {(at - start) // 4} (+{at - start:#x})"
                                 if start is not None else "no table start found")
                        print(f"  held at {at:#x}: {where}")
                    i = blob.find(needle, i + 1)
            for f in m.functions():
                fr = tr.frame(f.at)
                for c in fr.calls:
                    if c.direct != target:
                        continue
                    o = c.args[index - 1] if len(c.args) >= index else ("?", c.at)
                    text = show(o)
                    if o[0] == "local":
                        text += " " + row(tr, fr, o[1], c.at)
                    print(f"  called at {c.at:#x} in {fr.at:#x}: arg{index} = {text}"
                          f"{'' if fr.ok else '  DEPTH NOT VERIFIED'}")
        return 0
    if cmd == "sink":
        # every call at a slot's offset passing `nargs`, sorted by what its filter argument is:
        # `sink 0x18:3:2 0x28:4:3 ...` (offset : pushed arguments : which one is the filter)
        loaded = trackers()
        for spec in (a for a in argv[2:] if a != "-v"):
            offset, nargs, index = (int(v, 0) for v in spec.split(":"))
            needle = f"+ {offset:#x}]"
            total = 0
            kinds: dict[str, list[str]] = {"block": [], "param": [], "copy": [], "other": [],
                                           "unverified": []}
            for name, tr in loaded.items():
                for f in tr.m.functions():
                    fr = tr.frame(f.at)
                    for c in fr.calls:
                        if c.direct is not None or not c.target.startswith("[e") \
                                or not c.target.endswith(needle):
                            continue
                        total += 1
                        head = f"{name}:{c.at:#x} in {fr.at:#x} [{len(c.args)} pushed]"
                        if not fr.ok:
                            # the count is not to be trusted: show every pushed block there is
                            parts = []
                            for n, o in enumerate(c.args, 1):
                                if o[0] == "local" and any(
                                        fr.stores.get(o[1] + 4 * w) for w in (4, 5, 6, 7)):
                                    parts.append(f"arg{n}={show(o)} {row(tr, fr, o[1], c.at)}")
                                elif o[0] == "frame" and o[1] >= 4:
                                    parts.append(f"arg{n}={show(o)}")
                            if parts:
                                kinds["unverified"].append(head + ": " + "; ".join(parts))
                            continue
                        if len(c.args) != nargs:
                            continue
                        o = c.args[index - 1]
                        if o[0] == "local":
                            text = row(tr, fr, o[1], c.at)
                            if shaped(tr, fr, o[1], c.at):
                                kind = "block"
                            elif "->0x18" in text or "->0x1c" in text:
                                kind = "copy"
                            else:
                                kind = "other"
                            kinds[kind].append(f"{head}: arg{index} = {show(o)} {text}")
                        elif o[0] == "frame" and o[1] >= 4:
                            kinds["param"].append(f"{head}: arg{index} = {show(o)}")
                        else:
                            kinds["other"].append(f"{head}: arg{index} = {brief(o)}")
            print(f"##### +{offset:#x}, {nargs} pushed, filter argument {index}: "
                  f"{total} calls at the offset in all")
            for kind, lines in kinds.items():
                print(f"=== {kind}: {len(lines)}")
                if kind != "other" or "-v" in argv:
                    print("\n".join(lines))
        return 0
    if cmd == "closure":
        return closure(argv[2:])
    if cmd == "queries":
        return queries("-v" in argv)
    if cmd == "at":
        name, where = argv[2], int(argv[3], 0)
        words = int(argv[4]) if len(argv) > 4 else 8
        tr = trackers((name,))[name]
        f = tr.m.function_at(where)
        fr = tr.frame(f.at)
        for c in fr.calls:
            if c.at == where:
                print("\n".join(describe(tr, fr, c, words)))
        return 0
    if cmd == "fn":
        name, where = argv[2], int(argv[3], 0)
        tr = trackers((name,))[name]
        f = tr.m.function_at(where)
        fr = tr.frame(f.at)
        print(f"{name} {f.at:#x}: depth verified {fr.ok}, {fr.rets} rets")
        for c in fr.calls:
            print("\n".join(describe(tr, fr, c)))
        return 0
    print(__doc__)
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
