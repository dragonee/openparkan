# The behaviour scripts — `.scr`

`MISSIONS/SCRIPTS/` holds 58 `.scr` files, from 286 bytes to 21050. They are
the mission AI, and `ai.dll` loads and runs them. This reads **all 58 end to
end with nothing left over** — 677 handlers and 6065 nodes — resolves every one
of their 9239 operands against the symbol table the game ships beside them,
and reads the interpreter that runs them: what every kind of node does, and
which of the engine's 73 script functions each call reaches.

The feasibility note called the scripts "the main obstacle" and split the
problem in two: parsing the graph is a weekend, making the nodes *behave* is
months. The first half is done, and the second is further along than that
estimate: the control flow is **read** from the executor, every call is mapped
to its handler, and the functions are named as far as their code says —
problems raised and solved, units picked and ordered, targets found. The two
helpers the target functions lean on are read as well, and one of them was being
read wrong: what a script gets back from them is a **strength held over a place**,
not a distance ([What the functions do](#what-the-functions-do)) — and a
strength is now read to its two floats, **guns over hit points** ([What a
strength is](#what-a-strength-is--read-and-measured)). So is the planner that
runs the handlers: what the two numbers a problem is raised with do, and which
of `_Start` and `_Continue` runs when ([The
planner](#the-planner-when-a-_start-runs-and-when-a-_continue--read-and-measured)). The expression
language a statement's formula is written in is read arm for arm as well, and the
corpus uses **three** of its thirteen operators ([The `.fml`
operators](#the-fml-operators--read-and-measured)). What a handler asks
of the engine below *that* is mostly not followed.

**Every claim is tagged**: *measured* is re-derived by `openparkan verify`,
*read* comes from the disassembly at the address given (all `ai.dll` unless
another module is named), *derived* follows from the two, *guess* fits and is
not established.

## The file

Three words, then one record per handler, each carrying its own node list.

```
int32   73, the length of the function table
int32   handler count
handler x count:
    int32   name length
    char    name[length]                 not terminated, not padded
    uint8   always 0
    int32   index, 0 upward in file order
    int32   node count
    node x count:
        int32   head[0]                  function id, or -1
        int32   head[1]                  destination variable, or -1
        int32   head[2]                  source variable, or a number, or -1
        int32   head[3]                  kind, -1..6
        int32   relation                 an if's comparison 0..5, else 6
        int32   operand count
        int32   operands[count]
        int32   trailer                  formula index, or -1
```

Nothing is aligned and nothing is padded: a name of 18 bytes leaves everything
after it on an odd offset, which is why a naive scan for `int32` records finds
nothing. The first word is **73** in all 58 files, the byte after a name is
**0** on all 677 records, and every handler's index is its own position.

The loader is `0x10011b20` (*read*). It reads the first word into the
interpreter and allocates that many four-byte slots, then reads handlers into
0x14-byte records and nodes into 0x20-byte ones, keeping the four head words,
the relation, the operand list and the trailer. It also reads the name's
terminating byte, which is what the `0` after a name is.

A script carries 9 to 37 handlers and 1 to 585 nodes.

## The handlers

Two kinds, and the names tell them apart.

**Nine are the engine's own**, and they are in **every one of the 58 scripts**:

| | |
|---|---|
| `Init` | set the mission up |
| `Problems0` | the AI's problem set |
| `Mission` | the mission itself |
| `Fort_Task_Complete`, `Fort_Captured` | the base |
| `Mech_GeneratorFound`, `Mech_Mineral_Found`, `Mech_Task_Complete` | the units |
| `Hero_Teleported` | the player: a hero at a main teleport's out place ([27-ownership.md](27-ownership.md#teleport-out-0x4000--read)) |

That they are universal is what makes them the engine's rather than a
mission's: a mission cannot omit one. The SuperAI constructor looks up
`Mission`, `Problems0`, `Mech_GeneratorFound` and `Fort_Task_Complete` by name,
and reports "Script error - details in ai.log" when either of the last two is
missing (`0x1000160b`, *read*); function 33 can replace `Problems0` with any
`Problems<n>`. `Init` runs once, from SuperAI slot 5. `Mission` runs from
slot 9, at most every 2 s, and the game frame calls that slot only for the
local player's clan. The problem handler runs from the clan's takt, every 7 to
8 s. See [34-progression.md](34-progression.md#when-the-mission-handler-runs--read).

Five more names are neither — `All_Defence`, in seven scripts, and
`BUILD_MINE` and the difficulty handlers `Easy`, `Normal` and `Hard`, in one
each. (An earlier note put all five in a single script each.) They are
**subroutines**: nothing calls them by name, and they are exactly the handlers
the kind-4 jump reaches (*measured*, below).

**The rest are AI problems**, prefixed `PBM_`, and they come in halves. Every
`PBM_X_Start` in a file has a matching `PBM_X_Continue` — **across all 58
files, without a single exception**. The engine is why: function 2 raises a
problem by the *name* of the variable it is given, appending `_Start` and
`_Continue` (`0x1003d770`) and looking both handlers up; if either is missing
the problem is not raised (`0x100059f0`, *read*). So a problem is written as a
pair: what to do when it is raised, and what to do while it lasts.

Fourteen problems appear across the corpus, and a script carries 0 to 13:

| problem | scripts | | problem | scripts |
|---|---:|---|---|---:|
| `PBM_BUILDING_INF_CAPTURE` | 14 | | `PBM_BUILDING_ATTACK` | 4 |
| `PBM_ROBOT_NEEDED` | 9 | | `PBM_N_OPTIMAL_TRANSPORT` | 4 |
| `PBM_BASE_DEFENCE` | 8 | | `PBM_BUILDING_NEEDED` | 2 |
| `PBM_BUILDING_PROTECT` | 7 | | `PBM_MAKE_RESEARCH` | 2 |
| `PBM_PLACE_PROTECT` | 7 | | `PBM_MINE_NEEDED` | 1 |
| `PBM_ATTACK_UNIT` | 6 | | `PBM_N_E_ENERGY` | 1 |
| `PBM_BUILDING_CAPTURE` | 6 | | `PBM_UPGRADE_NEEDED` | 1 |

## The vocabulary: `varset.var`

The 9239 operands run 0 to 228 and are never negative. They are not indices
into the script that holds them — **49 of the 58 scripts name an operand at or
past their own node count**, and the ceiling is the same 228 whether a script
holds 17 nodes or 585. So the vocabulary is shared.

It is shared in the most ordinary way imaginable. `MISSIONS/SCRIPTS/` holds one
`varset.var`, it is plain text, and it documents its own format on line one:

```
//VAR( Type, Name, DefValue, Minimum, Maximum, Comment)
//STRING( Size, Name, DefValue, Comment)

VAR( float, f0, 0)
VAR( float, f1, 1)
...
VAR( DWORD, ClanBaseX, 950);
VAR( DWORD, ClanBaseY, 1000);
VAR( DWORD, ClanID, 0);
```

**231 declarations, and the index is the position.** Every one of the 9239
operands indexes it and none falls outside. The first node of `Init` in the
first script reads 224, 225, 226 — `ClanBaseX`, `ClanBaseY`, `ClanID`, which
function 19 fills in.

Read the file in order or not at all: eleven declarations end in a stray `;`
and four of those sit in the middle, so a parser that drops them shifts every
index after it.

**Two types are declared and a third is only documented.** All 231 are `VAR`:
**200 `DWORD` and 31 `float`**, and the `STRING(...)` form the header
advertises is never used once. The interpreter holds six, and names them in a
table of its own (`0x10037688`, *read*): **0 `void`, 1 `int`, 2 `BOOL`, 3
`float`, 4 `char*`, 5 `DWORD`**. The getters agree — `0x10013190` loads a type-3
value with `fld`, a type-5 one zero-extended with `fild qword`, and tests a
type-2 one against zero. A variable is a 0x30-byte record with the tag at `+8`
and its value behind the pointer at `+0x20` (`0x10002d30`).

### Which slot reads and which writes

`head[1]` is the node's **destination**, and the corpus proves it rather than
suggesting it. All 2504 non-null values are valid indices, and **not one of
them names any of the first 23 declarations** — while the operands read those
freely. Those 23 are exactly the things you cannot assign to:

| | |
|---|---|
| `f0`–`f9` | the float literals 0 to 9 |
| `d0`–`d9` | the integer literals 0 to 9 |
| `dCurrentProblem`, `dCurrentSender`, `fDifficulty` | written by the engine ([`fDifficulty` is the game level](#fdifficulty-is-the-game-level-0-05-or-1--read)) |

That last row is confirmed from the other side: the SuperAI constructor
resolves `dCurrentProblem` and `dCurrentSender` by name right after the table
gate and keeps pointers to them at `+0x868` and `+0x86c`, along with ten more —
the six `dMax*` limits, which function 11 compares against, and four factors
(`0x10001535`, *read*). So a node reads its sources and writes its
result, and 3561 of the 6065 write nowhere at all.

The same table serves the `.fml` formula files beside the scripts: of the 15
identifiers they use, 11 are `varset.var` declarations and the other four are
the format's own keywords (`FUNCTION`, `FormulaSet`, `export`, `file`).

### Who reads a script, and what 73 means

Not `Behavior.dll`. **`ai.dll`** carries `.scr`, `.var`, `.fml` and
`MISSIONS\SCRIPTS\`; `Behavior.dll` carries `ResTree` and owns the research
tree instead.

The first word is **the length of the function table the script was compiled
against** (*read*). The SuperAI constructor allocates `word × 4` bytes, writes
73 handler addresses into them (`0x1000128a`), compares the word with 73
(`0x100014f9`) and on a mismatch prints

> `(AI.DLL) ERROR: Scripts are not up to date ! Get newer version from REL\MISSIONS\SCRIPTS`

— and carries on: the table is copied into the interpreter either way
(`0x10011e70`). An earlier note here called 73 a format version; it is that in
effect, since a script built against a shorter table would name functions that
moved, but the number itself is a count. Nothing in the corpus carries any
other value.

### What `.trf` turned out to be

A dead end for this, and worth writing down so nobody walks it twice. The 29
`.trf` archives beside the scripts are the **research tree**, which is what
`ResTree` was saying: `TRF6` is 395 part ids (`e_gun_bc_05`), `TRF7` weapon
codes (`L80mmRG`), `TRF8` display names (`Large Rail Gun`), `TRF9`
descriptions, `TRFA` stat templates for a UI panel
(`@G@Weight @B,weight,G,t,5,1@`). `TRF0` being 14720 bytes — 64 × 230, against
an operand ceiling of 228 — is a coincidence, and a good reminder that a
number landing in the right range is not evidence.

## The kind — `head[3]`

The node executor is `0x10012020`, and the first thing it does is jump on
`head[3] + 1` through an eight-entry table (`0x10012380`, *read*). So
`head[3]` is **what the node is**, and each kind carries a fixed arity —
*measured* on all 939 comparisons and **3039 of 3039** other nodes that are
not calls:

| kind | node | operands | does | nodes |
|---:|---|---:|---|---:|
| −1 | **statement** | args | a call, or `dest = variable`, or `dest = formula` | 3539 |
| 0 | **if** | 2 | compare; opens a block | 939 |
| 1 | **end** | 0 | closes the innermost block | 944 |
| 2 | **label** | 0 | nothing — a jump target | 57 |
| 3 | **goto** | 1 | continue at the node the operand indexes | 85 |
| 4 | **switch** | 1 | continue in the handler the operand indexes | 25 |
| 5 | **return** | 0 | stop | 210 |
| 6 | **constant** | 0 | `dest = head[2]`, as a number | 266 |

The fifth word, which this page used to call the opcode, is **the relation of
an `if`** (`0x1001203f`, *read*): 0 `<`, 1 `==`, 2 `>`, 3 `<=`, 4 `>=`, 5 `!=`.
All 939 comparisons are kind 0 and carry 0 to 5; all 5126 other nodes carry 6
(*measured*). The old reading — "six binary operators is the size of a
comparison set" — was right, and the arity split it rested on is the `if`
taking two operands.

| relation | 0 `<` | 1 `==` | 2 `>` | 3 `<=` | 4 `>=` | 5 `!=` |
|---|---:|---:|---:|---:|---:|---:|
| nodes | 68 | 662 | 57 | 28 | 35 | 89 |

When the first operand's type tag is 5 both are fetched as `DWORD` and compared
**unsigned** — so `ERROR`, `0xffffffff`, is greater than everything; otherwise
both are fetched as floats. The scripts never mix the two: **902 comparisons
are `DWORD` against `DWORD` and 37 float against float** (*measured*).

### A statement

**2087 statements carry `head[0]`**, and every one is a call: `head[2]` and the
trailer are null on all 2087. The executor indexes the interpreter's function
table with `head[0]` and calls the handler with nothing on the stack
(`0x100122c9`, *read*):

```
head[0]      which function: a slot of the 73-entry table
operands     its arguments, which the handler fetches itself
head[1]      where the result goes, or null
```

The other statements copy. With `head[2]` set the destination takes that
**variable** (`0x10012313`); with neither, the destination takes the value of
**formula `trailer`** (`0x10012343`, both *read*):

| statement | `head[0]` | `head[2]` | trailer | nodes |
|---|---|---|---|---:|
| call | set | — | — | 2087, 786 of them writing a destination |
| `dest = formula` | — | — | set | 1379 |
| `dest = variable` | — | set | — | 73 |

**The trailer is a formula index, not a variable** — a correction. This page
read it as a variable because all 1379 values are below 231 and the commonest
are 0, 1 and 2, which `varset.var` names `f0`, `f1`, `f2`. They are formulas 0,
1 and 2 of the script's own `.fml`, and the data says so without the binary
(*measured*):

- **58 of 58 scripts** have exactly as many `FUNCTION` lines in their `.fml` as
  statements with a trailer — **1379** of each across the corpus.
- **1379 of 1379** trailers index their file, and on every one the formula is
  the node's own line — the *n*th trailer points at line *n* — or the first
  earlier line with the identical text. The script compiler shared duplicates.

A `.fml` is `//FormulaSet export file`, a blank line, and one
`FUNCTION( , <expression>,  )` per formula, the outer fields empty on all 1379.
The expressions use numbers, `+ - * ( )` and eleven variables:
`fTemp + 0.00001`, `dPlaceProtectHits - dTemp`, `20 + 55*fDifficulty`. The
evaluator knows **13 operators** and the corpus uses **three** of them — the
expression language is read in full below ([The `.fml`
operators](#the-fml-operators--read-and-measured)). A formula's value is set
through the float setter (`0x10013650`), so it is truncated when the
destination is a `DWORD`.

### `head[2]` is a variable under one kind and a number under the other

The two readings are hard to separate for small values, because the literal
pool sits at the front of `varset.var` — `f7` is at index 7 *and* has the
value 7. The kind separates them anyway, and the executor agrees:

- **Kind −1** — all 73 values are valid indices, and only **three distinct ones
  are ever used**: `d2`, `ERROR`, `dTemp3`. A variable, copied (`0x10013a80`).
- **Kind 6** — **63 of 266 cannot be an index at all**: 55 carry the top bit
  and two are `4094` and `65534`. The rest run **densely from 1 to 28** with
  one gap. A number, written with the `DWORD` setter when the destination is a
  `DWORD` (`0x100121e2`) — and all 266 destinations are.

## The function table — *read*, and *measured* against the scripts

The SuperAI constructor writes **73 handler addresses** into the table, at
offsets `+0x0` to `+0x120` (`0x1000128a`..`0x100014fc`), all distinct. An
earlier note counted **70**, from `+0xc`: the first three stores sit before the
address it quoted. So **function *n* is slot *n***: 73 slots for the ids 0 to 72
the scripts use, and the old "70 slots cannot cover 73 ids" was the miscount.

That is checked against the data rather than asserted.
`analysis/scrtable.py` cuts each handler at the next function start and reads
which operands it fetches — the handlers are unoptimised, so an argument is
always the current node's operand pointer followed by `[pointer + 4k]` — and
compares the count with the scripts:

- on **55 of the 57** functions the scripts call, the handler reads **exactly**
  as many operands as the longest call passes. The other two: function 0's
  handler reads none of the one it is given, and function 14 reads an optional
  fourth that only a `TARGET_BY_PLACE` target uses, which no script passes;
- **38 of 38** functions whose calls write a destination have a handler that
  writes the result slot, and the 8 called handlers that never write it are
  never given one.

`openparkan.behaviour.ARGUMENTS` carries the per-handler counts and
`scrtable.py` checks them against the binary.

### How a handler runs

The handlers are methods of the **SuperAI** (vtable `0x100341b8`), the 0x8b0-byte
object `CreateSuperAI` builds per clan ([23-economy.md](23-economy.md)). The
interpreter is embedded at its `+8` and points back at it (`0x100123a0`), so a
handler's `this` is the SuperAI and the fields below are the SuperAI's
(*read*):

| SuperAI | interpreter | |
|---|---|---|
| `+0x10` | `+0x8` | function-table length |
| `+0x14` | `+0xc` | the function table |
| `+0x18` | `+0x10` | the variables |
| `+0x2c` | `+0x24` | the formulas, 0x28-byte records |
| `+0x48` | `+0x40` | the running handler |
| `+0x4c` | `+0x44` | the running node |
| `+0x50` | `+0x48` | the result slot, cleared before each call |
| `+0x54` | `+0x4c` | the condition bytes, one per open block, 32 of them |
| `+0x74` | `+0x6c` | how many blocks are open |

A handler finds its arguments through the running handler and node, and
fetches each with the getter it wants — as `DWORD` (`0x10013570`), as float
(`0x10013190`), or not at all, when it writes the variable back (functions
18, 19, 25, 35–37, 40, 47, 64, 66, 67 and 71 write an operand). It leaves its
answer in the result slot, and the executor sets the destination through the
float setter when the destination is a float and the `DWORD` setter otherwise
(`0x100122e5`). A handler returning a float leaves its bits.

### Values, one type into another — *read*

What an engine running the scripts has to reproduce, value by value:

- **The getters.** As a float (`0x10013190`): an `int` signed, a `DWORD`
  unsigned (`fild qword` of the zero-extended word), a `BOOL` 1.0 or 0.0, a
  float as it is. As a `DWORD` (`0x10013570`): an `int` or `DWORD` as it is, a
  `BOOL` 0 or 1, a float truncated toward zero by `_ftol`.
- **The setters.** From a float (`0x10013650`, typed at `0x10012c00`): an
  `int` or `DWORD` takes it truncated by `_ftol`, a `BOOL` 1 unless it compares
  equal to 0.0. From a `DWORD` (`0x10013770`, typed at `0x10012fe0`): an `int`
  or `DWORD` takes the word, a `BOOL` 0 or 1, a float the word unsigned.
- **The result slot is four bytes, written two ways.** A call's destination
  that is a float takes the slot's bits *as a float*; any other type takes the
  slot through the `DWORD` setter (`0x100122ea`). A constant goes the other way
  round: a `DWORD` destination takes it through the `DWORD` setter, any other
  its bits through the float setter (`0x100121f7`).
- **A copy is whole.** `dest = variable` is the variable record's assignment
  (`0x10013a80`): the destination takes the source's type along with its
  value.
- **An `if` on floats is the x87's.** An unordered pair passes `<`, `==` and
  `<=` and fails `>`, `>=` and `!=` (`0x1001211a`–`0x1001218b`). A relation
  outside 0..5 fails.
- **A kind outside −1..6** runs as a statement with no condition test: the
  executor's bounds check jumps past the test (`0x10012032` to `0x100122b5`).
- **A switch** leaves the running node at `0xfffffe`. The step after it makes
  `0xffffff`, which ends the handler (`0x10011ffd`), and a handler that ends on
  `0xffffff` is run again from node 0 as whichever handler is now current
  (`0x10011f2f`). So a goto to node `0xffffff` would restart its own handler.
- **The condition bytes are not bounded.** An `if` writes the byte at the
  current depth without checking it against the 32 the SuperAI reserves; the
  shipped scripts nest five deep.
- **A formula evaluates in the x87's own arithmetic**, over a stack of 4-byte
  floats, and the thirteen operators are read one arm at a time
  ([below](#the-fml-operators--read-and-measured)).

### What the functions do

Named from their code — each line is what the handler does, one call deep —
with the scripts' vocabulary where it settles a name the code leaves open
(`dFreeMindNumber = fn49()`). *p* is a problem index, *g* a group, *id* a
logical id, *clan* a clan number. "Out" marks an operand the handler writes.
Unused functions are the sixteen no shipped script calls.

**Problems.** The clan's planner. A problem is a 0x64-byte record
(`0x10004e50`): its code (`PBM_*`), three parameters, a float **weight** at
`+0x14`, a state at `+0x18` (`ST_*`), its units and its groups.

| fn | uses | arguments | does |
|---:|---:|---|---|
| 2 | 176 | code, weight, *a*, *b*, *p1*, *p2*, *p3* | raise the problem the code's variable names; not again while one with the same code, *p1* and *p2* stands (`0x10004c50`) |
| 8 | 179 | state | set the running problem's state; `ST_SOLVED` and `ST_UNSOLVED` release its units first |
| 12 | 91 | — | the running problem's weight |
| 62 | 7 | weight | set it |
| 29 | 163 | 0, 1 or 2 | the running problem's *p1*, *p2* or *p3*; `ERROR` if it has none |
| 27 | 147 | action, target kind, target | record an `ACTION_*` and its target on the running problem |
| 6 | 54 | *id*, `UNIT_NORMAL`/`_CRITICAL` | attach a unit to the running problem, marked critical or not |
| 7 | 28 | — | release the running problem's units |
| 45 | 14 | — | the running problem's units' summed strength |
| 50 | 14 | order | a unit of the running problem executing that order |
| 33 | — | *n* | run `Problems<n>` as the problem handler from now on |
| 4, 5, 20, 42, 48 | — | *p*, … | 7, 8, 27 on problem *p*; a field; a capture-capable unit |

**The weight is a priority** (*read*). Functions 13, 14 and 25 take a unit that
is busy on another problem only when that problem's weight is **below** the
running one's (`0x100089a5`), and the scripts raise a sub-problem at a hair
above their own — `fTemp = fn12()`, `fTemp = fTemp + 0.00001`, then
`fn2(PBM_ROBOT_NEEDED, fTemp, …)`. A unit reserved with function 51 (problem
slot `0xfffe`) is taken by nobody.

**Groups** live inside a problem.

| fn | uses | arguments | does |
|---:|---:|---|---|
| 25 | 85 | `TAKE_*`, [strength in/out, target kind, target…] | open a group in the running problem and fill it: `TAKE_ALL_FREE` every free unit; `TAKE_ALL_BATTLE_UNITS` every battle unit that is free or on a problem of another code; `TAKE_BY_HITS` battle units nearest the target, free ones first and then those on lighter problems, until their summed strength reaches the amount — writing back what it gathered when that falls short. `ERROR`, and no group, if it gathers nothing |
| 28 | 85 | *g*, then as function 15 | give every unit of the group the order; 1 if all took it |
| 24 | 7 | *g* | release the group |
| 21, 22, 23, 26 | — | … | release, open and add to a group |

**Units.**

| fn | uses | arguments | does |
|---:|---:|---|---|
| 15 | 236 | *id*, order, insert, parameter, four floats, target kind, target… | build an order packet and give it to the unit: result 1 taken, 0 refused, **5 no such unit**. The unit is found by logical id through the clan areal map's slot 7, as for 52, so **any clan's** unit takes it; a place's two words become the packet's float x and y (`0x10008054`, [34-progression.md](34-progression.md#what-the-scripts-ask--read-and-measured-1)). The packet is [31-packages.md](31-packages.md)'s |
| 14 | 53 | `UNIT_ANY_UNIT` type · `UNIT_ANY_CAPTURER` · `UNIT_ANY_NEAREST_CAPTURER` target kind, target | pick a unit: one of a type, a capturer, or the capturer that reaches the target soonest by distance over its live top speed (IControl 145, `0x100091b0`) |
| 13 | 2 | `UNIT_FREE_UNIT`/`_FREE_CAPTURER`/`_ANY_UNIT`, type | pick a free unit of a type, a free capturer, or one free or on a lighter problem |
| 51 | 39 | *id*, `TRUE`/`FALSE` | reserve a unit from every problem, or free it |
| 34 | 29 | type | how many entries of the running clan's own SuperAI list (`+0x8c`) have **exactly** that type and a logical id; a building is on the list (`0x10009c30`, [34-progression.md](34-progression.md#what-the-scripts-ask--read-and-measured-1)) |
| 31 | 48 | *clan*, class mask | how many of clan *clan*'s units have a type sharing a bit with the mask: the entries of that clan's SuperAI unit list (`0x10055398[clan] +0x8c`) whose logical id is set ([34-progression.md](34-progression.md)) |
| 38 | 13 | *clan*, `FREE_UNITS`/`ALL_UNITS` | the summed strength of that clan's battle units |
| 11 | 11 | type | 1 when the clan already has as many of the type as its `dMax*` variable allows, or when a per-type counter the brain keeps (`+0x3e0`..`+0x3f8`) is set |
| 49 | 9 | — | the clan's free minds |
| 39 | 1 | *id* | 1 when the id names an object |
| 52 | 47 | *id* | the object's **owner**, its slot 17: a clan's index, 65534 once destroyed; `ERROR` only when no object answers the id. The object is the system areal map's by logical id, asked through the clan areal map's slot 7 ([34-progression.md](34-progression.md#what-the-scripts-ask--read-and-measured)) |
| 61 | 7 | *id* | the object's type word, its slot 14; `ERROR` if none |
| 66 | 6 | *id*, float out | the type word, and 0.04 for a generator, 0.05 for a factory, 0 for other buildings |
| 72 | 1 | *id* | the object's property `0x201` |
| 54, 58 | — | … | a unit's current order; one of the clan's buildings |

**Places and targets.** The SuperAI keeps its clan number at `+0x7c`, its base's
centre at `+0x80`/`+0x84` and a radius at `+0x88`. *Enemy* below is an object of
another clan whose entry in the SuperAI's clan table (`+0x448`, 16 bytes a clan)
is 0; that the table holds the mission's alliance matrix, 0 towards enemies, is
a *guess*.

**What these functions measure is a strength, not a distance** (*read*), and the
float *f* is the radius it is measured over. `0x10006130` walks the areal map,
keeps every object standing within *f* of a point — the enemy's alone unless a
clan is named, in which case that clan's — and sums `(q + 0.8) × p × 1e-5` over
each (`0x1000fc70`, over the two floats an areal-map entry caches at `+0x20` and
`+0x24`; the same formula gives one unit's strength at `0x100065e0`, over
`IControl` property `0x36` and interface `0x204`'s `+4`). What those floats are
is read [below](#what-a-strength-is--read-and-measured): **guns over hit
points**. So what these functions write into their "out" operand is **how
strongly a place is held against the clan asking** — which is what a script
hands `TAKE_BY_HITS` to size the group it sends:
`dTemp3 = fn44(fT, ERROR, TARGET_BY_LOGIC_ID, dX)`, then
`fn25(TAKE_BY_HITS, dTemp3, …)`.

Six of them **pick the least**: 35, 36, 37, 40, 64 and 71 gather their
candidates, score each, and keep the one whose score is strictly below the best
so far — the least defended, not the nearest. 44 and 67 score without picking.
An earlier reading of this page called the score a distance and these functions
"the nearest"; it was wrong.

| fn | uses | arguments | does |
|---:|---:|---|---|
| 19 | 58 | x out, y out, clan out | write the base's centre and the clan's number into the three — it reads the base, it does not place it |
| 68 | 9 | range | recompute the base radius from the clan's buildings within range of the centre |
| 63 | 3 | *id* | 1 when the object stands inside the base radius |
| 44 | 21 | *f*, *clan*, target kind, target… | the strength standing within *f* of a place or an object, of that clan or the enemy's for `ERROR` |
| 18 | 2 | x out, y out | a building site spiralling out from the base, at least 250 from every object and place it knows |
| 40 | 1 | x out, y out, strength out | the unclaimed place on the list the system areal map gives (slots 25 and 26) with the least about it — a mineral site, going by its one caller, `PBM_MINE_NEEDED_Start` |
| 35 | 5 | *f*, strength out | an enemy object no `PBM_BUILDING_CAPTURE` is raised for, the least defended of its rank: a generator first, then a factory, a mine, a research centre, a storage, anything else last |
| 36 | 2 | *f*, strength out | the enemy object with the least held against it |
| 71 | 9 | *f*, type, strength out | the enemy object whose type word is **exactly** that type with the least held against it; `ERROR` when the enemy holds none |
| 64 | 3 | *f*, strength out | the enemy inside the base radius with the least about it |
| 47 | 8 | range, strength out | an enemy within range of the base that no `PBM_BASE_DEFENCE` is raised for: the hero if one is there, else the strongest |
| 37 | 3 | strength out | the clan's own building with the least held against it |
| 67 | 2 | strength out | the clan's factory, and the strength about it |
| 10, 46, 55 | — | … | an areal-map entry of another clan; the base radius; a bounds test |

**Economy.**

| fn | uses | arguments | does |
|---:|---:|---|---|
| 3 | 3 | — | the clan's power available minus power demanded — the distributor's totals the HUD reads ([23-economy.md](23-economy.md)) |
| 16 | 1 | type, resource | free space for the resource, summed over the clan's buildings of the type |
| 17 | 1 | type, resource | the resource held, summed; `RESOURCE_LEFT_ORE` asks property `0x105` |

**The mission.**

| fn | uses | arguments | does |
|---:|---:|---|---|
| 30 | 244 | kind, value | hand both to the message callback `iron3d.dll` gives `CreateSuperAI` (`iron3d.dll:0x10060ce0`), channel 0 |
| 57 | 14 | *a*, *b* | the same callback, channel 2: run `mission.cfg`'s `script`*a* as a console command ([below](#channel-2-runs-a-line-of-the-missions-script-block--read-and-measured)). *b* is never read |
| 59 | 26 | delay | the clan's seconds clock plus the delay (`+0x854`, which the constructor sets from `timeGetTime` over 1000 and each clan takt steps 7 on: [34-progression.md](34-progression.md#when-the-mission-handler-runs--read)) |
| 60 | 20 | time | 1 once that time has passed — the clock strictly above it, compared unsigned — else `ERROR` |
| 70 | 1 | *n* | a random number below *n* |
| 32 | 62 | *route*, *id* | 1 when the unit with logical id *id* was last reported inside route *route*, the system areal map's tactical areal of that id (slot 33). Read here once as two clans; 36 of 36 resolved calls pass a route id and a unit's logical id ([34-progression.md](34-progression.md)) |
| 43 | 10 | — | load the files in `UNITS\UNITS\AI\` into the object at `+0x40c`, which also keeps the place list function 40 reads |
| 41 | 2 | *id* | a test of the unit through that object; `FALSE` ends `PBM_MAKE_RESEARCH_Start` as solved |
| 65, 56 | 1, 2 | — | a flag of that object (`dLargeResearched = fn65()`); the byte at `+0x431` |
| 69 | 7 | *n* | store *n* at `+0x41c`, the design store's own `+0x10`: **how far down its ranking the AI's next build may reach** ([below](#function-69-sets-how-sloppy-the-ais-design-pick-is--read-and-measured)). Negative is ignored; the result is 1 |
| 53 | — | *clan* | an entry of that clan's place list |
| 0, 1, 9 | 6, —, 8 | | stubs: 0 sets the result to 1, 1 reads a float and drops it, 9 does nothing |

The callback's channel 0 is the mission's message switch, and its six kinds are
`varset.var`'s own (`iron3d.dll:0x10061038`, *read*): `SYSTEM_MESSAGE` with
`MISSION_FAILED` plays `VOICE_MISSION_FAIL` and with `MISSION_COMPLETE`
`VOICE_MISSION_COMPLETE`, each recording the outcome;
`OBJECTIVE_COMPLETE` fetches resource 5040 from `services.dll`'s resource
manager and plays `VOICE_OBJ_COMPLETE`; `OBJECTIVE_FAILED` fetches 5041;
`OBJECTIVE_PROGRESS` updates the objective the value numbers; `MESSAGE_INFO`
hands its value to `iron3d.dll:0x10094e30`; `CLAN_HERO_KILLED` does nothing.

## Channel 2 runs a line of the mission's `script` block — *read*, and *measured*

The callback is `cdecl` and takes three words: the channel, then two values.
Only two functions call it, and each pushes its channel as a constant —
function 30 pushes 0 (`ai.dll:0x1000c2ee`), function 57 pushes 2
(`0x1000e4f0`), both through the pointer at `0x100555e4` that `CreateSuperAI`
was given. A channel that is neither falls straight out
(`iron3d.dll:0x10060d15`–`0x10060d22`), and the whole callback is gated on a
byte of the game object, `+0xe5`, which is set when the game's mode word reads
3 (`0x1005c75a`): with that byte set no channel does anything.

**Channel 2** (`0x10060d28`) reads the **first** value only — the second word
is on the stack and never touched. It formats that value into the key
`script%d` (`0x1003c8dd`), looks the key up in the configuration object the
game keeps at `+0x48`, and hands the answer to `0x1003ca50`, which splits it on
`()` and dispatches the first token against a ten-entry table at
`0x10103bf0`. That table is **the game's debug console**, its names and its
help text intact: `truth`, `kill`, `bkill`, `cls`, `summon`, `?`, `create`,
`delete`, `bcreate`, `death` — `create` documents itself as `(<x>, <y>, <z>,
<clan number>, <datafile name>)`, `delete` as `(Logic ID)`, `death` as
`(x, y, r, delay)`.

The configuration object is the mission's own `mission.cfg`, and the whole
install says so:

- **6 of the 29 shipped `mission.cfg` files carry a non-empty `object script`
  block**, 20 lines between them: 18 `create`, one `bcreate`, one `death` —
  exactly three of the table's ten names, and nothing else.
- **The 14 calls of function 57 sit in 4 scripts, and every one names a line
  its own mission declares.** `c2m2p` and `c2m3p` pass 1–3 against Campaign 2
  Mission 02's and Mission 03's three lines, `c4m2p` passes 1–5 against
  Campaign 4 Mission 02's five, and `c5m1e` passes 1–3 against Campaign 5
  Mission 01's three. **0 of 14** name a line the mission has not got, and
  **0 of 4** of those scripts belong to a mission with no block at all.
- **Two blocks are dead.** Campaign 3 Mission 02's two lines and Multi 03's
  four are never asked for: no script of either mission calls function 57.
- Every one of the 14 calls passes the **same number twice** — `fn57(d1, d1)`,
  `fn57(d2, d2)` — which the channel's one-argument arm makes harmless.
- Control: the same corpus makes **244** calls on channel 0.

So a mission's `script` block is a list of console commands and a script fires
them by index: Campaign 4 Mission 02's `death(1246, 1051, 100, 0)` and
`bcreate(1246, 1051, 10, 0, teleport.dat, 0)` are how its Teleport objective
clears its ground and puts the Teleport there. What each of the three does is
[below](#what-the-consoles-create-bcreate-and-death-do--read-and-measured).

## What the console's `create`, `bcreate` and `death` do — *read*, and *measured*

**The line.** The dispatcher (`0x1003ca50`) cuts the line at every `(` and
`)` with `strtok`, matches the first piece against the table's names with
`_stricmp` (`0x100be600`), so without regard to case, and hands the second
piece — the text between the brackets — to the command's handler: `create`
`0x1003d280`, `delete` `0x1003d590`, `bcreate` `0x1003d6a0`, `death`
`0x1003db10`. Each handler cuts its piece again at every comma, space and `)`
(`0x10103f00`), and **does nothing unless every field it reads is there**:
six for `create` and `bcreate`, four for `death`. Every field but the file
name goes through the C library's `atol` (`0x100c1a60` → `0x100b4730`): white
space, a sign, digits up to the first that is not one. **The numbers are whole
numbers.** The last field of each — `create`'s and `bcreate`'s sixth, `death`'s
`delay` — is converted and the result dropped: nothing reads it. The shipped
lines' `0` is a placeholder, and `death`'s delay has no unit because it has no
effect. The file is looked for under `units/auto/` (`0x10103ef4`, `%s%s`): all
13 files the 19 `create` and `bcreate` lines name are in the install's
`UNITS\AUTO`, of the 23 there (*measured*).

**`create(x, y, heading, clan, file, …)` makes a unit the way the mission loader
does.** It goes through `0x10077520`, which is the mission's own unit placer
(`0x10077480`, [23-economy.md](23-economy.md)) with different inputs:
- **The clan** is the fourth field, taken as the index into the level's clan
  table, as a placed object's clan word is ([04-missions.md](04-missions.md)):
  it is handed to `CreateObjectFromScheme` and indexes the clan's SuperAI at
  `+0x774`.
- **The place** is (x, y) and, for its height, the level's probe for the highest
  landscape or building surface there (`0x100a14d0` with mask `0xa`, 0 where it
  finds none; [39-boarding.md](39-boarding.md)) **plus 2** (`0x10077562`, the
  float at `0x100e5c0c`). The mission placer adds 1 (`0x100774a7`).
- **The third field is not a height.** The help text calls it `<z>`, but
  `0x10077520` takes its cosine and sine and builds a turn about z from it,
  alongside a second angle compiled as 0 (`0x100e5d70`), so the unit's own x
  axis points to (cos *z*, sin *z*): **the third field is the heading in
  radians**, the turn a placed object's `rotation` gives
  ([04-missions.md](04-missions.md)). All 18 shipped `create`s pass 10, so
  every unit they make faces 10 rad, 213°.
- **No host building, no given id**: the vertex pair is −1, −1, and the id −1,
  so `CreateObjectFromScheme`'s bit 2 is clear and the areal map gives the next
  robot id.
- **Create flag 8 unless the game is the auto-demo** (`+0xe5`, `0x1007765a`),
  as the loader's: **the unit takes one of its clan's free minds, and with none
  free it is not made** ([23-economy.md](23-economy.md)). The handler does not
  test what it gets back; it goes on to ask the object for interface `0x10`
  (`0x1003d54e`).
- **It is filed on its clan's list at once**: the handler asks the new object
  for its id (interface `0x10` slot 12) and sends the clan's SuperAI slot 4
  event 1 with it (`0x1003d560`), the event a unit placed at the load is filed
  with ([34-progression.md](34-progression.md)). So function 31 counts it, and
  function 25's `TAKE_ALL_FREE` takes it.

All of that happens **inside the call to function 57**, before the script's
next statement: the callback is called directly. Campaign 2 Mission 02's
`Mission` handler depends on it. It runs its three `create`s and, in the same
run, completes its bonus objective once neither `Enm` nor `Enm2` has a robot
(`c2m2p` nodes 40–53); the three are `Enm2`'s first.

**`bcreate(x, y, z, clan, file, …)` makes a building standing finished.** It
builds an unturned matrix whose translation is **(x, y, z) as the fields give
them** — no probe — and hands it to the mission loader's building maker
(`0x10033cb0`, [23-economy.md](23-economy.md)) with the clan, id −1, the start
flag 0 and no filing of its own; the handler then files the building on its
clan's SuperAI as slot 4's event 2 (`0x1003dad8`). What follows from those
inputs:
- **No construction sphere.** `CreateObjectFromScheme` gives a building order 18
  only when create flag bit 1 is set (`ArealMap.dll:0x10015df3`, the same test
  `Behavior.dll`'s copy makes at `0x1001dfa3`). `0x10033cb0` sets bit 0 for the
  start flag and bit 2 for an id, never bit 1. The control is the builder's
  `CreateBuilding`, which passes 2 (`Behavior.dll:0x10029268`) and gets the
  sphere ([32-builder.md](32-builder.md)).
- **It is placed in the landscape at once.** Sent no code, a building's
  controller plans from its constructor's record to its code-0 anchor, and on
  **all 30** `fortif.rlb` building controllers the first state on that way runs
  action 20, `CLandscape::PlaceBuilding`, at the moment it is made. None runs
  action 1, which hides the building (*measured*, through the engine's planner,
  which [32-builder.md](32-builder.md#what-the-buildings-controller-does-with-the-codes--read-and-measured)
  measured against the same 30).
- **So its z does not matter.** With the start flag clear, the insertion sets the
  building down on the mean of its cut contour
  ([03-terrain.md](03-terrain.md#a-building-is-set-down-on-the-mean-of-its-contour--read-and-measured),
  [04-missions.md](04-missions.md#the-start-flag-keeps-a-building-at-its-file-height--read-and-measured)).
  Campaign 4 Mission 02's line asks for z = 10 where the ground stands at 51.1,
  and the map's lowest vertex at 49.0 (*measured*). Set down, the Teleport's
  origin comes to 47.82: its base on the contour's mean, the origin 3.3 below the
  ground, as Tut_4's placed Main Teleport stands 3.7 below its own (*measured*,
  engine).

**`death(x, y, r, delay)` fells scenery.** Its sphere is centred on the probe's
surface at (x, y) — `0x100a14d0` again, with nothing added — and has radius r.
It asks the world's slot 3 (`Terrain.dll:0x10025f40`) for the objects in it with
mask **`0x400`**, and calls each one's interface `0x16` slot 7, the life system's
kill (`Control.dll:0x1000eb70`). That is the construction sphere's kill
([32-builder.md](32-builder.md#the-construction-sphere--read-and-measured)) with
one class in its mask where the sphere's `0x414` has three. The mask is a class
set, bit *n* for class *n*: [42-selection.md](42-selection.md) reads `0xa` as
classes 1 and 3 and `0x41a` as 1, 3, 4 and 10. **So `0x400` is class 10 alone,
the scenery**: trees and stones, taken when their own sphere meets the query's,
and never a unit, a building or the hero. The kill is made in the call; the
delay is not read. *Measured*, engine: Campaign 4 Mission 02's line fells the
stone standing 20 off the site, whose sphere's radius is 277, and a tree 140
off whose radius is 88, and no other of the mission's 17 pieces of scenery.

**Where the shipped lines lead** (*measured*: each clan's minds against the
robots it is placed with):

| mission | line | clan | minds, placed robots |
|---|---|---|---:|
| C02 Mission 02 | three `create`s: `22lwhl1`, `22lwhl2` (warriors), `22ltrk1` (an HQ) | 4, `Enm2` | 5, 0 |
| C02 Mission 03 | `create` `23mfly` (only above the lowest level) | 5, `Enm2` | 5, 0 |
| | `create` `23sfly1`, `23sfly2` | 1, `enemy` | 11, 8 |
| C04 Mission 02 | `death`, `bcreate` `teleport` | 0, `Plr` | — |
| | three `create` `42_mtp` (HQs) | 1, `Enm1` | 12, 7 |
| C05 Mission 01 | `create` `51W_trans`, two `51_trans` | 0, `Player` | 6, 1 |

Every `create` finds a mind free at the start. C02 Mission 03's `enemy` is the
one clan whose factories could take them first. The two dead blocks would too:
C03 Mission 02's `Enm2` (6, 4) and Multi 03's `Plr1` (8, 2). Campaign 5
Mission 01's transformers are the **player's**, made by the enemy's script
`c5m1e`.

## Function 69 sets how sloppy the AI's design pick is — *read*, and *measured*

`+0x41c` is not a field the SuperAI reads itself but the **design store's**
`+0x10`. The store is embedded at the SuperAI's `+0x40c` — function 43 loads
`UNITS\UNITS\AI\` into it (`0x1000d561`), and functions 15 and 40 reach the
same bytes by adding `+0x7c` and then `+0x390` (`0x10008678`, `0x1000d08e`).
Its constructor clears the field (`0x10010493`) and function 69's handler is
the only writer in the module: a sweep of every memory operand in `ai.dll`'s
`.text` with that displacement returns `0x1000f0c9` and nothing else. The
handler takes the value as a `DWORD`, stores it only when it is not negative,
and leaves 1 in the result slot.

**What reads it is the design pick** (`0x100107c0`), which function 15 reaches
through `0x10010be0` when a call's target kind is `TARGET_BY_NAME`
(`0x1000867e`). The pick scores every design the store loaded, sorts the
indices by that score largest first (a selection sort at `0x10010a37`), and
then takes **not the best but the one at index
`(rand() + timeGetTime()) % (n + 1)`** (`0x10010abc`–`0x10010ae1`; `rand` is
the CRT's own at `0x1001d6d0`, `timeGetTime` the import at `0x10034128`). An
index at or past the candidate count falls back to 0, and `n = 0` — what the
constructor leaves — always takes index 0. So the count is a **spread**: how
far down its own ranking the clan's next build may fall.

### What each `SELECT_*` scores — *read*

Which field is scored is the script's `SELECT_*`. The pick's second argument is
that constant, and `mode − 1` indexes a six-arm jump table at `0x10010bbc`,
whose six targets are read here one at a time:

| mode | | arm | what it puts in the score array |
|---:|---|---|---|
| 1 | `SELECT_BEST_WEAPON` | `0x10010895` | `+0x114`, the guns |
| 2 | `SELECT_BEST_ARMOR` | `0x10010919` | `+0x110`, property 54's hit points |
| 3 | `SELECT_BEST_RANGE` | `0x100108d7` | `+0x11c`, which the store's fill never writes |
| 4 | `SELECT_FASTEST` | `0x1001095b` | `+0x118`, property 145's live top speed |
| 5 | `SELECT_BEST_COMBAT` | `0x1001099d` | `0x1000fc70(+0x110, +0x114)` — **the strength formula** |
| 6 | `SELECT_SMALLEST` | `0x100109eb` | the same strength, and then the **last** of the ranking |

An earlier reading of this page said `SELECT_BEST_COMBAT` and `SELECT_FASTEST`
"rank by hit points and by speed". Only the second half was right, and the
mistake matters: **"best combat" is `(guns + 0.8) × hit points × 1e-5`, guns
over armour**, and it is `SELECT_BEST_ARMOR` that ranks by hit points alone. On
the 59 warrior designs in `UNITS\UNITS\AI\` the difference is the whole
character of an enemy clan's force (*measured*): by hit points the top five are
all large chassis, the first of them `AI_LS_10` at 93,008 points; by the
strength formula the top three are the size-2 `23_swlk1`, `wswlk12` and
`wswlk13` at 228.6 against its 207.2 — 5,715 hit points apiece and four
thousand of guns. A clan asking for `SELECT_BEST_COMBAT` builds **small,
heavily armed** warbots, not the biggest hull it can.

`SELECT_SMALLEST`, 6, skips the draw altogether and takes the last of the
ranking (`0x10010aab`, `0x10010b65`) — the *weakest* by that same strength, not
the least chassis.

**The candidates** are every design whose `+0x108` Type **equals** the order's —
an equality, not a mask (`0x10010833`) — whose `+0x104` byte is set and whose
Type does not carry `CLASS_BUILDING` (`0x1001083d`).

**The factory-size gate is dead code** (*read*). Before the draw, `0x10006820`
walks the clan's own object list for every entry of type `BUILDING_PLANT`, asks
each for property `0x201` and keeps the **largest** — the biggest factory the
clan owns. Its answer is then thrown away: `or eax, 0xffffffff` at
`0x10010af5` overwrites `eax` before the comparison at `0x10010b0f` that would
have used it, so that comparison tests a size class against `0xffffffff`
unsigned and always passes. The pick applies no size limit; a design too big
for the factory is refused later, when `M_Task_Construct` starts
([36-factory.md](36-factory.md#production--read)).

The scripts pass the mode as the `TARGET_BY_NAME` target of
their `ORDER_BUILDING_CONSTRUCT`, out of the problem's third parameter
(`dT1 = fn29(d2)`): all **108** raises of `PBM_ROBOT_NEEDED` in the corpus pass
a `SELECT_*` there — 65 `SELECT_BEST_COMBAT`, 42 `SELECT_FASTEST`, one
`SELECT_SMALLEST`.

*Measured*: **all 7 calls sit in `Init`**, in 6 scripts, every one of them an
enemy script that also builds by name — and the value is a **difficulty knob**:

| script | what `Init` sets | `fDifficulty` 0 | 1 |
|---|---|---:|---:|
| `c2m1e` | `fn69(2)`, then `fn69(1)` inside `if fDifficulty > 0` | 2 | 1 |
| `c2m3e` | `dT = 6 − 4·fDifficulty` | 6 | 2 |
| `c3m1e` | `dT = 3 − 2·fDifficulty` | 3 | 1 |
| `c3m2e` | `dT = 6 − 6·fDifficulty` | 6 | 0 |
| `c3m2e2` | `fn69(2)` | 2 | 2 |
| `c4m2e2` | `dT = 7 − 6·fDifficulty` | 7 | 1 |

An easy game makes the clan build worse designs. **9 scripts build by name**
([23-economy.md](23-economy.md#the-bot-limit-is-the-clans-mind-count--read-and-measured));
the three that never call 69 — `c1m3e`, `c1m4e`, `scream` — keep the spread at
0 and always take the best their `SELECT_*` ranks.

**Neither function's result is ever read**: 0 of the 14 calls of 57 and 0 of
the 7 of 69 name a destination, so what 69 leaves in the result slot goes
nowhere.

## `fDifficulty` is the game level, 0, 0.5 or 1 — *read*

`varset.var` declares `fDifficulty` 0.5 and marks it *"Be careful this var
changing from CPP code"*; what writes it, and with what, is read here.
`ai.dll:0x10005d00` is the only code in the module that names the variable
(`'fDifficulty'` at `0x1003d784`, one reference, `0x10005d17`). It takes an
**index**, stores it at the SuperAI's `+0x384`, builds a six-float array on its
own stack, looks the variable up in the table by name through the interpreter's
slot 4 and writes it with the float setter (`0x10013650`):

| index | the float it writes |
|---:|---|
| 0 | 0.0 |
| 1 | 0.5 |
| 2 | 1.0 |
| 3, 4, 5 | 0.0 |

Its one caller is the SuperAI's constructor (`0x10001270`), which passes one of
`CreateSuperAI`'s own arguments (`0x10001265`, `[esp + 0x7fc]`). The index is
therefore `iron3d.dll`'s game level — `Iron_3D.ini`'s `[CS] GAME_LEVEL`, 0
easy, 1 medium, 2 hard ([22-settings.md](22-settings.md)), the same 0/1/2 the
level ratio is picked by ([26-damage.md](26-damage.md)) — which the constructor
is handed per clan; that last step is *derived*, from there being no other
0-to-2 level in the engine and from the declared default, 0.5, being exactly
this table's medium.

**So every difficulty branch in the corpus is the game level.** `fDifficulty`
appears in 25 of the 24 distinct formula lines the scripts carry
([What the corpus uses](#what-the-corpus-uses--measured)), and the three shapes
it takes on C02 M03 alone are the spread of the design draw
(`6 − 4·fDifficulty`, so 6 at easy and 2 at hard), the gate on the whole
`PBM_BASE_DEFENCE` block (`if fDifficulty > f0`, so **no base defence at all**
at easy) and `dPlaceProtectHits` (`100 − 40·fDifficulty`, and a flat 100 at
easy). Elsewhere it moves the Convoy's two raid timers by 300 and 400 seconds
([34-progression.md](34-progression.md#the-convoys-two-raids--read-and-measured)).

## What a strength is — *read*, and *measured*

The formula is `0x1000fc70`, three instructions long:

```
strength = (guns + 0.8) × hit points × 1e-5
```

Both floats are the object's own, and this page left them unnamed for a reason
worth writing down. The control system's
property interface (`Control.dll:0x1000dcc0`, 180 ids, 37 implemented) answers
neither 38 nor 54 — both fall to the default at `0x1000e002`, which returns 0
and leaves the caller's out-pointer untouched. That looked like a shipped bug
and it is not: **`LoadControlSystem` makes a derived class for every agent kind
but 9**, 0x670 bytes rather than 0x668 (`0x10032290`, constructor `0x10031490`,
vtables `0x1003d298`…), and *its* `ILifeSystem` slot 5 is a second dispatcher —
`0x1000e6c0`, ids 38 to 179, which handles the two and hands the rest down to
the base's (`0x1000e875`). Find the derived class before reading a property
table: the base's silence says nothing about what the object answers.

**Property 54 is what the object's life could be, 38 what it has** (*read*).
Both go through one helper, `Control.dll:0x100138b0`, which takes a flag:

| property | the hit points | the shield |
|---:|---|---|
| 54 (`0x36`) | `+0x58c`, the **maximum** summed over the nodes | device query 15, every deflector full |
| 38 (`0x26`) | `+0x590`, the life those nodes **have left** | device query 14, the shield as it stands |

`+0x590` is where the control system accumulates each node's life as it builds
the object (`0x1000fa42`), and `+0x58c` is a copy of it taken while everything
is whole (`0x1000fa7c`); both are rescaled together when a scale changes
(`0x10009f5d`), which is how [26-damage.md](26-damage.md) reads a node's life.
The helper also divides by `[+0x5b4]`'s `+4` when that pointer is set — it is
cleared in the constructor (`0x10007172`), never assigned anywhere in
`Control.dll`, and only the destructor frees it, so **the divide never
happens** and the branch that answers `FLT_MAX` for a zero divisor is dead.

**The other float is the unit's guns**, `IGameObject` variable `0x204`'s `+4`
(*read*). The variable getter is `MBehaviour`'s own switch
(`Behavior.dll:0x1000a490`, the one [23-economy.md](23-economy.md#how-ore-reaches-a-consumer--read-after-two-corrections)
reads the ore ids off); id `0x204` (`0x1000a7b1`) hands back `&[MBehaviour +
0x674]` and, before it does, **recomputes the second word of that pair**: it
refreshes the unit's weapon table from its machine (`0x1001c1a0`, asking every
gun for its rounds left, `0x204` id `0x700`, and its rate figure, id 6) and
then sums `a ÷ b × rounds` over it into `+0x678` (`0x1001ccb0`, skipping a gun
whose *b* is not above 0). Which two authored figures *a* and *b* are — the
gun record's `+0x0c` and `+0x28`, which that refresh does not fill — is **not
read**. So a strength is fresh on every ask, and an unarmed machine still
counts: the 0.8 is what it is worth without a gun.

**The cached form and the live one do not measure the same thing** (*read*).
The clan areal map's contact record is 40 bytes and
`ArealMap.dll:0x10006e40` fills it from four interfaces at once: the logic id,
Type and clan at `+0`/`+4`/`+8` (`IGameObject` slots 12, 14 and 17), the
position at `+0xc`, and then **`+0x18` property 54, `+0x1c` device query 6,
`+0x20` property 38, `+0x24` variable `0x204`'s `+4`**. `0x10006130` sums
`(+0x24 + 0.8) × +0x20 × 1e-5` — property **38** — while `0x100065e0`, which
fetches one object by logical id, asks property **54**. So *how strongly a
place is held against you* is measured on what its defenders have **left**,
and *what your own group is worth* on what it would have at **full**. That
also names the three numbers [31-packages.md](31-packages.md#what-it-scores-slot-13-0x1002d390-and-lets-through-slot-12-0x1002d250)
weighs in every engagement score: its *a*, *b* and *c* are the life left, the
life at full and the guns' rate, so `2 − (a + 1) ÷ (b + 1)` is 1 on a whole
target and rises towards 2 as it is shot apart.

**The design store scores a design the same way** (*read*). Its per-design fill
(`ai.dll:0x10010c30`) builds a real object from the scheme
(`ArealMap.dll:CreateObjectFromScheme`) and reads five figures off it into the
0x124-byte record: `+0x108` the Type word, `+0x10c` property `0x201` (the size
class), **`+0x110` property 54**, **`+0x114` variable `0x204`'s `+4`**, `+0x118`
property 145 (the live top speed). Those are the first two of the six floats
`SELECT_*` chooses between ([Function 69](#function-69-sets-how-sloppy-the-ais-design-pick-is--read-and-measured)),
so `SELECT_BEST_COMBAT` and `SELECT_FASTEST` rank by hit points and by speed.
The fill also computes the strength itself and drops it — the three calls
around it (`0x10010daa`, `0x10010db4`, `0x10010dcd`) are stubs that `ret`, a
trace that was compiled out.

### What a `TAKE_BY_HITS` amount is worth — *measured*

The scale is only useful with a number beside it. Summing every component's
`.ndp` durabilities over the **458** shipped assemblies gives 0 to 1,084,514
hit points, median 8,774; five carry no damage table at all (`b_ruin`,
`e_ruin`, `m_ruin`, `s_ruin`, `mas_l_n1`). All **19** hero assemblies sum
**7,362** and the largest bunker 66,010, so unarmed and whole the hero is worth
**0.0589** and that bunker **0.528**. So armour alone barely moves the number:
a `TAKE_BY_HITS` amount of 25 is 424 hero hulls' worth of it, which no group
the AI can raise will ever reach. **A two-digit amount is a demand for guns.**

Where the amount comes from, over all **74** `TAKE_BY_HITS` calls — every one
of them passing `dTemp3` (*measured*):

| the amount was last written by | calls |
|---|---:|
| `fn29(d2)`, the running problem's third parameter | 30 |
| a formula: `dBuildingProtectHits − dT2`/`− dTemp`, `dPlaceProtectHits − …`, and one 0 | 27 |
| `fn44`, the strength standing about a place | 13 |
| `fn38`, a clan's battle units' summed strength | 4 |

So **17 of the 74 are a strength the engine has just measured**, and most of
the 30 out of the problem are one at second hand: six codes pass `dTemp3` as
their third parameter (`PBM_BUILDING_CAPTURE`, `PBM_BASE_DEFENCE`,
`PBM_BUILDING_PROTECT`, `PBM_PLACE_PROTECT`, `PBM_ATTACK_UNIT`,
`PBM_BUILDING_ATTACK`), and of those 37 raises **12 hand it straight from
`fn44`** and 5 more scale what `fn44` left (`dTemp3*1.5`, `dTemp3*2.5`,
`dTemp3 + 0.5*dTemp3`), while 16 carry whatever an earlier handler left in the
variable.

Only the 27 formulas are authored numbers, and they are small. `varset.var`
declares both `*Hits` variables 2,000,000; every script overwrites them, with
0, 10, 15, 16, 25, 51, 55, 100, 150, 250, 500 or 9,999,999, twice with
`20 + 55·fDifficulty` and once with `100 − 40·fDifficulty`.

## The planner: when a `_Start` runs, and when a `_Continue` — *read*, and *measured*

A problem is a 0x64-byte record in a list at the SuperAI's `+0xa0`, and the
clan's takt (slot 3, `ai.dll:0x10001780`, every 7000 + rand % 1000 ms:
[34-progression.md](34-progression.md#when-the-mission-handler-runs--read))
walks it four times, in this order:

1. **The drain** (`0x10005910` → `0x10004ee0` per problem). Every problem with
   a code loses its **`+0x2c`** from its **`+0x24`**, and at 0 or below it is
   retired: `0x10005010` releases its units and zeroes its code, its three
   parameters and its state, which frees the slot. A test ahead of the
   subtraction, taken only while the drain is above 0, retires it the same way
   on its `+0x20` count instead (`0x10004efa`–`0x10005040`); what that count
   holds is not followed.
2. **The `_Continue` pass** (`0x10001e50`). Every problem whose state is
   `ST_SOLVING`, in list order: it writes the problem's **index** into
   `dCurrentProblem` (`0x10001ea0`) and runs handler
   `[SuperAI + code × 4 + 0x244]`.
3. **`Problems<n>`**, the handler at `+0x89c` that function 33 re-points.
4. **The `_Start` pass** (`0x10001bf0`). It takes the **largest weight** among
   the problems that are neither `ST_SOLVING` nor `ST_SOLVED`, collects every
   problem at exactly that weight into `0x10054ac8`, and for each writes its
   index into `dCurrentProblem` (`0x10001da4`) and runs handler
   `[SuperAI + code × 4 + 0xb4]`. If a handler leaves its problem neither
   solved nor solving, a flag is set and **the whole pass runs again**
   (`0x10001e3d`), so one takt drains the list from the heaviest down.

**Two things the pass's reading leaves open**, both of which an engine running
it has to answer. `fn8(ST_SOLVED)` is read as far as releasing the problem's
units; whether the record then keeps its slot is not, and it matters, because
the raise's duplicate test (`0x10004c50`) matches on code, `p1` and `p2` alone
and would block the next want for as long as a solved record stood — which
[23-economy.md](23-economy.md#the-bot-limit-is-the-clans-mind-count--read-and-measured)'s
reading of the nine build sites says does not happen. And the `_Start` pass's
repeat is read as *while any handler left its problem unstarted*: taken
literally that never ends, because `PBM_ROBOT_NEEDED_Start` returns without
setting a state whenever the clan has no factory, and the pass would offer the
same problem again every round. What the repeat excludes is not read.

**Nothing else writes `dCurrentProblem` or `dCurrentSender`** (*read*). The
SuperAI keeps pointers to the two variables at `+0x868` and `+0x86c`
([Which slot reads and which writes](#which-slot-reads-and-which-writes));
`0x10013770` is the value setter and `0x10013570` the getter. Sweeping every
memory operand at those two displacements — **ebp included, because this build
uses it as an object pointer** — returns the constructor's two stores of the
pointers themselves (`0x10001545`, `0x10001560`) and **20 loads**, of which
exactly **three reach the setter**: `dCurrentProblem` at `0x10001da4` (the
`_Start` pass) and `0x10001ea0` (the `_Continue` pass), and `dCurrentSender`
once, at `0x10005d7c`, from the argument of the event dispatcher `0x10005d70`.
The other 17 go to the getter — that is how each function handler finds the
running problem. So the three handlers that switch on `dCurrentSender`
([The jumps](#the-jumps--read-and-measured)) are reading whoever raised the
**event**, not whoever raised the problem. A first sweep for this page missed
`0x10001da4` by skipping `ebp`-based operands, and would have published one
writer where there are two.

**A raise names its own handlers** (*read*). `fn2` (`0x10009610`) reads its
seven operands and hands them to `0x100059f0` together with **the name of the
variable its first operand is**, `0x1000f6d0` — so `fn2(PBM_BASE_DEFENCE, …)`
looks for `PBM_BASE_DEFENCE_Start` and `PBM_BASE_DEFENCE_Continue` by name in
the script (`0x10011f40`, the suffixes are two strings at `0x1003d770` and
`0x1003d778`) and caches their indices in the two 100-entry tables. **If either
name is missing the raise is abandoned** (`0x10005aa1`, `0x10005aff`): no
record is made and nothing is queued. Then, before the record is pushed, the
list is searched for one with the same code, `p1` and `p2` (`0x10004c50`); a
match **refreshes the standing problem's `+0x24` by `+0x28`** (`0x10005070`)
instead of raising a second.

The record, as the constructor lays it out (`0x10004e50`):

| field | fn2's operand | what |
|---|---|---|
| `+0x00` | 0 | the `PBM_*` code |
| `+0x14` | 1 | the weight, the priority the pass ranks by |
| `+0x24`, `+0x28` | **2** | the life counter, and the value a re-raise reloads it by |
| `+0x2c` | **3** | what each takt takes off the counter |
| `+0x04`, `+0x08`, `+0x0c` | 4, 5, 6 | `p1`, `p2`, `p3`, which function 29 reads back |
| `+0x10` | — | −1 |
| `+0x18` | — | the state, `ST_NONE` |

So the two numbers a problem is raised with are **a life and a drain**, counted
in clan takts of 7–8 s. *Measured* over the **176** raises in the corpus, every
one of which passes two plain numbers:

| a / b | raises | how long the problem stands |
|---|---:|---|
| 25 / 24 | 72 | two takts |
| 5 / 2 | 37 | three takts |
| 1 / 0 | 28 | for ever |
| 15 / 1, 20 / 1, 25 / 1, 30 / 1, 3 / 1 | 26 | *a* takts, 21 s to 4 min |
| 35 / 34, 15 / 14, 14 / 13, 51 / 50 | 13 | two takts |

The commonest shape is *a* = *b* + 1, which leaves the problem exactly **two
takts** unless the script raises it again — and a raise of the same code, `p1`
and `p2` adds *a* back. So `PBM_ROBOT_NEEDED`, raised 70 times as 25/24, is a
standing request the script must keep repeating, while the 28 raises of 1/0 —
every `PBM_BUILDING_INF_CAPTURE`, `PBM_BUILDING_PROTECT` and
`PBM_PLACE_PROTECT` — never expire and only a handler's `fn8(ST_SOLVED)` ends
them.

**21 of the 176 raises are dead** (*measured*), because the script raises a
code whose handler pair it never defines: 17 of `PBM_ROBOT_NEEDED` in `c1m2e`,
`c2m4e`, `c3m4e` and `c4m2e1`, one of `PBM_BASE_DEFENCE` in `c2m1e`, and three
of `PBM_BUILDING_NEEDED` in `c3m1e`, `c3m2e` and `c3m2e2`. The name lookup
fails and the record is dropped, so four of the campaign's enemy clans ask for
warbots they will never plan for. The control is the other 155, whose pairs all
resolve, and the ten `_Start` handlers that sit in a script which never raises
their code — the mirror image, and harmless.

## Reading a script

With the kinds read, the renderer names the control flow and prints formulas
as the expressions their `.fml` gives. `openparkan behaviour c1m2e
PBM_BUILDING_INF_CAPTURE_Start`:

```
   0  dX = fn29(d0)
   1  dT = fn14(UNIT_ANY_NEAREST_CAPTURER, TARGET_BY_LOGIC_ID, dX)
   2  if dT == ERROR
   3    fTemp = fn12()
   4    fTemp = fTemp + 0.00001
   5    dTemp1 = 25
   6    dTemp2 = 24
   7    fn2(PBM_ROBOT_NEEDED, fTemp, dTemp1, dTemp2, ROBOT_BATTLEUNIT, NONE, SELECT_FASTEST)
   8    return
   9  end
  10  fn15(dT, ORDER_ROBOT_CAPTURE, INSERT_ORDER_REPLACE, NONE, f0, f0, f0, f0, TARGET_BY_LOGIC_ID, dX)
  11  fn6(dT, UNIT_NORMAL)
  12  dTemp3 = fn29(d2)
  13  dT3 = dTemp3
  14  if dTemp3 == d0
  15    goto 31
  16  end
  17  dAttackGroup = fn25(TAKE_BY_HITS, dTemp3, TARGET_BY_LOGIC_ID, dT)
  ...
  27  if dAttackGroup == ERROR
  28    goto 31
  29  end
  30  fn28(dAttackGroup, ORDER_ROBOT_PATROL, INSERT_ORDER_REPLACE, NONE, f0, f0, f0, f0, TARGET_BY_LOGIC_ID, dT)
  31  label
  32  fn27(ACTION_DESTROY, EXP_TARGET_BY_LOGIC_ID, dX)
  33  fn27(ACTION_CAPTURE_BUILDING, EXP_TARGET_BY_LOGIC_ID, dX)
  34  fn8(ST_SOLVING)
```

Read with the table: take the building this problem was raised for; pick the
capturer that gets there first; if there is none, raise `PBM_ROBOT_NEEDED` a
hair above this problem's weight and stop. Otherwise order it to capture and
attach it. If the problem's third parameter asks for an escort, gather units
by strength to that amount — and if the escort came up short, ask for battle
robots too — then patrol them round the capturer. Either way, record that the
building is to be destroyed or captured and mark the problem as being solved.
The **functions** still print as numbers, because the table index is what the
file holds; the words are the table above.

### Blocks — *read*, and *measured*

An `if` pushes its result onto the condition bytes, ANDed with the enclosing
block's, and `end` pops (`0x1001219f`, `0x100121c4`). A statement, a goto, a
switch and a return check the innermost condition before they act; **the
constant does not** — `0x100121e2` writes its number whatever the condition.
**63 of the 266 constants sit inside a block** (*measured*), so on a false
branch they still land.

That is not a reading of one arm against seven; it is what the jump table
holds. Four of its eight entries open with the **same five instructions** —
the statement's, the goto's, the switch's and the return's — which fetch the
open-block count and hand control to the do-nothing tail when the innermost
condition byte is zero. The `if`'s arm fetches both operands and compares
first. The label's has nothing to guard: it pops its frame and returns. **The
constant's is the only one that goes straight to the destination variable**
(all *read*, and *measured* by `verify` over the table's eight entries).

- **675 of 677 handlers hold exactly as many `end`s as `if`s**, and bracket
  cleanly — the depth never goes negative and ends at zero. Nesting reaches
  **five** deep, against 32 condition bytes.
- The two that do not are both `Mission` handlers, in `c2m2p.scr` and
  `c4m1p.scr`, carrying spare `end`s. The engine clamps the depth at zero
  (`0x100121cb`), so they are harmless.
- **`return` is followed immediately by `end` on 210 of 210** nodes, and the
  jumps on 109 of 110: an exit is written as the last thing in its block.

### Does any script depend on it? — *measured*

**No.** The engine writes a suppressed constant 63 times over, and not one of
those writes can be observed. Every one of the 63 lands in one of five
variables — `dTemp` 38, `dT` 14, `dT1` 7, `dX` 3, `dT2` 1 — and building the
executor's control flow faithfully (a false block runs its constants and
nothing else; a `goto`, `switch` or `return` fires only in a true one; a
`switch` is an edge into the target handler at node 0) gives **121 scenarios**,
one per (constant, enclosing `if`) pair, since some constants nest two or three
deep. Asking of each whether the value can be read before something else writes
the variable:

| | |
|---:|---|
| **88** | overwritten before any read on **every** path the script can take |
| **33** | no read at all; the handler ends and control returns to the engine |
| **0** | read |

The 33 are **17 (script, variable) pairs**, and none of them is live when the
engine next calls in: no entry point of that script — the 9 events plus every
`PBM_*_Start` and `_Continue`, the five switch-only subroutines excluded
because nothing but a `switch` reaches them — reads the variable before writing
it, **0 of 17**. Counting *every* operand of a call as a read, rather than
excluding the ones the handler writes back, leaves both numbers at 0.

The engine does not read them either. `ai.dll` carries **16 of the 231**
`varset.var` names as strings, and they are exactly the ones it resolves by
name: `dCurrentProblem`, `dCurrentSender`, `fDifficulty`, the six `dMax*`, the
four factors, `PBM_BUILDING_INF_CAPTURE`, `TRUE` and `FALSE`. None of the five
is among them, and four of the five — `dTemp`, `dT1`, `dT2` and `dX` — do not
occur in the image even as a byte sequence (*measured*).

**The control is in the check.** The same pass over the **1063** non-constant
writes inside a block — the ones the block *does* guard, asked from where that
block closes — returns **98** observable, 108 under the conservative reading.
Those are the objective flags a `Mission` handler sets inside `if dfN == d0`
and tests again further down, the tutorials' `dcl0`, and `c2m2p`'s `dTime0`:
`c1m3p`'s `Mission` writes `df1` at node 5 and reads it at node 22. So the
query can see a read where there is one, and its answer for the constants is
0 because there is none.

Two side doors are closed as well. An `if` inside a false block **does**
evaluate its comparison, so it can read a variable a suppressed constant
clobbered — but `0x1001219f` forces the byte it pushes to 0 when the enclosing
byte is 0, so the comparison cannot change control flow and the read changes
nothing. And `0x10011fb0` starts every handler at node 0 with the depth
zeroed, so there is no resume point at which a stale value could be picked up.

### The jumps — *read*, and *measured*

**Kinds 3, 4 and 5 are goto, switch and return**, and when taken each clears
every open block (`0x10012258`, `0x10012225`, `0x10012285`):

- **goto** sets the open-block count to zero (`0x1001226e`) and then the
  running node to its operand minus one, so the next node run is the operand.
  **85 of 85** gotos name a **label** in their own handler (*measured*), all
  but one forward; 52 of the 57 labels are aimed at, and the arithmetic closes
  exactly — **33 labels take two gotos and 19 take one**, and 33 × 2 + 19 = 85.
- **switch** makes its operand the running handler and starts it at node 0.
  **25 of 25** name a handler of the same script, and the ones they reach —
  `All_Defence` (21 times, from `PBM_BASE_DEFENCE_Start`), `Easy`, `Normal`,
  `Hard` (from `Mission`) and `BUILD_MINE` — are **exactly** the five handlers
  that are neither events nor problems.
- **return** stops the handler, and so does running off its end
  (`0x10011ffd`).
- **label** does nothing at all: the arm pops its frame and returns 1
  (`0x10012376`). Every one of the 57 is bare — `head = (−1, −1, −1, 2)`,
  relation 6, no operands, trailer −1 (*measured*).

**A dead end, written down.** This page once read the goto's operand as a
weight, because its commonest value, 26, is `fPry` in `varset.var`, and argued
that a problem's `_Start` returning a priority is how a planner would rank its
work. The operand is a **node index**: 26 is node 26. The switch's `d9`,
`dCurrentSender` and `dBaseFactor` are handlers 19, 21 and 30 — `All_Defence`
wherever it happens to sit — and `f3`, `f4`, `f5` are `Easy`, `Normal` and
`Hard`. The label is not "where a handler stops planning" but where its gotos
land, which is where the bookkeeping sits. There *is* a priority,
and it is the problem's weight, above.

#### Five labels are dead code, and one is not — *measured*

Five of the 57 labels no goto aims at, and one label sits inside a block. This
page used to file the two together as six anomalies. They are **five plus
one**, and the one is not an anomaly.

**The five are one label, five times over.** It is
`PBM_BUILDING_PROTECT_Continue`'s **node 19 of 20** — the handler's last —
in `c1m3e`, `c2m3e`, `c3m1e`, `c4m2e2` and `scream`, at depth 0. The
interpreter does reach it, by falling in from node 18, and reaching it does
nothing. It is an **authoring leftover rather than a code-generator artefact**,
and the corpus says so from three sides:

- those five handlers contain **no goto at all**; both their early exits are
  `return`, at nodes 3 and 15;
- `PBM_BUILDING_PROTECT` appears in **seven** scripts, and the other two —
  `c1m4e` and `c2m1e` — carry the **same handler with 19 nodes and no label**,
  identical node for node to the five's first nineteen. The only differences
  are the per-script formula indices in the five trailers, and all seven
  resolve to the same five expressions: `dBuildingProtectHits - dTemp`, `0.5`,
  `25`, `24`, `150`. One source compiled twice does not come out both ways;
- the sibling shows what a trailing label is *for*.
  `PBM_BUILDING_INF_CAPTURE_Continue` has one at node **35 of 36** in **13**
  scripts, and it is the common exit: two gotos per script aim at it, **26** in
  all. In `PBM_BUILDING_PROTECT_Continue` the two exits were written as
  `return` instead, so nothing needs to jump to the end. That count is also the
  control on the negative — asking for gotos that aim at node 19 anywhere in
  the corpus returns 0, and the same question about node 35 returns 26.

**The sixth is the ordinary case.** `c5m1p.scr`, `PBM_BASE_DEFENCE_Start`, node
9 of 11 — and **the goto at node 3 lands on it**. It is inside a block only
because the handler's whole body is one `if df2 == d0` opened at node 0.
Nothing breaks, and the binary says why: a taken goto sets the open-block count
to zero (`0x1001226e`, *read*) before setting the node, so the `end` at node 10
decrements to −1 and is clamped back to 0 (`0x100121cb`, *read*).

## The `.fml` operators — *read*, and *measured*

The expression language behind a statement's trailer has **13 operators**, and
the shipped corpus uses **three**.

### The table, and the address this page had wrong

The doc used to put the operator table at `0x10037c90`. **Nothing in `ai.dll`
holds that address**, and the copy of the table that begins eight bytes later
is dead. Enumerating every base relocation — **3788** type-3 sites — and
reading the dword at each settles it, with `0x10012380`, the executor's own
jump table, as the positive control: that comes back exactly **once**, at the
executor's `jmp` through it. The only nine pointers anywhere into
`0x10037000..0x10038a68` land in the CRT's own data at the very front of it,
below `0x100371c0`; none reaches the copy at `0x10037c98`, and none reaches the
six type-tag names at `0x10037688` either (all *measured*).

The same sweep finds what the evaluator does use: `0x10047d70`, from
`0x10015cbc` — which is the **arity field of record 0**, indexed `op × 0x110`,
so the code itself dictates the stride and the field offset. The table's first
record therefore begins at **`0x10047c70`**, and the count **13** sits in the
dword at **`0x10047c68`**. It is **byte-identical** to the `0x10037c98` copy
over all 13 records, which is an independent check on the layout.

**The layout, corrected**: `name[0x100]`, then **arity** at `+0x100`, **symbol**
at `+0x104`, a **flag** at `+0x108` and a **priority** at `+0x10c`; stride
`0x110`. The old note had the fields right and landed one record off, because
it started eight bytes before the first name — on the count. The tokeniser
walks the symbol at `0x100153e8`, the flag at `0x10015435` and the priority at
`0x100154a4`.

### What the interpreter implements — *read*

The evaluator (`0x10015b30`) is a stack machine over the parsed tokens — token
type 1 an operator, 2 a variable, 3 a literal — and dispatches through a
13-entry jump table at **`0x10016064`**. A binary operator takes the right-hand
operand from one slot of the evaluator's frame and the left from another; a
unary one reads only the left, which is how the arity column is confirmed a
second way — **arms 0 to 6 read both slots and arms 7 to 12 only one**. Its
constants are 1.0, 0.0, −1.0 and 0.5 (`0x1003470c`..`0x10034718`), `0x1001df70`
is `_ftol` and `0x10021030` the CRT's `pow`.

| # | name | sym | arity | flag | prio | computes |
|---:|---|:--:|:--:|:--:|--:|---|
| 0 | Addition | `+` | 2 | 0 | 1 | `l + r` |
| 1 | Subtraction | `-` | 2 | 0 | 1 | `l − r` |
| 2 | Multiplication | `*` | 2 | 0 | 2 | `l × r` |
| 3 | Division | `/` | 2 | 0 | 2 | `l / r` — but it compares `r` with 0.0 first and **answers 0.0 when `r` is zero**, never dividing |
| 4 | Power | `^` | 2 | 0 | 3 | `pow(l, r)` through the CRT |
| 5 | And | `&` | 2 | 0 | 2 | 1.0 when `_ftol(l)` and `_ftol(r)` are both non-zero, else 0.0 |
| 6 | Or | `\|` | 2 | 0 | 1 | 1.0 when either is, else 0.0 |
| 7 | Sign change | `-` | 1 | 0 | 1 | negate |
| 8 | Not | `!` | 1 | 0 | 1 | 1.0 when `_ftol(x)` is 0, else 0.0 |
| 9 | Normalisator | `N` | 1 | 1 | 100 | 0.0 below −1, `(x + 1) × 0.5` on [−1, 1], 1.0 above 1 |
| 10 | Significator | `S` | 1 | 1 | 100 | `x` when `x > 0`, else 0.0 |
| 11 | Booleanisator | `B` | 1 | 1 | 100 | 1.0 when `x > 0`, else 0.0 |
| 12 | Absolute | `A` | 1 | 1 | 100 | `\|x\|` |

The three that truncate first are worth the emphasis: `0.5 & 1` is **0**, and
`!0.5` is **1**, because each value goes through `_ftol` before it is tested.
Every comparison in the table is the x87's, so an unordered operand falls to
the arm's other branch — a NaN is not greater than 1, so `N` gives 0.0 for one.

Two more things the language has and the corpus does not touch. The tokeniser
knows the literals **`TRUE`** and **`FALSE`** without the symbol table
(`0x10048a40`, `0x10048a48`, used at `0x10015973` and `0x100159a6`;
`varset.var` happens to declare them 1 and 0 as well, so nothing turns on it).
And it rewrites operator 1 into operator 7 — binary `-` into Sign change — when
a state word it carries is zero (`0x1001541e`–`0x10015429`), which is how the
unary minus is recognised.

**What is still not read** is the parser above the arms: the flagged four take
a branch of their own (`0x1001543b`) instead of going through the precedence
stack, which is what priority 100 says, and how that branch differs in detail
was not followed.

### What the corpus uses — *measured*

All **1379** formulas, **198** distinct expressions. The only non-alphanumeric
characters that appear inside an expression are `+` 106, `-` 50, `*` 29 and
five bracket pairs. **50 of 50** minus signs have a left operand, so every one
is Subtraction and **Sign change is never used**. The 11 identifiers are all
`varset.var` declarations — `fTemp`, `fT`, `fDifficulty`, `dTemp`, `dTemp3`,
`dT`, `dT2`, `dTime0`, `dBuildingProtectHits`, `dPlaceProtectHits`, `df5` —
and neither `TRUE` nor `FALSE` appears. The five bracket pairs group nothing:
`( df5 + 4 )`, `( df5 + 2 )`, `( df5 - 3 )`. So the corpus uses **3 of the 13**,
at priorities 1 and 2 only. The commonest are `fTemp + 0.00001` (44),
`fTemp + 0.001` (23), `fTemp + 0.0001` (22), the protect-hit differences (28
lines across four forms) and the difficulty ramps in the shape
`20 + 55*fDifficulty` (25 lines, 24 of them distinct).

**The control on a negative** is that the scan can see the characters it says
are absent. The raw `.fml` files hold `/` **116** times — 58 files × the two
slashes of `//FormulaSet export file` — and the census counts every one, with
zero inside an expression. The arithmetic closes on both sides: 1384 `(` and
1384 `)` against 1379 `FUNCTION(…)` wrappers plus the 5 real pairs, 2758 `,`
against 1379 × 2, and no other non-alphanumeric byte anywhere. A `/`, `^`, `&`,
`|`, `!`, `N`, `S`, `B` or `A` in any of the 1379 lines would have been
counted.

So **ten of the thirteen operators are never exercised by a shipped mission**.
An engine can implement them from the arms above — [the
engine](../engine/crates/parkan-sim/src/script.rs) does — but play would not
show a wrong one.

## The top bit is `CLASS_BUILDING` — *measured*, and *read*

The 55 constants with the top bit set are **logical ids of buildings**.

- `varset.var` declares `CLASS_BUILDING` as `0x80000000`, and **all 14
  `BUILDING_*` types carry it** while none of the 11 robot, resource and class
  words does. Function 31 tests `type & mask`, so `CLASS_ROBOT` counts every
  robot type (*read*, `0x1000c42a`).
- A mission gives the bit to **568 of 568** placed objects that are not units —
  buildings, vegetation, rocks — and to **0 of 296** units, in their
  `LogicalID` ([04-missions.md](04-missions.md)).
- **47 of 48** flagged constants in scripts a mission names are the logical id
  of a building that mission places, against **39%** of missions at large; 7
  more sit in scripts no mission names. The one miss is `c2m4e.scr`'s
  `0x80000008` on `Mission.04`.
- The engine tests the bit on logical ids to mean *building*: the areal map's
  slot 8 answers only an id that carries it (`ArealMap.dll:0x10001a57`),
  function 37 masks it to pick the clan's buildings (`0x1000cf61`), and the base
  radius counts only ids with it (`0x10006757`, all *read*).

The constants flow into what takes a logical id — function 52 (33 calls),
function 15's target (27), and functions 44, 2 and 72 (5). The renderer
prints them `CLASS_BUILDING|3`.

## 65534 is a destroyed object's owner — *read*, and *measured*

Function 52 answers **an object's slot 17**, and slot 17 is its owner word:
`ai.dll` compares it with the clan brain's own clan number (`0x100052b9`) and,
elsewhere, with `0xfffe` (`0x1000698d`, *read*). `0xfffe` is what a destroyed
object's owner word is set to ([26-damage.md](26-damage.md)).

The scripts use it that way. Seven player scripts set a variable to 65534,
and **11 of 11** comparisons against it test function 52's answer
(*measured*) — just before `OBJECTIVE_FAILED` and `SYSTEM_MESSAGE,
MISSION_FAILED`. `c1m3p.scr` has the same three blocks but sets the variable to
**4094**, `0xffe`: an owner word never equals it, so its three "building
destroyed" failures can never fire (*derived*; that 4094 is a slip for 65534 is
a *guess*). `c3m3p.scr` sets 65534 and never compares it.

The scripts read two other answers beside it: 0, the player's clan
(`PLAYER_CLAN`), ticks the objective, and 1 marks it in progress.

**A dead unit answers 65534 only until it is deleted** (*derived*). The game
deletes it its controller's `+92` ms after it dies
([26-damage.md](26-damage.md)), and then no object answers its id: function 52
gives `ERROR`. A building is never deleted, so its shell answers 65534 for good.
`c2m1p.scr` ticks *The Iron Monster*'s third objective, the enemy's heavy
warbot, on `ERROR` for logical id 22.

## Naming the functions from the binary

The first pass concluded the binary would not name them: the handlers carry no
strings and index structures with no names. Two things were in the way, and
neither was the binary.

- **The table was miscounted**, so there seemed to be no mapping to find. With
  73 slots the id is the slot, and the operand counts line up 55 of 57.
- **`coverage.py` fused the handlers.** They have no padding and no direct
  caller, so `0x10007fd0`..`0x1000f4d7` came out as two functions of 30 KB, and
  any site read inside one marked the lot read. It now believes a run of six
  or more code addresses stored by code, as it already believed one stored in
  data; that adds exactly the 72 handlers it lacked and two comparators in
  `Ngi32.dll`, and nothing else.

After that the handlers are short and their helpers shorter, and the SuperAI
names its own fields: the constructor links `dMaxBuilder` into `+0x870`, so the
handler that reads `+0x870` for `ROBOT_BUILDER` is checking a limit. What is
not followed is the engine side of the object calls — slot 17 is an owner,
property `0x201` is 1 or 2 on a unit functions 13, 14 and 48 count as a
capturer — which is why the table stops one call deep.

## What is not read here

- **What the helpers below the handlers compute.** ~~A unit's *strength*
  (`0x100065e0`), a distance (`0x10006130`)~~ are now read, and the second was
  not a distance at all: both are the strength formula at `0x1000fc70`, and
  `0x10006130` sums it over a radius ([What the functions
  do](#what-the-functions-do)). ~~What the two floats that formula multiplies
  are — `IControl` property `0x36` and interface `0x204`'s `+4` — and so what a
  strength is worth in the numbers the scripts compare it against~~ is now read
  and measured: **guns over hit points**, where 54 is the life an object could
  have and 38 the life it has, the guns are the behaviour's own recomputed
  total, and a whole hero is worth 0.0589 ([What a strength
  is](#what-a-strength-is--read-and-measured)). What is still not read is which
  two authored gun figures the total divides (`Behavior.dll:0x1001ccb0`'s
  `+0x0c` and `+0x28`). The problem's action record (`+0x34`) and the object at
  `+0x40c` behind functions 40, 41, 43, 53 and 65 are not read either. The table
  says what each handler does with them. ~~The areal-map list function 32 tests~~ is now read:
  a route's list of the units last reported inside it
  ([34-progression.md](34-progression.md)).
- ~~**The two numbers a problem is raised with** — `fn2`'s third and fourth
  arguments, kept at `+0x24` and `+0x2c` (`25` and `24` above) — and what the
  engine does with a problem once raised: which problem handler runs when, and
  who writes `dCurrentProblem` and `dCurrentSender`.~~ All read. The two are a
  **life counter and a drain**, in clan takts of 7–8 s; `_Continue` runs for a
  problem in `ST_SOLVING` and `_Start` for the heaviest that is not, repeated
  until none is left to start; `dCurrentProblem` is written by those two passes
  alone (`0x10001da4`, `0x10001ea0`) and `dCurrentSender` by the event
  dispatcher (`0x10005d7c`) ([The planner](#the-planner-when-a-_start-runs-and-when-a-_continue--read-and-measured)).
  When `Init`, `Mission` and `Problems<n>` run is read in
  [34-progression.md](34-progression.md).
- ~~**Channel 2 of the message callback** (function 57), and the `+0x41c` count
  function 69 stores.~~ Both are now read. Channel 2 runs `mission.cfg`'s
  `script`*a* as a debug-console command, and the 14 calls name only lines
  their own missions declare ([Channel 2 runs a
  line](#channel-2-runs-a-line-of-the-missions-script-block--read-and-measured)),
  and what the three commands they ship do is read too ([What the console's
  `create`, `bcreate` and `death`
  do](#what-the-consoles-create-bcreate-and-death-do--read-and-measured));
  `+0x41c` is the design store's spread, how far below the best the AI's next
  build may fall, and the 7 calls set it from `fDifficulty` ([Function
  69](#function-69-sets-how-sloppy-the-ais-design-pick-is--read-and-measured)).
  What `MESSAGE_INFO`'s value selects in `iron3d.dll` was already
  read: a `messages.cfg` id, played as [34-progression.md](34-progression.md)
  describes. ~~Still not read: what the design store **scores** — the six floats
  at the design record's `+0x110` upward that `SELECT_*` chooses between.~~ All
  six arms are now read ([What each `SELECT_*`
  scores](#what-each-select_-scores--read)), and the first reading of two of
  them here was wrong: `SELECT_BEST_COMBAT` is the strength formula, guns over
  armour, not hit points, and `SELECT_SMALLEST` takes the weakest by that same
  figure rather than the least chassis. What is still unread is the one float
  `SELECT_BEST_RANGE` reads, `+0x11c`, which the store's own fill never writes,
  and which no shipped raise asks for.
- ~~**What writes `fDifficulty`.**~~ Read: the SuperAI's constructor, from a
  six-float table indexed by the game level ([`fDifficulty` is the game
  level](#fdifficulty-is-the-game-level-0-05-or-1--read)).
- ~~**Whether any script depends on a constant landing inside a false block.**~~
  Now answered: **no**, over all 121 (constant, enclosing `if`) pairs, with a
  control that returns 98 on the writes the block does guard ([Does any script
  depend on it?](#does-any-script-depend-on-it--measured)).
- ~~**Five labels no goto aims at**, and why one label sits inside a block.~~
  Now read: the five are one dead label repeated, and the sixth is an ordinary
  common exit ([Five labels are dead
  code](#five-labels-are-dead-code-and-one-is-not--measured)).
- ~~**`.fml` operators the corpus never uses** — beyond their names and
  arities.~~ All thirteen arms are now read ([The `.fml`
  operators](#the-fml-operators--read-and-measured)). What is still not
  followed is the **parser** above them: the branch at `0x1001543b` that the
  four priority-100 operators take instead of the precedence stack.
- **`.trf`**, the research tree — identified above, and read in
  [16-research.md](16-research.md).
