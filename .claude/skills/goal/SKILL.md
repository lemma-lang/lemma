---
name: goal
description: Runs the chain of roles end to end, without handing control back between steps or between issues, until a goal is reached. Use it when the user wants the project to move forward on its own — "/goal parse loop carries", "/goal LEM-1", "/goal all". Never lifts the exit gate.
---

You drive the project toward a goal **without stopping at each step**. This is the only mode in
which the rule "you hand control back between two steps" from `.claude/orchestration.md` is
lifted: invoking this command *is* the lifting that this rule provides for, and you record it as
such.

Everything else in `.claude/orchestration.md` and `AGENTS.md` still applies, to the letter.

## Arguments

`/goal <goal>`: in plain words ("parse loop carries"), or a key (`LEM-6`), or a list
(`LEM-1 LEM-2`), or `all` for every open issue.

Options:

- `--review`: restores the stop at plan review (rule 2). Without it, you write the plan, you
  report a digest of it and **you move on**.
- `--until LEM-n`: stops after that issue.
- `--one-step`: runs only one step of the current plan, then hands control back. Useful for resuming.

## What you lift, and what you never lift

**Lifted** (that is the purpose of the command): the stop between two steps of the plan; the stop
between two issues; the user's review of the plan, unless `--review`.

**Never lifted, under any pretext, including when the goal is far off and stopping costs time:**

1. **The exit gate in full**, before each end-of-issue commit: the repository's checks run by you
   and read (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace`, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`, and
   `cargo deny check` when dependencies moved), tests and specs untouched since the red commit
   (rule 4), verdict from the `reviewer`, and the `auditor`, `proof-engineer`, `perf-engineer` or
   `devsecops` in parallel when the change calls for them. `INCOMPLETE` is not a green light.
2. **Every commit goes through the `committer`.** You never commit yourself.
3. **Writes to the outside world remain subject to the harness**: every `glab issue create` or
   `update` shows its permission request to the user. You do not bypass them, you do not add
   them to an `allow` list, and a refusal stops the current issue.
4. **Force pushes remain denied** by `.claude/settings.json`, and you do not bypass that. An
   ordinary push is shown to the user before it goes out; once pushed, the issue moves on to
   `mr-writer` then `mr-reviewer`, and the merge is submitted to the user like everything else.
5. **Nothing destructive without the user**: `git reset --hard`, rewriting history, deleting an
   issue, wiping a shared `target/` another worktree is building into.
6. **Root stays with the user**: a measurement that needs `sudo` (kperf cycle counters) is handed
   to the user, never run.

## Flow

### 1. Establish the scope and the order

Read the issues with `glab issue list --all` and, for those the goal targets, their content.
Translate the goal into a list of keys. **Order them by their dependencies**: a description that
starts with "Blocked by #n" goes after issue n, unless n is closed or already handled in this run.
In case of a cycle or ambiguity, stop and ask.

If no issue covers the goal, have the `ticket-writer` create one (the user will approve the
creation) rather than working outside a ticket.

Write the chosen scope and order to `.claude/state/goal.md` before starting: that is what the
next session will read if this one closes.

### 2. For each issue, the chain

`ticket-reader` → `ticket-brancher` → (`language-designer`) → `planner` → loop per step →
exit gate → `committer` → push → `mr-writer` → `mr-reviewer` → merge.

You pass the baton yourself: one role's report goes into the next one's prompt. The plan
circulates **by its path** (`.claude/plans/<KEY>.md`), never copied out.

The `language-designer` goes before the `planner` when the issue touches the language itself
(syntax, typing, effects, memory model, the spec / impl / equiv contract). If it judges the
proposal out of scope (`docs/adr/0002-scope.md`), the issue stops there: that is a product
question, you report it.

After the plan: a five-line digest (chosen approach, number of steps, what each step delivers),
written **in the same message as the call that launches the first step**, unless `--review`. A
message that only carries the digest ends your turn, and the run stops there without anyone
having asked for it. If the plan says it is missing an external fact, launch the `researcher`
and have the `planner` rework it before going further.

### 3. For each step of the plan

```
(proof-engineer, when the step brings a new spec: it writes the spec and the wrong outputs to reject)
tester (one pass per requested test type)
  → wiring lines the tester reports (mod tests;, dev-dependency), added by you
  → commit of the red tests by the committer, a single hash, announced paths only
  → coder (or release-engineer / perf-engineer if the deliverable is pipeline config / a benchmark)
  → integrity check: tests, wiring lines and spec clauses unchanged since <hash> (rule 4)
  → next step, without handing control back
```

A step the plan marks **"Competing passes: yes"** follows the competing passes protocol of
`.claude/orchestration.md`. Automation does not remove it; it simply chains on after it.

A step the plan announces **without a tester** (pipeline, benchmark, documentation) is launched as
is, and you say so in your trace so it does not look like an oversight.

**Update `.claude/state/<KEY>.md`** after each test commit and each coder pass: the table of
steps with their state and the hash of the test commit on the relevant row.

### 4. End of issue

Full exit gate, `committer`, push, `mr-writer`, `mr-reviewer`, merge, then next issue **without
stopping**. Record the finished key in `.claude/state/goal.md`. A merge the user declines, or an
`mr-reviewer` verdict other than `MERGEABLE`, is a stop like those below.

## When you stop anyway

These are states, not fatigue. You hand control back, you say which one it is, and you propose
what comes next:

- **`NON-COMPLIANT` twice in a row** on the same issue: the problem is in the plan or the ticket.
- **The `coder` gives up** after three attempts: launch the `debugger`, apply its verdict once;
  if that does not unblock it, stop.
- **A test or a spec reported as wrong** by the coder (rule 5).
- **A guard that refuses the same write twice** to the same role.
- **A missing secret or access** that you cannot obtain. You do not make it up.
- **A proof that turns `unknown` or unstable**, or a **regression** the `perf-engineer` confirms:
  you raise it before piling code on top of it.
- **A critical or high finding** from the `devsecops` or the `auditor`.
- **A product or language question** that the plan, an issue or an ADR leaves open.
- **A permission refusal** from the user on an external write.

## What you report

Along the way, one line per step completed: issue, step, state, hash. Write that line in the same
message as the call that launches the next role: on its own, it ends your turn.

At the end, or on stopping: the finished issues with their commits and merge requests, those
remaining, what was explicitly set aside, the open findings, and **what is waiting on the user**.

## A guarantee you give

At the end of a `/goal`, the state of the repository is a state that the full chain has
validated: every commit has gone through the exit gate, no test file and no spec has moved under
the coder, and no write to the outside world has gone out without the user seeing it. If you
cannot uphold that, you stop and say so.
