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

### Searches that do not discriminate, so nobody repeats them

- **Looking for a filter by its constant pair.** A face query's filter is six
  words, of which two are a required and an excluded triangle mask four bytes
  apart. Scanning every module for a pair of dword immediate stores four bytes
  apart with a candidate mask returns several hundred hits, dominated by
  unrelated `0x10`s in `ArealMap.dll` and `Behavior.dll`, and separates nothing.
  What did work was going the other way: enumerate the readers of the field, and
  then the callers of each.
- **Looking for a flag by its immediate.** A plain immediate scan finds a mask
  only where the compiler kept it as one. `Ngi32.dll`'s texture loader isolates
  bit 21 of its load flags with a shift and an `and 1`, so the constant
  `0x200000` is nowhere in the module, while every neighbouring load flag is
  ([02-texm.md](02-texm.md#who-loads-a-texture-opaque--read-and-measured)). A
  sweep for shifts by the bit's index, across all the modules at once, is short
  enough to read: by `0x15` it returns seven sites, six of them one statically
  linked CRT routine.


## Reading other people's work

Disassembly is not the only source of facts. `Land.map`'s cell array and the
mesh batch list came out of the binaries; the mesh **slot** layout came out of
[fparkan](https://github.com/valentineus/fparkan)'s format reference, after
this project had spent a round failing to find it — including an exhaustive
search over `header + count * unit + tail` layouts that never tried a fixed
140-byte header.

The same discipline applies as with a binary, plus one more consideration.
fparkan is GPL-2.0-only — its `Cargo.toml` says exactly that, and nothing in
the project elects "or later" — while this project is GPL-3.0. Those two do
not combine in either direction, so the rule is not a preference:
**documentation only, never source.** A file format is a fact and facts are
not copyrightable; a particular implementation is expression and is. Every
borrowed fact was then checked against the shipped data before being relied
on — `140 + 68 * count` accounts for stream 2 exactly on all 434 meshes, and
every node slot index addresses a real slot.

That check is not a formality. It is what turns someone else's claim into
something this project knows.

### The second consultation went the other way

fparkan's notes were read again for two bits this project could not name: the
terrain surface word's `0x10` and the draw order's `0x80`. **Neither is named
there either**, which is worth writing down — a negative result from the other
project saves the next person the same trip.

What it did give was a frame. The surface word turns out to be a **16-bit
compaction of a 32-bit engine mask** with a documented bit mapping, so the
unnamed `0x10` is the engine's `0x00001000`; and the engine packs a **six-bit
surface class** from six more of those bits, which is what face field 13 is.
That reading was checked before it was used: every one of the 275882 faces
holds a field-13 value below 64. It also settled an older negative — a
bitfield's groups have no reason to be spatial or to track a material, which
is exactly why field 13 failed as a patch id.

And in two places the reading here goes further: fparkan leaves the face
record's last eight bytes uninterpreted, six of which are the face's own
normal, and it describes stream 11 only as "cell accelerator data", where it
is the draw order with a batch-start bit. Both are checked on all 275882
faces.

The exchange is not one-directional, and recording which way it ran each time
is part of the same honesty as recording where a fact came from.
