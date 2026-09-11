"""Minimal PE + x86 disassembly helpers for reverse-engineering the game DLLs.

This directory is analysis scaffolding, not part of the ``openparkan`` library.
Its purpose is to recover *format facts* -- field offsets, record layouts,
constants -- which then get written up in ``docs/`` and implemented
independently.  No disassembled code is copied into the library.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

import capstone
import pefile


@dataclass
class Ref:
    addr: int          # virtual address of the instruction
    text: str          # disassembled instruction


class Binary:
    def __init__(self, path: str):
        self.path = path
        self.pe = pefile.PE(path)
        self.base = self.pe.OPTIONAL_HEADER.ImageBase
        self.md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
        self.md.detail = True
        self.text = next(s for s in self.pe.sections
                         if s.Name.rstrip(b"\0") == b".text")
        self.text_va = self.base + self.text.VirtualAddress
        self.text_data = self.text.get_data()

    def exports(self) -> dict[str, int]:
        """Every named export, as ``name -> virtual address``."""
        self.pe.parse_data_directories(directories=[
            pefile.DIRECTORY_ENTRY["IMAGE_DIRECTORY_ENTRY_EXPORT"]])
        directory = getattr(self.pe, "DIRECTORY_ENTRY_EXPORT", None)
        if directory is None:
            return {}
        return {e.name.decode(): self.base + e.address
                for e in directory.symbols if e.name}

    # --- address helpers -------------------------------------------------
    def va_to_off(self, va: int) -> int | None:
        rva = va - self.base
        for s in self.pe.sections:
            span = max(s.Misc_VirtualSize, s.SizeOfRawData)
            if s.VirtualAddress <= rva < s.VirtualAddress + span:
                return s.PointerToRawData + (rva - s.VirtualAddress)
        return None

    def read(self, va: int, n: int) -> bytes:
        off = self.va_to_off(va)
        return b"" if off is None else self.pe.__data__[off:off + n]

    def cstring(self, va: int, limit: int = 200) -> str:
        raw = self.read(va, limit)
        return raw.split(b"\0")[0].decode("latin-1", "replace")

    # --- strings ---------------------------------------------------------
    def find_strings(self, pattern: str) -> list[tuple[int, str]]:
        """Virtual addresses of NUL-terminated strings matching ``pattern``."""
        rx = re.compile(pattern.encode(), re.I)
        out = []
        for s in self.pe.sections:
            data = s.get_data()
            base = self.base + s.VirtualAddress
            for m in re.finditer(rb"[\x20-\x7e]{4,}\x00", data):
                if rx.search(m.group()):
                    out.append((base + m.start(), m.group()[:-1].decode("latin-1")))
        return out

    # --- code ------------------------------------------------------------
    def xrefs_to(self, value: int) -> list[Ref]:
        """Instructions whose immediate or displacement equals ``value``."""
        out = []
        for ins in self.md.disasm(self.text_data, self.text_va):
            for op in ins.operands:
                if op.type == capstone.x86.X86_OP_IMM and op.imm == value:
                    out.append(Ref(ins.address, f"{ins.mnemonic} {ins.op_str}"))
                elif op.type == capstone.x86.X86_OP_MEM and op.mem.disp == value \
                        and op.mem.base == 0 and op.mem.index == 0:
                    out.append(Ref(ins.address, f"{ins.mnemonic} {ins.op_str}"))
        return out

    def function_start(self, va: int, back: int = 0x900) -> int:
        """Walk backwards to the nearest plausible prologue before ``va``."""
        start = max(self.text_va, va - back)
        best = None
        for ins in self.md.disasm(self.text_data[start - self.text_va:va - self.text_va], start):
            if ins.mnemonic == "push" and ins.op_str == "ebp":
                nxt = list(self.md.disasm(
                    self.text_data[ins.address + ins.size - self.text_va:][:8],
                    ins.address + ins.size))
                if nxt and nxt[0].mnemonic == "mov" and nxt[0].op_str == "ebp, esp":
                    best = ins.address
        return best if best is not None else start

    def disasm(self, va: int, count: int = 80, annotate=None) -> list[str]:
        off = va - self.text_va
        out = []
        for ins in self.md.disasm(self.text_data[off:off + count * 8], va):
            line = f"  {ins.address:08x}  {ins.mnemonic:<7} {ins.op_str}"
            if annotate:
                extra = annotate(self, ins)
                if extra:
                    line += f"   ; {extra}"
            out.append(line)
            if len(out) >= count:
                break
        return out


def string_annotator(bin_: Binary):
    """Annotate instructions that reference a printable string."""
    def ann(b: Binary, ins):
        for op in ins.operands:
            if op.type == capstone.x86.X86_OP_IMM:
                s = b.cstring(op.imm, 60)
                if len(s) >= 4 and all(32 <= ord(c) < 127 for c in s):
                    return repr(s)
        return None
    return ann
