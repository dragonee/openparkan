# analysis

Scaffolding for reverse-engineering the game DLLs. **Not part of the
`openparkan` library** — nothing under `openparkan/` imports it.

Its job is to recover *format facts* — field offsets, record sizes, the order
of reads, constants — which are then written up in `docs/` and implemented
independently. No disassembled code is copied into the library. See
[../docs/09-method.md](../docs/09-method.md).

```
uv sync --group analysis
```

`pe.py` wraps `pefile` and `capstone` with the helpers this binary needs:
section and export listing, string extraction, raw cross-reference search over
`.text`, a prologue-walker, and a disassembler that annotates string operands.

`names.py` extracts the engine's name-to-number tables. Several of the game's
data files are plain text naming things like `MCMD_LEFT`, `CICLS_TURRET` and
`SCAN_A`, so every binary that reads one carries a resolver — a chain of
string compares, each case returning a constant. Those chains are the
developers' own vocabulary, and this pulls them out:

```
uv run --group analysis python analysis/names.py World3D.dll
uv run --group analysis python analysis/names.py iron3d.dll --prefix CMD
```

Its regression test is that it reproduces every table already in
`openparkan/controls.py` byte for byte — `SCAN` (174), `CMD` (43+31),
`MCMD` (22), `CIS` (15), `CICLS` (13), `MAN` (2). Re-run it after any change.
Swept across the whole installation, `World3D.dll` and `iron3d.dll` are the
only binaries that carry a resolver chain at all; the `GMSG_*` and `VOICE_*`
identifiers elsewhere are log strings and lookup keys, not compare cases.

## Things that cost us time

- **A resolver has four shapes**, and missing one loses entries silently. They
  are documented in `names.py`'s module docstring: the ordinary `mov eax, K`;
  a case returning the zero `strcmp` already left in `eax`; a case writing to
  an out-parameter (`mov dword ptr [edi], 0x2ed`), which is what `iron3d.dll`
  does; and the chain's branchless last case
  (`neg eax; sbb eax, eax; and eax, M; add eax, K`). The compiler also elides
  the test where a case and the default are both zero.
- **A family that comes out all zeros is a bug, not a finding.** That is the
  signature of latching onto pushed strings that are not compare cases.
- **A linear capstone sweep over `.text` desynchronises** on jump tables and
  inline data, and then silently stops. Every scan must restart a byte later
  when `disasm` stalls. `pe.py`'s `xrefs_to` does *not* do this and so misses
  references; `function_start` is unreliable for the same reason and has
  pointed at the wrong function more than once. Prefer the loop in `names.py`.
- **Don't match disassembly with regular expressions where operands matter.**
  Use `detail = True` and read `ins.operands` and their `access` flags. Two
  regex passes over the same property table once disagreed with each other.
- **Check a recovered table against the shipped files before believing it.**
  The scan codes were confirmed twice over: they are the real IBM PC set-1
  numbers, and `ScanCode.dsc` lists its first 56 entries in code order.


## Where the behaviour interpreter lives

`ai.dll` loads and runs the `.scr` scripts — not `Behavior.dll`, which owns
the research tree. Useful addresses, all in `ai.dll`:

| address | what |
|---|---|
| `0x100014f9` | `cmp edi, 0x49` — the script version check; the error text calls them "not up to date" |
| `0x1000129e` | the loader's initialiser, which writes 70 handler pointers at object offsets `0xc`..`0x120` |
| `0x100122b5` | the dispatch loop: load `head[0]`, test against −1, `call [table + id*4]`, follow `[node+8]` to the next |
| `0x10012313` | the branch taken when `head[0]` is −1 |

See [../docs/15-behaviour.md](../docs/15-behaviour.md). The handlers carry no
strings, so the binary does not name them.


## The component registry

`Comp.ini` is read by `World3D.dll`, and `registry.py` checks every row of it
against the export table of the DLL it names:

```
uv run --group analysis python analysis/registry.py
```

| address | what |
|---|---|
| `0x10014790` | the reader; its own error string calls it `LoadComponentAddr` |
| `0x10026bec` | `'%d %s %s'` — a row is an int and two words, the rest free text |
| `0x100666b8` | the table it fills, 16 bytes a row: id, module handle, entry point |
| `0x10795284` | the row counter, advanced only when both lookups succeed |
| `0x10013f84` | the caller, which pushes the filename |

See [../docs/22-settings.md](../docs/22-settings.md).


## The research tree's loader

`Comp.ini` names it: `CID_RESEARCH 7 misload.dll LoadResearch`. Addresses, all
in `MisLoad.dll`:

| address | what |
|---|---|
| `0x100025f0` | `LoadResearch`; allocates 0x138 and a 0x80-byte reader at `+0x130` |
| `0x1000e18c` | the reader's vtable; slot 6 (`+0x18`) loads |
| `0x10002fe0` | the load: twelve streams in a fixed order, ten of them required |
| `0x1000302c` | the version gate -- the directory's second count over `TRF0` must be 3 |
| `0x1000306d` | the same field over `TRF1`, kept as a boolean |
| `0x100030c1` | the one writable copy: `TRF1` into a zeroed buffer |
| `0x100032cd` | the count/pointer pairing over `TRF2`/`TRF3`, and `0x100033a5` for `TRF4`/`TRF5` |

| `0x10002f60` | slot 44: `TRFB` index -> part id text and the item that researches it |
| `0x10002d50`.. | ten per-field getters over the 40-byte record, `0x30` apart |
| `0x1000e130` | the loaded tree's vtable, 55 slots |

**A search that missed its target, worth remembering.** A first pass concluded
nothing in the DLL indexed a 40-byte record, having looked for `imul` by 40 and
for `lea r,[r+r*4]` followed by `shl r,3`. The compiler emitted neither: it
scales by five with `lea eax, [eax + eax*4]` and by eight in the *addressing
mode* of the load that follows, `[ecx + eax*8 + 0x23]`. There is no multiply
instruction to find. When searching for a stride, search for the scaled
addressing mode too, and treat "no hits" as "the search was wrong" until a
positive control says otherwise.

See [../docs/16-research.md](../docs/16-research.md).
