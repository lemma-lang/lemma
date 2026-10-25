---
name: lemma-code-review
description: Use when reviewing a pull request or a branch in the Lemma repository, or before asking for review of your own. Covers getting the real diff, checking it against what was asked and against AGENTS.md, and the soundness checks specific to a verifying compiler (LLVM facts, unsafe and the JIT, verifier answers, hostile input), and trying to break the change with probes that are run.
---

# Reviewing a Lemma change

Two questions, kept apart: **does the change do what was asked**, and **is it well done, can it
break or be broken**. Perfectly conforming code can be bad. Prioritize soundness and broken
behavior over style; one substantiated blocker beats a list of nits.

## Get the real diff

The object of review is everything the branch produced since its base, commits and working tree
together.

```sh
git fetch origin
git merge-base origin/<base> HEAD     # the base is not always main: stacked branches exist
git diff --stat <merge-base>
git diff <merge-base>
git status --porcelain                # untracked files are in no diff: read them in full
```

An empty or unobtainable diff means there is nothing to review: say so instead of reviewing files
by reading them, which would miss what was deleted.

## Against what was asked

1. Read the issue's acceptance criteria and the ADRs it cites before the diff. A criterion
   without matching code is a gap even if the suite is green.
2. Code that serves no criterion is drift.
3. Run the read-only checks of [lemma-checks](../lemma-checks/SKILL.md) and
   `tools/rules/check.sh <merge-base>`; quote findings with path and line.

## Against AGENTS.md

- Every new public item has a doc comment; examples in doc comments are doc-tests.
- Errors are structured values with a location (and, for proofs, the Z3 counterexample); no
  `unwrap()` / `expect()` outside tests.
- `unsafe` only in `lemma-jit`, each block with a true `// SAFETY:` comment.
- A change of described behavior updates `README.md`, `docs/design.md` or `AGENTS.md` in the same
  change; a reversed or extended decision adds or supersedes an ADR.
- Any change to `[workspace.lints]` in `Cargo.toml`, and any new `#[allow(…)]` or `#[expect(…)]`,
  is a deviation unless the issue asked for it.
- Comments state the *why*, above the item; none narrates the change or names a ticket.

## Tests and specs

The most serious defect is the one that turns wrong work green: a test weakened, deleted,
`#[ignore]`d or made tautological; a spec loosened; an `allow(...)` added to an `equiv` clause.
Read every changed `fn` signature and contract line of the `.lm` files. Flag tests that are
redundant with an existing one, naming it. See [lemma-testing](../lemma-testing/SKILL.md) and
[lemma-specs-and-proofs](../lemma-specs-and-proofs/SKILL.md).

## Soundness checklist

In this order, as it applies:

- **Every LLVM fact emitted is a proven fact.** For each `noalias`, alias scope metadata,
  `memory(...)`, `nonnull`, `dereferenceable`, `align`, `inbounds`, `nsw` / `nuw` or fast-math
  flag the diff emits, find the proof obligation or type guarantee it rests on (disjoint regions,
  `ro` / `wo`, bounds). An attribute without one is undefined behavior waiting for an optimization
  pass. A `pure` function reading through a pointer is at most `memory(argmem: read)`, never
  `memory(none)`. Fast-math flags only under the matching `allow(...)`.
- **Unsafe and the JIT.** Does each `// SAFETY:` comment cover every operation of its block? Is a
  JIT function pointer's signature the one the code was compiled for? Can code be freed while a
  pointer to it is live (hot replacement)? Is any `transmute` sized and aligned?
- **Verifier answers.** A `sat` / `unsat` / `unknown` confusion, `unknown` treated as success, an
  obligation dropped on an error path, "proven" without a query.
- **Hostile input.** The parser and the verifier receive arbitrary source: a panic, an index out
  of bounds, unbounded recursion or allocation is a finding.
- **Resources and errors.** LLVM and Z3 contexts released, errors not swallowed, no shared state
  across threads without a reason.

## Try to break it

Reading finds what looks wrong; running finds what is wrong. For each suspected defect, and for
each boundary the change claims to handle (an error path, a malformed input, an ownership hand-off
at the FFI, a lifetime the borrow checker is said to enforce), write the smallest program that
would show the failure, and run it:

- A probe is a throwaway crate outside the tracked files, with its own empty `[workspace]` table
  so that Cargo does not attach it to the repository's workspace, path dependencies on the crates
  under review, and `--target-dir <repository>/target` so that LLVM and Z3 are not rebuilt.
- A compile-fail claim is probed by a program that must not compile.
- A targeted test, an IR dump of a case to read an emitted attribute, and
  `cargo +nightly miri test -p lemma-jit` when installed, are probes too.

A counterexample that holds is reported with its input and what it produced, and becomes a
non-regression test in the change; the probe itself never lands in the tree. A finding no probe
reproduces is reported as a hypothesis, with the reason none could be written.

## Reporting

Each finding: path and line, what goes wrong (miscompilation, undefined behavior, crash on input,
unproven program accepted, criterion not met), and how to trigger it. Reopen the cited line before
reporting it; mark an unconfirmed hypothesis as such. Call a theoretical risk theoretical. Spell
out an obvious fix in full. Separate blockers from suggestions, and say what you could not check.
