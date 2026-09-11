# Task: land the `CMD_` command table

A handoff brief. Read this, then `README.md` and `docs/09-method.md`.

## The project in three sentences

`openparkan` is a clean-room, MIT-licensed reimplementation of the readers for
*Parkan: Iron Strategy* (Nikita, 1998). It ships no game data; you point it at
your own installation. Every factual claim in `docs/` is re-derivable by
`uv run openparkan verify`, which is the contract the whole project rests on.

## Rules that are not negotiable

- **fparkan is GPL-2.0 and this project is MIT. Read its *documentation only*,
  never its source.** A file format is a fact and facts are not copyrightable;
  a particular implementation is expression and is. Every borrowed fact is
  checked against the shipped data. See `docs/09-method.md`.
- Disassembly recovers **format facts only** — offsets, sizes, read order,
  constants. No disassembled code is copied into the library. `analysis/` is
  scaffolding and is imported by nothing.
- **No game assets in the repository.** Tests build the bytes they read.
- `uv run ruff check` and `uv run pytest` must pass before every commit, and
  `uv run openparkan verify` must stay green.
- Don't ship a reader that doesn't work. Record partial results as documented
  open questions instead.
- Commits end with the Co-Authored-By and Claude-Session attribution lines.

## Background: why this is possible at all

The game parses several of its data files as **text**. `hero.tbl` names a
movement command `MCMD_LEFT`, a component class `CICLS_TURRET`, a key
`SCAN_A`. So every binary that reads one of those files has to carry a
resolver turning each name into the number the engine dispatches on — and
those resolvers are the developers' own vocabulary, sitting in the binary in
plain sight.

That is how `openparkan/controls.py` already holds `SCAN` (174 entries),
`MCMD` (22), `CICLS` (13), `CIS` (15) and `MAN` (2). See `docs/14-controls.md`.

## The tool

`analysis/names.py` extracts these tables. It is written and validated:

```
uv run --group analysis python analysis/names.py World3D.dll
uv run --group analysis python analysis/names.py iron3d.dll --prefix CMD
```

**It reproduces all five shipped tables byte for byte** — that is its
regression test, and you should re-run it after any change:

| family | tool | `controls.py` |
|---|---:|---:|
| `SCAN` | 174 | 174 |
| `MCMD` | 22 | 22 |
| `CICLS` | 13 | 13 |
| `CIS` | 15 | 15 |
| `MAN` | 2 | 2 |

## What the sweep already found

Run across every DLL in the installation, **only two carry resolver chains**:

| binary | names | families |
|---|---:|---|
| `World3D.dll` | 269 | `SCAN`(174), **`CMD`(43)**, `MCMD`(22), `CIS`(15), `CICLS`(13), `MAN`(2) |
| `iron3d.dll` | 33 | **`CMD`(31)**, `DEFAULT`(1), `SFX`(1) |

`Net.dll`, `Behavior.dll`, `ai.dll`, `Control.dll`, `Terrain.dll` and the rest
have **none**. So this technique is narrower than it first looked: it is not a
general key to the engine, it is the parser for the text-configured layer. The
`GMSG_*` and `VOICE_*` identifiers in the binaries are log strings and lookup
keys, not resolver cases — don't chase them.

## The task

**The `CMD_` table is the one thing the sweep turned up that is not yet in the
library, and it is complete.**

- `World3D.dll` resolves **43** names, values **1..66**.
- `iron3d.dll` resolves **31** names, values **35..754**.
- Between them they cover **all 72 names in `Command.dsc`**, with nothing left
  over in either direction.
- Exactly **one** name is in both — `CMD_CAMERA_INFRARED`, and the two
  resolvers **agree** that it is 35.

The two look like a partition: the in-cockpit commands (`CMD_OBJ_MOVE_LEFT` is
1, `CMD_FIRE_ALL` is 66) against the commander's (`CMD_JAMES_HQ_MOVE_LEFT` is
723). Establish whether that reading holds before writing it down.

What to do:

1. Add `CMD` to `openparkan/controls.py` beside the other tables, with the
   same provenance comment style. Decide and **document** how the two sources
   are merged — one dict or two — and say which binary each value came from.
2. Give `Binding` a way to resolve its command to a number, the way `Action`
   already has `.code`, `.class_id` and `.bits`.
3. Add checks to `check_controls` in `openparkan/verify.py`:
   - every name in `Command.dsc` has a number (72/72);
   - every command named by the 275 `.man` bindings resolves;
   - the one overlapping name agrees between the two binaries.
4. Add unit tests in `tests/test_controls.py` under `TestRecoveredTables` —
   they need no game data, so assert the values directly.
5. Document it in `docs/14-controls.md`, in the section "The numbers behind
   the names".
6. Update `README.md`'s status table and `TODO.md`.

## Pitfalls, all of them paid for

- **A resolver has four shapes.** Missing one loses entries silently. All four
  are documented in `analysis/names.py`'s module docstring: the ordinary
  `mov eax, K`; a case returning the zero `strcmp` left in `eax`; a case
  writing to an **out-parameter** (`mov dword ptr [edi], 0x2ed`), which is what
  `iron3d.dll` does; and the chain's branchless last case
  (`neg eax; sbb eax, eax; and eax, M; add eax, K`).
- **A family that comes out all zeros is a bug, not a finding.** That is the
  signature of the tool latching onto pushed strings that are not compare
  cases. It happened twice — 36 `VOICE_` names and 31 `CMD_` names.
- **A linear capstone sweep over `.text` desynchronises** on jump tables and
  inline data and silently stops. Every scan must restart a byte later when
  `disasm` stalls. `analysis/pe.py`'s `xrefs_to` does **not** do this, so it
  misses references; `function_start` is also unreliable and has pointed at
  the wrong function more than once. Prefer the resilient loop in `names.py`.
- **Don't match disassembly with regular expressions where operands matter.**
  Two regex passes over the same property table disagreed with each other.
  Use capstone with `detail = True` and read `ins.operands`.
- **Check a table against the shipped files before believing it.** The scan
  codes were confirmed twice over: they are the real IBM PC set-1 numbers
  (`SCAN_A` is 30, `SCAN_ESC` is 1), and `ScanCode.dsc` lists its first 56
  entries in code order.

## Done looks like

`uv run ruff check`, `uv run pytest` and `uv run openparkan verify` all green,
with verify's count up by the checks you added; `docs/14-controls.md` carrying
the table and its provenance; and a commit message that says what was found
rather than what was edited.

## If you want more after that

The honest state of the rest is in `TODO.md`. Section 2 holds four questions
that have each survived several attempts, and section 4 the bit-level unknowns.
Two larger things are untouched: the `.scr` behaviour scripts that
`Behavior.dll` interprets, and the network protocol. Both are gameplay rather
than rendering, which the project has deliberately stayed out of so far — that
is a scope decision for the repository's owner, not for you.
