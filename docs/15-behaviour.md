# The behaviour scripts — `.scr`

`MISSIONS/SCRIPTS/` holds 58 `.scr` files, from 286 bytes to 21050. They are
the mission AI, and `Behavior.dll` interprets them. This reads **all 58 end to
end with nothing left over**: 677 handlers and 6065 nodes.

Before this they were the project's largest unread format. The feasibility
note called them "the main obstacle" and split the problem in two: parsing the
graph is a weekend, making the nodes *behave* is months. That split is real,
and this document is the first half only. **What a node does is not read
here**, and the last section says exactly what would be needed.

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

## Where the vocabulary is not

The 9239 operands run **0 to 228**, with 170 distinct values, and they are
never negative. They are not indices into the script that holds them:
**49 of the 58 scripts name an operand at or past their own node count**, and
the ceiling is the same 228 in a script of 17 nodes as in one of 585.

So the node vocabulary is shared and lives outside the `.scr` files. It is not
in the binaries either — `Behavior.dll` carries no `PBM_` string at all and
`ai.dll` carries exactly one, so there is no name table to lift the way
`World3D.dll`'s [command resolvers](14-controls.md) were lifted.

The open lead is `.trf`. The same directory holds 29 of them; each is an NRes
archive of 12 streams tagged `TRF0`–`TRFB` and every one is named `ResTree` —
a resource *tree*. `TRF0` is **14720 bytes in all 29 files**, which is the only
stream whose size never varies, and 14720 is 64 × 230 — the right order for a
table the operands' ceiling of 228 would index. That is suggestive and nothing
more: the stream's own record boundaries do not fall at 64, and this project
does not write down a reading it has not checked. `.trf` is unread.

## What is not read here

- **What a node does.** The head fields, the opcode meanings and the operand
  namespace are all open. This is the months-long half of the problem and it
  is gameplay, not format.
- **The four head fields.** Their ranges differ, so they are not four of a
  kind: slot 0 runs −1..72, slot 1 −1..228 — the operands' own range, so it is
  probably a reference of the same kind — slot 2 is −1 on 5726 of 6065 nodes
  with a handful of values carrying the top bit set, and slot 3 runs −1..6,
  the opcode's range.
- **The trailer**, −1 on 4686 of 6065 nodes and 86 distinct values otherwise.
- **`.trf`**, above, and **`.fml`** — 58 plain-text formula sets in the same
  directory, `FUNCTION( , fTemp + 0.001, )`.
