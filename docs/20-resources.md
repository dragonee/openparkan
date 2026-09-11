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
type-5 descriptors are `ambient_music_loop`, the mission's looping theme, and
type 4 is every one-shot beside it. That reads as *streamed against sampled* —
a **guess**; what is measured is only that the split is exact.

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
