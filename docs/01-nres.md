# NRes — the container format

Every archive Parkan: Iron Strategy ships is the same container: `*.rlb`,
`*.lib`, `*.dlb`, `*.res`, the per-mission `*.trf` scripts, and the per-map
`Land.msh` / `Land.map`. 116 files in a Steam install, 6697 members in total.

Two files are *not* NRes: `gamefont.rlb` and `sprites.lib` — see
[06-open-questions.md](06-open-questions.md).

## Layout

```
offset  size  field
0x00       4  magic, always the ASCII bytes 'NRes'
0x04       4  uint32 version, always 0x00000100
0x08       4  uint32 member count
0x0C       4  uint32 total file size (matches the file exactly, always)
0x10     ...  member payloads, each padded to an 8-byte boundary
   (end)      directory: member count x 64-byte records
```

The directory sits at the very end, so it is found at
`filesize - count * 64`. Member payloads occupy everything between the header
and the directory.

## Directory record (64 bytes)

```
offset  size  field
0x00       4  type. Either a FourCC tag ('Texm', 'SWAV', 'MESH', 'CTLD', ...)
              or, in per-map files, a small little-endian integer used as a
              stream selector
0x04       8  zero in every shipped file
0x0C       4  uint32 payload size in bytes
0x10       4  uint32, always 1
0x14      32  name, NUL-padded ASCII
0x34       4  zero in every shipped file
0x38       4  uint32 payload offset from the start of the file
0x3C       4  uint32 index — a member ordinal that is *not* the directory
              position and *not* the file order
```

## Things worth knowing

**Alignment.** Every member offset in all 116 archives is a multiple of 8, and
the 1–7 byte gaps between members are zero-filled. Any writer has to reproduce
this padding to round-trip a file.

**The `index` field is not the directory position.** In `system.rlb` the
directory order is `0, 5, 6, 7, 8, 9, 10, 11, 12, 16, 1, 4, 15, 13, 14, 2, 3`.
Treat it as an opaque id the engine uses to cross-reference members; do not
assume it matches iteration order.

**Names are not unique.** In `Land.msh` every member is called `Land` and only
the numeric type distinguishes them. Look members up by type there, by name
elsewhere. `NResArchive` offers both (`find`, `one_of_type`).

**Case is inconsistent.** `Mistips.mis` and `mistips.mis` both occur across
mission directories, as do mixed-case member names. Look up case-insensitively.

## Type tags seen in the shipped data

| Tag | Where | Contents |
|---|---|---|
| `Texm` | `Textures.lib`, `ui/*.lib` | textures ([02-texm.md](02-texm.md)) |
| `SWAV` | `sounds.lib`, `voices.lib` | RIFF/WAVE, uncompressed |
| `MESH` | `static.rlb`, `system.rlb` | object geometry |
| `CTLD` | `static.rlb`, `system.rlb` | `.ctl` controller data |
| `EXPL` | `system.rlb` | explosion definitions |
| `WEAR`, `NDPR`, `CTPT` | `static.rlb` | per-object tables |
| `BULL`, `WPNS` | `objects.rlb` | projectile and weapon records |
| `INTO`, `EXTO`, `STAT`, `FORT`, `BTLU` | `objects.rlb` | object class records |
| `SUND`, `SUNO` | `system.rlb`, `objects.rlb` | sun / lighting |
| `TRF0`–`TRFB` | `MISSIONS/SCRIPTS/*.trf` | behaviour graph streams |

## Verified by

`uv run openparkan verify`, checks 1–4: header size agreement, 8-byte alignment,
non-overlapping ranges, zero-filled padding — over all 116 archives.
