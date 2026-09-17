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
problems raised and solved, units picked and ordered, targets found. What a
handler asks of the engine below it (a unit's strength, a distance) is
mostly not followed.

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
| `dCurrentProblem`, `dCurrentSender`, `fDifficulty` | written by the engine |

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
evaluator knows more than that — a table of **13 operators** (`0x10037c90`,
*read*) names `Addition`, `Subtraction`, `Multiplication`, `Division`, `Power`,
`And`, `Or`, `Sign change`, `Not`, `Normalisator`, `Significator`,
`Booleanisator` and `Absolute`. A formula's value is set through the float
setter (`0x10013650`), so it is truncated when the destination is a `DWORD`.

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
- **The formula evaluator's operator table** (`0x10037c90`) is 13 records of a
  256-byte name, then arity, symbol, a flag and a priority: `+` `-` `|` 1,
  `*` `/` `&` 2, `^` 3, the unary `-` and `!` 1, and the one-letter `N`, `S`,
  `B`, `A` 100, flagged. How the parser uses the priorities, and what the
  flagged four compute, is not read.

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

**Places and targets.** The SuperAI keeps its base's centre at `+0x80`/`+0x84`
and a radius at `+0x88`. *Enemy* below is an object of another clan whose entry
in the SuperAI's clan table (`+0x448`, 16 bytes a clan) is 0; that the table
holds the mission's alliance matrix, 0 towards enemies, is a *guess*. The
distances come from `0x10006130`, which takes the float *f* as well; what *f*
changes is not read.

| fn | uses | arguments | does |
|---:|---:|---|---|
| 19 | 58 | x out, y out, clan out | write the base's centre and the clan's number into the three — it reads the base, it does not place it |
| 68 | 9 | range | recompute the base radius from the clan's buildings within range of the centre |
| 63 | 3 | *id* | 1 when the object stands inside the base radius |
| 44 | 21 | *f*, *clan*, target kind, target… | a distance to a place or an object |
| 18 | 2 | x out, y out | a building site spiralling out from the base, at least 250 from every object and place it knows |
| 40 | 1 | x out, y out, distance out | the nearest unclaimed place on the list the system areal map gives (slots 25 and 26) — a mineral site, going by its one caller, `PBM_MINE_NEEDED_Start` |
| 35 | 5 | *f*, distance out | an enemy object no `PBM_BUILDING_CAPTURE` is raised for: a generator first, then a factory, a mine, a research centre, a storage, anything else last |
| 36 | 2 | *f*, distance out | the nearest enemy object |
| 71 | 9 | *f*, type, distance out | the nearest enemy object of a type |
| 64 | 3 | *f*, distance out | the nearest enemy inside the base radius |
| 47 | 8 | range, strength out | an enemy within range of the base that no `PBM_BASE_DEFENCE` is raised for: the hero if one is there, else the strongest |
| 37 | 3 | distance out | the nearest of the clan's own buildings |
| 67 | 2 | distance out | the clan's nearest factory |
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
| 57 | 14 | *a*, *b* | the same callback, channel 2 |
| 59 | 26 | delay | now plus the delay, in whole seconds (`+0x854`, which the constructor sets from `timeGetTime` over 1000) |
| 60 | 20 | time | 1 once that time has passed, else `ERROR` |
| 70 | 1 | *n* | a random number below *n* |
| 32 | 62 | *route*, *id* | 1 when the unit with logical id *id* was last reported inside route *route*, the system areal map's tactical areal of that id (slot 33). Read here once as two clans; 36 of 36 resolved calls pass a route id and a unit's logical id ([34-progression.md](34-progression.md)) |
| 43 | 10 | — | load the files in `UNITS\UNITS\AI\` into the object at `+0x40c`, which also keeps the place list function 40 reads |
| 41 | 2 | *id* | a test of the unit through that object; `FALSE` ends `PBM_MAKE_RESEARCH_Start` as solved |
| 65, 56 | 1, 2 | — | a flag of that object (`dLargeResearched = fn65()`); the byte at `+0x431` |
| 69 | 7 | *n* | store *n* at `+0x41c` |
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

- **675 of 677 handlers hold exactly as many `end`s as `if`s**, and bracket
  cleanly — the depth never goes negative and ends at zero. Nesting reaches
  **five** deep, against 32 condition bytes.
- The two that do not are both `Mission` handlers, in `c2m2p.scr` and
  `c4m1p.scr`, carrying spare `end`s. The engine clamps the depth at zero
  (`0x100121cb`), so they are harmless.
- **`return` is followed immediately by `end` on 210 of 210** nodes, and the
  jumps on 109 of 110: an exit is written as the last thing in its block.

### The jumps — *read*, and *measured*

**Kinds 3, 4 and 5 are goto, switch and return**, and when taken each clears
every open block (`0x10012258`, `0x10012225`, `0x10012285`):

- **goto** sets the running node to its operand minus one, so the next node
  run is the operand. **85 of 85** gotos name a **label** in their own handler
  (*measured*), all but one forward; 52 of the 57 labels are aimed at and one
  sits inside a block (`c5m1p.scr`, `PBM_BASE_DEFENCE_Start`).
- **switch** makes its operand the running handler and starts it at node 0.
  **25 of 25** name a handler of the same script, and the ones they reach —
  `All_Defence` (21 times, from `PBM_BASE_DEFENCE_Start`), `Easy`, `Normal`,
  `Hard` (from `Mission`) and `BUILD_MINE` — are **exactly** the five handlers
  that are neither events nor problems.
- **return** stops the handler, and so does running off its end
  (`0x10011ffd`).
- **label** does nothing at all.

**A dead end, written down.** This page once read the goto's operand as a
weight, because its commonest value, 26, is `fPry` in `varset.var`, and argued
that a problem's `_Start` returning a priority is how a planner would rank its
work. The operand is a **node index**: 26 is node 26. The switch's `d9`,
`dCurrentSender` and `dBaseFactor` are handlers 19, 21 and 30 — `All_Defence`
wherever it happens to sit — and `f3`, `f4`, `f5` are `Easy`, `Normal` and
`Hard`. The label is not "where a handler stops planning" but where its gotos
land, which is where the bookkeeping sits. There *is* a priority,
and it is the problem's weight, above.

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

- **What the helpers below the handlers compute.** A unit's *strength*
  (`0x100065e0`: IControl property `0x36` and object property `0x204`), a
  distance (`0x10006130`), the problem's action record (`+0x34`), the object at
  `+0x40c` behind functions 40, 41, 43, 53 and 65. The table says what each
  handler does with them. ~~The areal-map list function 32 tests~~ is now read:
  a route's list of the units last reported inside it
  ([34-progression.md](34-progression.md)).
- **The two numbers a problem is raised with** — `fn2`'s third and fourth
  arguments, kept at `+0x24` and `+0x2c` (`25` and `24` above) — and what the
  engine does with a problem once raised: which problem handler runs when, and
  who writes `dCurrentProblem` and `dCurrentSender`. When `Init`, `Mission` and
  `Problems<n>` run is read in [34-progression.md](34-progression.md).
- **Channel 2 of the message callback** (function 57), and the `+0x41c` count
  function 69 stores. What `MESSAGE_INFO`'s value selects in `iron3d.dll` is
  read: a `messages.cfg` id, played as [34-progression.md](34-progression.md)
  describes.
- **Whether any script depends on a constant landing inside a false block.**
  The engine does it 63 times over; the scripts may overwrite the variable
  before reading it every time.
- **Five labels no goto aims at**, and why one label sits inside a block.
- **`.fml` operators the corpus never uses** — `Division`, `Power`, `And`,
  `Or`, `Not`, `Sign change` and the one-letter `N`, `S`, `B` and `A` —
  beyond their names and arities.
- **`.trf`**, the research tree — identified above, and read in
  [16-research.md](16-research.md).
