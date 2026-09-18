# The resource descriptor — how a name finds its bytes

The game never hardcodes a path to a sound, a texture or a line of dialogue.
It writes a **descriptor**, and the same six-line shape turns up in
`mission.cfg`, in `ui/*.cfg` and in `DATA/TextRes.cfg`:

```
object  briefing_sounds
 desc       = "resource"
 library    = "voices.lib"
 libtype    = "multi"
 type       = 4
 T01_T01    = t01_t01.wav
 T01_T02    = t01_t02.wav
end

object text_resources
 desc       = "resource"
 library    = "data\TextRes.dll"
 libtype    = "multi"
 type       = 6
 T01_T01    = 8
 T01_T02    = 9
end
```

`desc = "resource"` marks the object, the four keys after it describe the
library, and **every other key is a binding**: a name the rest of the game
quotes, against either a member of that library or an index into it. It is the
same `object … end` syntax `mission.cfg` already uses, so the parser was
already there ([04-missions.md](04-missions.md)); what was missing was the
reading.

**132 descriptor objects across 32 files bind 751 names, and all 751
resolve** — 507 by member name, 244 by index. Read by
`openparkan.resources`, listed by `uv run openparkan resources`.

## The seven libraries

| library | members | bound by |
|---|---:|---:|
| `voices.lib` | 234 | 37 descriptors |
| `sounds.lib` | 167 | 60 |
| `ui/minimap.lib` | 53 | 29 |
| `ui/ui_back.lib` | 32 | 1 |
| `ui/ui.lib` | 15 | 2 |
| `ui/font.lib` | 9 | 2 |
| `DATA/TextRes.dll` | 173 strings | 1 |

Six are NRes archives the readers already open. The seventh is not an archive
at all, and that is the interesting one.

## `type` — five values, one of them borrowed

`libtype` is `multi` on all 132. `type` sorts them by what the library holds —
*measured*:

| type | | descriptors | libraries |
|---:|---|---:|---|
| 1 | textures | 32 | `minimap.lib`, `ui.lib`, `ui_back.lib` |
| 2 | fonts | 2 | `font.lib` |
| 4 | sounds | 68 | `sounds.lib`, `voices.lib` |
| 5 | music | 29 | `sounds.lib` |
| 6 | text | 1 | `TextRes.dll` |

Nothing uses 3.

Types 4 and 5 both name `sounds.lib`, and the **role** separates them: all 29
type-5 descriptors are `ambient_music_loop`, the mission's theme, and type 4 is
every one-shot beside it — including all 29 `ambient_music_variation`
descriptors, so the theme's own variations are type 4 (*measured*).

**What 4 and 5 decide is the loop** — *read*. `services.dll`'s loader switches
on the `type` twice. The **object** comes from the table at `0x1000a17c`
(`0x100096e1`), where types 4, 5 and 7 share one case (`0x100097ff`, a
0x20-byte resource of kind 3, vtable `0x1003a498`) — so far the three are
alike. Each **binding** then goes through a second table, `0x1000a198`
(`0x10009909`), whose cases differ by one immediate: the flags the sound is
built with are **0** for type 4, **2** for type 5 and `0x200` for type 7
(`0x10009ab2`, `0x10009ae3`, `0x10009b14`). `Ngi32.dll` keeps that word at the
sound's `+0xc` (`0x1000e980`, masked by `0x702`) and its play shifts bit 1 out
as `IDirectSoundBuffer::Play`'s looping flag (`0x1000eb15`–`0x1000eb1c`). So
**type 5 means looping and type 4 means once**, which is exactly the theme
against its one-shots; *streamed against sampled* was the earlier **guess**
here and is withdrawn
([34-progression.md](34-progression.md#ambient-sound--read-and-measured)).
Nothing uses 7, so what its `0x200` asks for is not read.

**A repeated key keeps the later line** — *read*. Each binding is stored with
`map[name] = sound` (`0x10009b32` into the `operator[]` at `0x1000b6b0`, the
result written at `0x10009b37`), so a name written twice ends as its second
value. One shipped descriptor does it: `MISSIONS/Single.02`'s
`ambient_music_variation` writes `DAY_VARIATION1` twice, so its 8 lines bind 7
names and `atm_bees.wav` never plays there (*measured*; across the 29 missions,
171 variation lines bind 170 names, and this is the only repeat).

For the DLL the number is not the game's own. **6 is Win32 `RT_STRING`**, so a
descriptor over a PE states the Win32 resource type directly.

## `TextRes.dll` — the game's script is a string table

`DATA/TextRes.dll` is a resource-only PE. Its string table holds **173
strings** in 13 blocks, ids 8 to 193, all under language 1033 (US English).

`TextRes.cfg` names **173**, and the two sets are equal: every name has a
string and every string has a name. Nothing is orphaned in either direction.

So a line of dialogue is two hops from the file that wants it:

```
briefing.cfg   TextResID = "T01_T01"
TextRes.cfg    T01_T01   = 8
TextRes.dll    string 8  = 'Tara, The Home Base. Mission 1: "Line of fire".'
```

and its voice is two more, through `mission.cfg`'s own descriptor into
`voices.lib`. See [21-briefing.md](21-briefing.md).

Reading it needs the PE's own directories rather than an NRes table, which is
about fifty lines of structure and no dependency — cheaper than giving up on
the text or making every user of the library install a PE parser. A block
always writes sixteen lengths and the unused ones are zero, which is the file
saying *this id is not in use* rather than *this string is empty*; the reader
drops those.

The strings are UTF-16 and read as English throughout. One artefact survives
the translation: **three of the 173 carry a character above U+007F**. One is
the name *Аskold*, which kept its Russian А both times it appears; the other
two are a curly apostrophe and an ellipsis.

```
$ uv run openparkan resources --text ISFINAL
# 3 of 173 names match 'ISFINAL'
  ISFINAL_01      156  Captain, the portals have opened! The rebels are crushed…
  ISFINAL_02      157  Our forces have landed on Laks. The ruins of the Black Tower…
  ISFINAL_03      158  If you hear us, respond...
```

## What this does not say

- **Which library a type *must* have.** The five values are read off 132
  shipped descriptors; the engine may accept more.
- **Whether an index-bound name could also be name-bound.** Both forms appear,
  and no library carries both, so nothing here says the engine prefers one.
- **The other Win32 resource types.** `TextRes.dll` carries only
  `RT_STRING`; no other DLL in the installation was searched for resources.

Everything above is re-derived by `uv run openparkan verify`.
