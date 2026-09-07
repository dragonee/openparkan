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
