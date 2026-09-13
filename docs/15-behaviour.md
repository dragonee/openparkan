# The behaviour scripts — `.scr`

`MISSIONS/SCRIPTS/` holds 58 `.scr` files, from 286 bytes to 21050. They are
the mission AI, and `ai.dll` loads them. This reads **all 58 end to end with
nothing left over** — 677 handlers and 6065 nodes — and resolves every one of
their 9239 operands against the symbol table the game ships beside them.

Before this they were the project's largest unread format. The feasibility
note called them "the main obstacle" and split the problem in two: parsing the
graph is a weekend, making the nodes *behave* is months. That split is real,
and this document is the first half. **What a node does is not read here** —
but what it reads and what it writes are, by name.

## The file

Three words, then one record per handler, each carrying its own node list.

```
int32   magic, always 73
int32   handler count
handler x count:
    int32   name length
    char    name[length]                 not terminated, not padded
    uint8   always 0
    int32   index, 0 upward in file order
    int32   node count
    node x count:
        int32   head[4]                  four reference fields, meanings open
        int32   opcode                   0..6
        int32   operand count
        int32   operands[count]
        int32   trailer
```

Nothing is aligned and nothing is padded: a name of 18 bytes leaves everything
after it on an odd offset, which is why a naive scan for `int32` records finds
nothing. The magic is **73** in all 58 files, the byte after a name is **0** on
all 677 records, and every handler's index is its own position.

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
| `Hero_Teleported` | the player |

That they are universal is what makes them the engine's rather than a
mission's: a mission cannot omit one. Five more names appear in a single
script each — `All_Defence`, `BUILD_MINE`, and the difficulty handlers `Easy`,
`Normal` and `Hard`.

**The rest are AI problems**, prefixed `PBM_`, and they come in halves. Every
`PBM_X_Start` in a file has a matching `PBM_X_Continue` — **across all 58
files, without a single exception**. So a problem is written as a pair: what
to do when it is raised, and what to do while it lasts.

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

They read as a strategy layer's vocabulary — capture a building, defend the
base, need a robot, route transport, make research.

## The node, and the one thing its shape proves

A node is four head fields, an opcode, a counted operand list and a trailer.
The opcode runs 0 to 6, and **its arity is fixed by its value**:

| opcode | nodes | operands |
|---:|---:|---|
| 0 | 68 | exactly 2 |
| 1 | 662 | exactly 2 |
| 2 | 57 | exactly 2 |
| 3 | 28 | exactly 2 |
| 4 | 35 | exactly 2 |
| 5 | 89 | exactly 2 |
| 6 | 5126 | 0 to 11 |

**939 nodes on opcodes 0–5 take two operands and not one takes any other
number.** That is not a coincidence of the corpus; it is the shape of six
fixed-arity operators against one variadic form, and it is the strongest
structural claim this document makes. Six binary operators is the size of a
comparison set, and opcode 6 — five sixths of all nodes — is what a call or a
sequence looks like. Both readings are *unconfirmed*, and the reader names
neither: it exposes `opcode` and `binary` and stops there.

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
first script reads 224, 225, 226 — `ClanBaseX`, `ClanBaseY`, `ClanID`, which is
what setting a mission up looks like.

Read the file in order or not at all: eleven declarations end in a stray `;`
and four of those sit in the middle, so a parser that drops them shifts every
index after it.

**Two types are declared and a third is only documented.** All 231 are `VAR`:
**200 `DWORD` and 31 `float`**, and the `STRING(...)` form the header
advertises is never used once. The engine is readier than the data: `ai.dll`
carries a value formatter that switches six ways on a type tag — a string
copy, decimal through `itoa`, a hex form that writes its own `0x`, a boolean
test and two more — and fourteen near-identical methods switch on the same
tag. So the script vocabulary uses two of at least six types the interpreter
can hold. Which tag is which type is not established.

A negative worth keeping, now checked exhaustively rather than by search:
**there is no 73-entry switch in `ai.dll`.** Enumerating every jump table in
the binary — 72 of them — the widest has **13 entries**, so the 73 function
ids are not dispatched by a switch anywhere. The 70-entry handler table the
loader builds remains the only dispatch, and the id-to-slot mapping stays
open.

### Which slot reads and which writes

`head[1]` is the node's **destination**, and the corpus proves it rather than
suggesting it. All 2504 non-null values are valid indices, and **not one of
them names any of the first 23 declarations** — while the operands read those
freely. Those 23 are exactly the things you cannot assign to:

| | |
|---|---|
| `f0`–`f9` | the float literals 0 to 9 |
| `d0`–`d9` | the integer literals 0 to 9 |
| `dCurrentProblem`, `dCurrentSender`, `fDifficulty` | written by the engine at load |

That last row is confirmed from the other side: `ai.dll` resolves
`dCurrentProblem` and `dCurrentSender` by name immediately after the version
check, along with ten more it writes itself. So a node reads its sources and
writes its result, and 3561 of the 6065 write nowhere at all.

The same table serves the `.fml` formula files beside the scripts: of the 15
identifiers they use, 11 are `varset.var` declarations and the other four are
the format's own keywords (`FUNCTION`, `FormulaSet`, `export`, `file`).

### Who reads a script, and what 73 means

Not `Behavior.dll`. **`ai.dll`** carries `.scr`, `.var`, `.fml` and
`MISSIONS\SCRIPTS\`; `Behavior.dll` carries `ResTree` and owns the research
tree instead. The loader checks the first word against 73 and, on a mismatch,
prints

> `(AI.DLL) ERROR: Scripts are not up to date !`

so 73 is a **format version**, not a magic number — the engine's own words for
it. Nothing in the corpus carries any other value.

### What `.trf` turned out to be

A dead end for this, and worth writing down so nobody walks it twice. The 29
`.trf` archives beside the scripts are the **research tree**, which is what
`ResTree` was saying: `TRF6` is 395 part ids (`e_gun_bc_05`), `TRF7` weapon
codes (`L80mmRG`), `TRF8` display names (`Large Rail Gun`), `TRF9`
descriptions, `TRFA` stat templates for a UI panel
(`@G@Weight @B,weight,G,t,5,1@`). `TRF0` being 14720 bytes — 64 × 230, against
an operand ceiling of 228 — is a coincidence, and a good reminder that a
number landing in the right range is not evidence.

## The node has two forms, and `head[0]` tells them apart

`head[0]` was the last field with no reading at all. It has one, and it
splits the node in two — *measured*, with no exceptions in 6065 nodes.

### A call

**2087 nodes carry `head[0]`, and every one is opcode 6.** On all 2087,
`head[2]`, `head[3]` and the trailer are **null**. So a node with a function
uses only the function, its operands and its destination:

```
head[0]      which function, 57 distinct over 0..72
operands     its arguments
head[1]      where the result goes, or null
```

A function's signature is fixed by its number:

| | |
|---|---:|
| appear under a single opcode | **57 of 57** |
| take a single number of arguments | **52 of 57** |
| either always write a destination or never | **56 of 57** |

That is a function table. The five with a variable count (14, 15, 25, 28, 44)
take 2–3, 9–11, 1/4/5, 10–11 and 4–5 arguments, which reads as optional
trailing ones.

### An assignment

The other 3039 opcode-6 nodes have no function, and they write from one of two
places. The correspondence is exact:

| writes | `head[2]` | trailer | operands | nodes |
|---|---|---|---|---:|
| yes | — | set | — | 1379 |
| yes | set | — | — | 339 |
| no | — | — | — | 1211 |
| no | — | — | one | 110 |

**All 1718 nodes that write carry exactly one of the two sources, and all 1321
that write nothing carry neither.** The trailer is a variable index — 1379 of
1379 valid, and the commonest are `f0`, `f1`, `f2`, the literal pool — so that
form is `destination = variable`. `head[2]` is a small integer, 1 to 16 mostly,
with the top bit set on 55 of 339, and its destination is a **`DWORD` on all
339**, never a float.

And `head[3]` is a **selector of its own**, the way `head[0]` is for a call,
with its own fixed arity — *measured*, **3039 of 3039**:

| tag | operands | writes | nodes |
|---:|---:|---|---:|
| −1 | 0 | yes, from a variable | 1452 |
| 1 | 0 | no | 944 |
| 2 | 0 | no | 57 |
| 3 | **1** | no | 85 |
| 4 | **1** | no | 25 |
| 5 | 0 | no | 210 |
| 6 | 0 | yes, from a number | 266 |

So the 110 nodes carrying a single operand are exactly tags 3 and 4, and
nothing else distinguishes them from the 1211 bare ones.

### `head[2]` is a variable under one tag and a number under the other

The two readings are hard to separate for small values, because the literal
pool sits at the front of `varset.var` — `f7` is at index 7 *and* has the
value 7. The tag separates them anyway:

- **Tag −1** — all 73 values are valid indices, and only **three distinct ones
  are ever used**: `d2`, `ERROR`, `dTemp3`. Scattered, named, meaningful. This
  is a variable.
- **Tag 6** — **63 of 266 cannot be an index at all**: 55 carry a flag in the
  high half (`0x8000_0000`) and two are `4094` and `65534`, past the end of a
  231-entry table. The rest run **densely from 1 to 28** with one gap, which is
  what a small integer looks like and not what a choice of variables looks
  like. This is a number.

The flag on those 55 is **unknown**; a sign is the obvious guess and nothing
here tests it. `Node.reference` and `Node.literal` return one or the other and
`NULL` for the wrong tag.

### What the functions are — *guess*

Nothing names them; `ai.dll` holds no run of 73 identifiers. But the arguments
do the work, because they are `varset.var` names and the developers wrote
those in full:

| fn | uses | its arguments include | reading |
|---:|---:|---|---|
| 30 | 244 | `MESSAGE_INFO`, `OBJECTIVE_COMPLETE` | show a message, tick an objective |
| 15 | 236 | `ORDER_ROBOT_PATROL`, `INSERT_ORDER_REPLACE`, `fSuccess` | give a robot an order |
| 8 | 179 | `ST_SOLVING`, `ST_SOLVED` | set the problem's state |
| 2 | 176 | `PBM_ROBOT_NEEDED`, `ROBOT_BATTLEUNIT` | raise a problem |
| 27 | 147 | `ACTION_DESTROY`, `EXP_TARGET_BY_LOGIC_ID` | set a group's action |
| 28 | 85 | `dAttackGroup`, `ORDER_ROBOT_ATTACK` | order a group |
| 25 | 85 | `TAKE_BY_HITS`, `TARGET_BY_PLACE` → `dAttackGroup` | form a group |
| 19 | 58 | `ClanBaseX`, `ClanBaseY`, `ClanID` | place the clan's base |

Function 19 appears in `Init` and nowhere else, which is where placing a base
belongs. What each `ORDER_*` a script gives does is in
[31-packages.md](31-packages.md); `ORDER_ROBOT_CAPTURE` is the search task
restricted to buildings, not a task of its own. These readings are **guesses from the argument vocabulary** and the
reader names none of them: it exposes `Node.function` as a number.

## Reading a script

Every field of a node now has a role, so a handler can be printed as
pseudo-code. `openparkan behaviour c1m2e PBM_BUILDING_INF_CAPTURE_Start`:

```
   0  dX = fn29(d0)
   1  dT = fn14(UNIT_ANY_NEAREST_CAPTURER, TARGET_BY_LOGIC_ID, dX)
   2  op1(dT, ERROR)
   3  fTemp = fn12()
   ...
   7  fn2(PBM_ROBOT_NEEDED, fTemp, dTemp1, dTemp2, ROBOT_BATTLEUNIT, NONE, SELECT_FASTEST)
   8  tag5
   9  tag1
  10  fn15(dT, ORDER_ROBOT_CAPTURE, INSERT_ORDER_REPLACE, NONE, f0, f0, f0, f0, TARGET_BY_LOGIC_ID, dX)
  11  fn6(dT, UNIT_NORMAL)
  17  dAttackGroup = fn25(TAKE_BY_HITS, dTemp3, TARGET_BY_LOGIC_ID, dT)
  30  fn28(dAttackGroup, ORDER_ROBOT_PATROL, INSERT_ORDER_REPLACE, ...)
  32  fn27(ACTION_DESTROY, EXP_TARGET_BY_LOGIC_ID, dX)
```

**`fnN`, `opN` and `tagN` are numbered, not named**, and the renderer will not
name them: the shipped files say what they take, not what they do. Everything
else on those lines is a real name out of `varset.var`.

Read as English, that handler finds the nearest capturer, asks for a battle
robot if there is none, orders the one it has to capture, takes a group by
hits, patrols it and marks the target for destruction. That is a *reading*,
and the only part of it the data states is the argument names.

### A comparison opens a block and tag 1 closes it — *measured*

The tags are not scattered, and the counts give it away: there are **944 tag-1
nodes against 939 comparisons**. Per handler the match is near-exact.

- **675 of 677 handlers hold exactly as many bare tag-1 nodes as
  comparisons**, and bracket cleanly — the depth never goes negative and ends
  at zero. Nesting reaches **five** deep.
- The two that do not are both `Mission` handlers, in `c2m2p.scr` and
  `c4m1p.scr`, and each carries a spare closer.
- **tag 5 is followed immediately by tag 1 on 210 of 210** nodes, so it is
  always the last thing inside a block.
- **tag 3 follows a comparison on 82 of 85**, and is one of the two tags that
  take an operand.

So the renderer indents, and the result reads as guard clauses:

```
 0  dT = fn50(ORDER_ROBOT_CAPTURE)
 1  op5(dT, ERROR)
 2    tag5
 3  tag1
 4  fn7()
 5  dX = fn29(d0)
 6  dT = fn14(UNIT_ANY_NEAREST_CAPTURER, TARGET_BY_LOGIC_ID, dX)
 7  op1(dT, ERROR)
 8    fTemp = fn12()
12    fn2(PBM_ROBOT_NEEDED, fTemp, ..., ROBOT_BATTLEUNIT, NONE, SELECT_FASTEST)
13    tag5
14  tag1
15  fn15(dT, ORDER_ROBOT_CAPTURE, INSERT_ORDER_REPLACE, ...)
```

### What the other tags do — *measured*, and then a reading

With the blocks established, the remaining tags sort themselves.

**Tags 3, 4 and 5 end the block they sit in.** Across the corpus they are
**followed immediately by a closer on 319 of 320** nodes — tag 5 on 210/210,
tag 4 on 25/25, tag 3 on 84/85 — and they sit at depth 1 or deeper. They
differ only in what they carry: tag 5 nothing, tags 3 and 4 one operand each.

Tag 3's operand is the interesting one. It is a **weight**:

| operand | nodes |
|---|---:|
| `fPry` | 26 |
| `dArealFactor` | 24 |
| `fAgressive` | 10 |
| `dMaxPlant` | 7 |
| `dUnitBattleFactor`, `dMaxMine`, `fPlentyResourceAmount` | 2 each |

Tag 4's is more mixed — `d9` on 9 of 25, then `dCurrentSender`,
`dMaxTransport`, `dBaseFactor`.

**Tag 2 is not a terminator.** It sits at the outermost depth on **56 of 57**,
is followed by a closer only once, and ends 18 handlers outright. What comes
after it is the handler's bookkeeping: `fn27(ACTION_DESTROY, ...)` and
`fn27(ACTION_CAPTURE_BUILDING, ...)` on 45 nodes, `fn8(ST_SOLVING)` on 26,
`fn2(PBM_…)` on 55. It appears only in `PBM_*_Start` (27), `PBM_*_Continue`
(18) and `Problems0` (12).

*Guess*, and only this much: a comparison is an `if` and tag 1 its end; tags
3, 4 and 5 are three ways of leaving a block, with tag 5 a plain return and
tag 3 a return carrying a priority — `fPry` reads as exactly that, and an AI
problem's `_Start` handler returning a weight is how a planner would rank what
to do next. Tag 2 marks where a handler stops planning and starts committing.
The bracketing and the counts are measured; the words for them are not, and
the renderer prints `tagN`.

## What the interpreter looks like

`ai.dll` runs these scripts, and its dispatch loop is at **`0x100122b5`**:

```
mov  eax, dword ptr [edi]        ; the node's first field
cmp  eax, -1
je   0x10012313                  ; -1 goes the other way
mov  edx, dword ptr [esi + 0xc]  ; the handler table
call dword ptr [edx + eax*4]     ; table[head[0]]
mov  edi, dword ptr [edi + 8]    ; on to the next node
```

That is the two-form node **confirmed from the code**, and it was derived from
the data first: `head[0]` is loaded, tested against −1, and either indexes a
handler table or takes the other branch. Nothing about the split was a
reading. The last line also says nodes are a **linked list** once loaded,
whatever they are on disk.

The handler table is written contiguously by the loader's initialiser at
**`0x1000129e`** — **70 stores**, at object offsets `0xc` through `0x120`,
four bytes apart with no gaps, and all **70 targets distinct**. They are real
functions with ordinary prologues.

**The mapping from a function id to one of those 70 slots is not
established.** The scripts use ids 0 to 72 with 57 distinct values, and 70
slots cannot cover 73 ids. Nothing writes past `0x120`, and the base the
dispatch indexes from is loaded out of another object, so which slot is id 0
stays open.

### Naming the functions from the binary did not work

The handlers index the interpreter's own structures — `imul eax, 0x14` for one
stride, `shl ecx, 5` for another — and call helpers. **None of the ones
inspected references a string.** There is no name table, no debug text and no
log line to hang an identifier on, which is why the readings in this document
come from the argument vocabulary instead. Naming them properly means
following the helpers into the engine's object model, which is the
multi-month half and is not attempted here.

## What is not read here

- **What the 57 functions compute**, and what the six fixed-arity opcodes do.
  The shapes are settled; the meanings are the months-long half, and they are
  gameplay rather than format.
- **What the exit tags mean.** Their shape is read — 3, 4 and 5 end a block,
  2 does not — but whether tag 3's weight is a priority, and what separates
  the three exits, is not.
- **The flag bit on 55 literals** (`0x8000_0000`), and the two sentinel
  values `4094` and `65534`.
- **`.trf`**, the research tree — identified above, not read.
