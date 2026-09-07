# Method, and what changed when we started disassembling

Most of this project was derived by observing the shipped data files: parse a
format, predict something it implies, and check the prediction against the
whole install. That worked for the container, textures, terrain, missions and
object geometry.

It stopped working for `Land.map`. Three rounds of inference established what
the file was *not* — not a raster, not a fixed-stride record array, not
run-length encoded — and produced a partial reading of the first 5% that could
not be walked reliably. The blocker was structural: the record size depends on
a field, and the element count is not in the payload at all.

So from that point on, `ArealMap.dll` was disassembled. The format in
[08-arealmap.md](08-arealmap.md) came out of the loader in about an hour, after
three rounds of guessing had failed.

## What we take from a binary, and what we don't

**Taken:** facts about the file format — field offsets, record sizes, the order
of reads, the meaning of a flag, the constants a loop compares against. These
are facts about a data layout, not expression.

**Not taken:** code. Nothing is transcribed, translated or paraphrased from
disassembly into `openparkan/`. The readers are written from the documented
layout, in the project's own idiom, and every one of them is checked against
the shipped data rather than against the binary.

The `analysis/` directory holds the scaffolding used to do this — a small PE
loader and a capstone wrapper. It is not part of the library and nothing in
`openparkan/` imports it.

## Why this is a reasonable thing to do

Reverse engineering for interoperability is long-settled ground: EU Software
Directive 2009/24/EC Article 6, DMCA §1201(f) in the US, and *Sega v. Accolade*
(9th Cir. 1992). It requires a lawfully obtained copy — this repository
contains no game data and expects you to point it at your own installation.

The distinction that matters in practice is the one above: a file format is
not copyrightable, an implementation of it is. Keeping the readers clean of
copied code is what preserves the value of the work, not avoiding the
disassembler.

## It cuts both ways

Disassembly also **corrected** things inference had got wrong, and confirmed
things it had got right:

- The NRes directory field at +4, documented as "zero in every shipped file",
  is an **element count**. It is zero on most members, which is how the error
  survived. `ArealMap.dll` reads its areal count from it.
- That field then independently confirmed the terrain parse: for every terrain
  stream on every map, the engine's element count equals the vertex or face
  count derived from the stream data alone — 5692 and 4601 on SC_3.
- The loader looks up chunk type 12 and strides the directory by 64 bytes,
  matching the container model built from data.

## Tools

`analysis/pe.py` wraps `pefile` and `capstone`: section and export listing,
string extraction, raw cross-reference search over `.text` (linear sweep
desynchronises on this binary, so references are found by scanning for the
little-endian address), a prologue-walker, and an annotating disassembler.

```
uv sync --group analysis
uv run python -c "
import sys; sys.path.insert(0, 'analysis')
from pe import Binary, string_annotator
from openparkan import gamedir
b = Binary(str(gamedir.find() / 'ArealMap.dll'))
print('\n'.join(b.disasm(0x1001e0d0, 40, annotate=string_annotator(b))))
"
```

## What the binaries volunteer

The DLLs were shipped with names and diagnostics intact, which does much of the
work:

- **Exports are named**: `CreateArealMap`, `CreateSystemArealMap`,
  `CreateHallWay`, `GetSystemArealMap`.
- **Debug dumps name the fields**: `Areals number`, `CellStartIndex`,
  `CellItemNum`, `Overal vertices number`, `Minimum number verices` (sic).
- **Panics name the invariants**: *"ArealMap Cells are empty"*, *"Incorrect
  ArealMap"*, *"->CellArray is broken"*, *"->Hole in ArealGeometry"*,
  *"has center that outside contour"*.
- **One source path survived**:
  `C:\ESTARIOL\IronStrategy\ArealMap\SystemArealMap.cpp`.

That thread ran further than expected. `MHallWay panic: cannot load path
graph` led to `MHallWay::LoadFromResource`, whose loader reads chunk type 17
of a nested resource — which turned out to be an object-mesh stream that is
non-empty on buildings and nowhere else.

And the per-face texture assignment, which three rounds of data inference had
failed to find, fell out of a detail the binary supplied rather than the
disassembly proper: the NRes element-count field. It gave stream 13 a stride of
20 bytes rather than the 12 a hex dump suggested, and at 20 bytes the stream is
plainly a list of draw batches. There was never a per-face material field to
find.
