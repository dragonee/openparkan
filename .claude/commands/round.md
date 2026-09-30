---
description: Work a round of open questions — pick a batch, research it in parallel worktrees, land it, write up the queue
argument-hint: "[how many, default 12] [areas to prefer, e.g. sky effects]"
---

# A round of open questions

Close a batch of [OPEN-QUESTIONS.md](../../OPEN-QUESTIONS.md) by research, fix the
engine where the answer contradicts it, and write the result back into the docs
and the queue.

Default shape: **12 questions, four areas of three, four agents.** `$ARGUMENTS`
may change the count or name areas to prefer. Keep the count a multiple of the
group size so every agent gets a coherent area.

## 1. Pick the batch

Read `OPEN-QUESTIONS.md` and pick questions, grouped by the file's own area
headings. It holds only the open lines; closed ones, with their answers, are in
`COMPLETED-QUESTIONS.md`. Prefer, in this order:

1. **Questions the engine answers with a stand-in** — `[M1]`–`[M5]` in the queue,
   and the stand-ins table in `engine/README.md`. These give the agent something
   to validate and possibly fix, not just a doc to write.
2. **Questions tractable from the shipped files or the DLLs** — a flag whose
   bearers can be counted, a reader that can be enumerated, a value that can be
   measured across the install.
3. Questions whose answer would change how a mission plays.

Two constraints on the grouping:

- **Each area must own a disjoint set of docs.** Four agents editing one doc
  conflicts; four agents editing `engine/README.md`'s stand-in table row-wise
  does not. Say in each brief which docs that agent owns, and name any doc a
  neighbour owns this round.
- **Check the line is not already answered.** Queue lines go stale — an earlier
  round sometimes answers a question and never moves it out of the queue. Grep
  the docs, `COMPLETED-QUESTIONS.md` and `git log` for the subject before
  spending an agent on it.

## 2. Check the engine first

Work lands between rounds — fixes, milestones, a session chasing a bug — and it
can do a question's work without ever touching the queue. Before a question goes
to an agent, look at how the engine implements it now: `codegraph explore
"<the question's subject>"` from the repo root, the code it points at, and the
subject's row in `engine/README.md`.

- **Researched already.** The code cites a read for it — the addresses, or a
  doc section marked *read* or *measured* — and no `STAND-IN` marks it. Move
  the question to `COMPLETED-QUESTIONS.md`, under its section, as closed (`- [x]
  ~~…~~ — closed <date>: already in the engine`, with the code's citation and the
  commit that brought it, found by `git log -S`), research it no further, and
  pick another question in its place.
- **A stand-in.** The code marks it `// STAND-IN: docs/NN#section`, or its row
  still sits in the stand-ins table: the engine is guessing. Research it further
  — it goes to an agent, with the stand-in's current wording in the brief.

## 3. Launch the agents

Launch all four **in a single message**, each with `subagent_type:
"general-purpose"` and `isolation: "worktree"`, so they research in parallel on
their own branch. Give each the brief below with its own three questions
appended, including everything the queue already knows about them (prior
findings, the stand-in's current wording, the doc's own "next place to look").

> You are one of four research agents working a round of open questions on
> **openparkan**, a clean-room reimplementation of *Parkan: Iron Strategy*. You
> are in your own git worktree of the repo, branched off `main`. Work only in
> your worktree.
>
> **Read first**: `docs/09-method.md` (the project's method and its legal
> discipline), then the docs listed for your questions. The method in one line:
> facts come from the shipped game files and from disassembling the shipped
> DLLs; **no code is ever transcribed, translated or paraphrased from
> disassembly into this repo** — readers and the engine are written from the
> documented layout in the project's own idiom, and every claim is checked
> against the whole install.
>
> **The install** is found by `uv run python -c "from openparkan import gamedir;
> print(gamedir.find())"`. The Python toolkit `openparkan/` parses the formats;
> `tests/` checks it against the install; `analysis/pe.py` wraps pefile+capstone
> for the DLLs (`uv sync --group analysis` first; the recipe is in
> docs/09-method.md). `engine/` is the Rust engine. `codegraph explore
> "<question or symbol>"` from the repo root answers structural questions about
> the engine faster than grep.
>
> **What "answered" means here.** A claim is worth writing only if it is
> measured over the shipped data with counts ("all 6166", "12 of 2634", "0 of
> 458"), or read out of a binary with the addresses named. A **negative** result
> is publishable and must carry a **control**: the same search, asked where the
> thing is known to exist, that does find something. Never round a guess up to a
> fact; *not established* and *inferred* are respectable words in these docs and
> are used precisely. Do not claim you ran something you did not run. Read
> docs/09-method.md's "Searches that do not discriminate" before designing an
> immediate scan.
>
> **Your task, for each of your three questions:**
> 0. Try to close it. If it will not close, report exactly how far it got:
>    *partly answered* (with the control) or *narrowed* (what remains).
> 1. Check what the engine does with it today. Stand-ins are marked in the Rust
>    source as `// STAND-IN: docs/NN#section` and tabulated in
>    `engine/README.md`.
> 2. If the engine has it wrong, fix it, and pin the fix with a test (Rust tests
>    live beside the code in `engine/crates/*`; install-data tests live in
>    `tests/`). If the engine already had it right, say so — that is a result
>    too.
> 3. Update the docs you own: write the finding into the right section in the
>    house voice, and in the doc's *Not established* / *Not resolved* / *What is
>    not read here* list strike the answered line (`~~...~~`) and follow it with
>    the answer and a link, exactly as the neighbouring entries do. If a stand-in
>    closes or changes, update or drop its row in `engine/README.md` — keep that
>    edit to the row itself, four agents are editing that table.
> 4. Commit in your worktree. Commit style: a lowercase declarative sentence
>    that states the *fact*, not the activity — e.g. `fix: a tracked warbot's
>    four belts lie along the ground under them, and its hull does not`. Body:
>    prose paragraphs with the numbers and addresses. End every commit message
>    with this session's `Co-Authored-By:` attribution line.
>
> Run `uv run pytest`, `uv run openparkan verify` and, if you touched the engine,
> `cargo test` inside `engine/` before committing, and report what passed.
> **Give cargo its own target dir** (`CARGO_TARGET_DIR=$PWD/engine/target`):
> sharing one across worktrees has served agents another worktree's build —
> shifting test counts, and once a compile error from a type that did not exist
> in that tree.
>
> **Do not touch `OPEN-QUESTIONS.md` or `COMPLETED-QUESTIONS.md`** — the
> coordinator updates both at the end. Do not touch docs outside your list unless a fact genuinely belongs there;
> if it does, keep the edit small and say so in your report.
>
> **Report back**: your branch name, your commits (hash + subject), and for each
> question the verdict, 3–5 sentences of the finding in the queue's voice (with
> the numbers and addresses), whether the engine was right or wrong and what
> changed, and which docs moved.

## 4. Land each branch as it finishes

Do not wait for all four. As each agent reports:

1. Read its commit (`git show --stat`, and the diff of anything surprising).
   Spot-check at least one load-bearing claim against the install or the binary
   yourself — a tidy explanation is not evidence.
2. Rebase its branch onto the current `main`, from inside its worktree:
   `git rebase main`.
3. Fast-forward `main` onto it: `git merge --ff-only <branch>`.

Rebasing in completion order keeps each conflict small. Agents that touched the
same table (`engine/README.md`, `openparkan/verify.py`) usually merge textually;
resolve by keeping both rows.

## 5. Verify the combined tree

Once all four are in, run the whole thing — the agents each verified their own
branch, not the merge:

```
uv run pytest -q
uv run openparkan verify
uv run ruff check openparkan/
cd engine && cargo test --workspace
cd engine && cargo test --workspace -- --ignored   # install-backed, ~2.5 min
```

## 6. Write up the queue

`OPEN-QUESTIONS.md` and `COMPLETED-QUESTIONS.md` are the coordinator's alone.
For each question in the batch that closed, remove its `- [ ]` line from the
queue and add a `- [x] ~~struck~~ — closed <date>:` entry to
`COMPLETED-QUESTIONS.md`, under the same section and subheading, carrying the
finding, its counts and its control, and a link to the doc that now holds it. A
remainder the answer names goes back into the queue as a line of its own. A
question that did not close stays in the queue and is **rewritten to say what is
left**, not what was asked.

Then add a paragraph to *The rounds* at the top of `COMPLETED-QUESTIONS.md`: what
closed, which were negatives and what their controls were, where the engine
changed, and —
most importantly — **any premise of an earlier round this one corrected**. A
round that overturns an earlier count says so in the file, and the earlier entry
is rewritten where it stands rather than quietly replaced. Recording which way
the correction ran is part of the same honesty as recording where a fact came
from.

Commit the queue separately from any doc correction you made yourself, and
finish by removing the agents' worktrees and branches:

```
git worktree remove --force .claude/worktrees/agent-<id>
git branch -D worktree-agent-<id>
```
