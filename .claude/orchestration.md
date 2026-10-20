# The chain of roles

This repository has eighteen specialized subagents, in `.claude/agents/`. They, their guards
(`.claude/hooks/`), their settings (`.claude/settings.json`), the shape rules (`.claude/rules/`)
and this policy are **committed**: every clone and every worktree gets the same setup. What is
not committed is their working memory (plans, state, reviews, reports), listed in `.gitignore`.

**Tickets are the project's GitHub issues.** The key `LEM-<n>` refers to issue no. `<n>`; the
prefix is declared in `.claude/project.conf` (`TICKET_KEY=LEM`). The key is what names the
branch (`LEM-12-<slug>`), the plan, the state and the review. `gh` must be installed and
authenticated (`gh auth status`); without it, `ticket-reader` and `ticket-writer` stop and say so, and a request
already written out in plain text goes straight to the `planner`. Delegate work to the roles **on
your own initiative**, without waiting to be told their names: each carries the model suited to
its task, and calling them is what avoids handling everything at the price of reasoning.

The repository rules live in `AGENTS.md` (commands, commit convention, code style, documentation
that stays true) and the decisions in `docs/adr/`. Every role reads them; this file says who does
what, in which order, and what is checked before anything is committed.

## Which role for which intent

| The user wants… | Role |
| ------------------- | ---- |
| to learn about an issue, a spec, a written request | `ticket-reader` |
| to create an issue, complete a description, split a milestone | `ticket-writer` |
| to **tackle** a ticket, start the work, create the branch | `ticket-brancher` |
| to decide whether a language feature belongs in Lemma, change its semantics, record a decision | `language-designer` |
| to decide **how** to do it, weigh options, split into steps | `planner` |
| to write tests: unit, integration, snapshot, fuzz, differential spec-vs-impl | `tester` |
| to make tests go green, implement a step of the plan | `coder` |
| to check that **coded** work does what the ticket asked | `reviewer` |
| a quality audit, a soundness pass, a second opinion | `auditor` |
| to judge Z3 obligations, proof stability, whether a spec actually constrains | `proof-engineer` |
| to measure performance, write a benchmark, decide whether something regressed | `perf-engineer` |
| a fact that is not in the repository: a crate's docs, an LLVM or Z3 behavior, a version | `researcher` |
| to understand an anomaly nobody can explain | `debugger` |
| to write or fix the GitHub Actions workflows (`.github/`), the `justfile`, `cog.toml`, `deny.toml`, lint, format or toolchain config | `release-engineer` |
| supply chain: dependencies, `cargo deny`, CI hardening, secrets | `devsecops` |
| to open the pull request for a pushed branch, complete its description | `pr-writer` |
| to decide whether a pull request goes into `main`, and merge it | `pr-reviewer` |
| to commit, split a working tree into commits | **the exit gate**, then `committer` |

The trigger is the intent, not the vocabulary used: "what's LEM-12" calls the `ticket-reader` just
as surely as "read issue #12", and "let's start the work" calls the `ticket-brancher` just as
surely as "let's tackle it".

**Only one row of this table does not lead directly to a role: "commit".** "Commit" does not mean
"call the committer", but "carry this work through to the commit", which starts with the exit
gate. See below; it is the easiest rule to break, because the word designates a role that bears
the same name.

**The `reviewer` only reviews written code, and you are the one who brings it the scope.** Two
distinct rules, not to be confused:

- *The criterion is what the branch has produced, not the topic of the conversation.* And a clean
  tree does not mean "nothing to review": in a multi-step plan, the earlier steps are already
  committed. Look at the whole branch: `git merge-base <base> HEAD`, then `git diff --stat
  <hash>`. If anything comes out, there is something to review. If that diff is empty, say so
  instead of delegating: reviewing a plan or a specification is your job, or that of the
  `planner` you relaunch.
- *It fetches the diff itself.* Do not paste it into its prompt: you would pay for the diff twice,
  and a file left out of the paste would be a file not reviewed. Give it the scope: the ticket,
  the plan's path, the hash of the tests commit, and the base the `ticket-brancher` reported. It
  recomputes the base anyway; what you give it is a hint, not an authority.

Two common-sense caveats. A one-off question about the code ("where is this function defined") is
answered directly. And a one-line fix whose test already exists does not deserve the full chain.

## The chain

`ticket-reader` → `ticket-brancher` → (`language-designer`) → `planner` → `tester` →
**red tests commit** → `coder` → **exit gate** → `reviewer` (∥ `auditor`, `proof-engineer`,
`perf-engineer` when the change calls for them) → `committer` → push → `pr-writer` →
`pr-reviewer` → merge

The `ticket-brancher` comes **after** the reader: the branch name is built from the ticket title,
and its base is deduced from the linked tickets, which must therefore have been read.

**A branch starts from an up-to-date `main`, and the local `main` almost never is.** So `git fetch
origin` first, and you base on `origin/main`, or on the previous ticket's branch when the
`ticket-brancher` chains the tickets of a module, never on the local `main` without refreshing it.
A branch started from a stale `main` lacks what was just merged, piles up needless conflicts, and
**its diff lies**: a two-dot comparison against a lagging `main` shows every file that arrived in
the meantime as a deletion.

**After every merge**, bring the local `main` up to date and rebase the in-progress branches that
will live on for a while.

**No subagent can call another.** You are the one who passes the baton: you collect a role's
report and feed it into the next one's prompt. A role launched without the previous one's output
starts its work over, and pays for it twice.

You do not necessarily enter at the start: an already specified request starts at the `planner`,
an already reproduced bug starts at the `tester`.

### The loop runs once per step of the plan

```
for each step of the plan:
    tester  →  red tests commit  →  coder  →  integrity check
    →  STOP: you report and you wait
then, only once, when all steps are done:
    exit gate  →  reviewer  (∥ auditor, proof-engineer, perf-engineer, as called for)  →  committer
```

Do not ask the `coder` to implement the whole plan in one pass. A step is a scope it can keep in
mind, a failure that can be localized, and a tests commit that references few files.

**Name the step in the prompt.** "Step 2: …" at the top of the request, to the `tester` as to the
`coder`. Their reports announce the step they handled; if it is not the one you expected, that is
where it shows.

### You hand control back between two steps

**A finished step does not roll into the next one.** You stop, and you report in a few lines: the
step cleared, what it changed, the count of what remains, and **what the next step plans**. Then
you wait. A plan is a hypothesis, and step 1 is what puts it to the test for the first time.

The user can lift the stop: "keep going", "run the plan", "don't ask me again". The lift then
applies **to the rest of the plan**, and you record it in `.claude/state/`. What is never allowed
is chaining on your own initiative because the step went well. `/goal` lifts it for a whole goal.

Two stops are not lifted by that: a test the `coder` reports as wrong (rule 5), and a step that
overflows and must be sent back to the `planner`.

The exit gate, for its part, **runs only once**, at the end: a ticket is not judged in thirds.

### Unit tests live in a sibling file, and one line of their wiring is production code

The convention (`AGENTS.md`, "Code style"): each module declares `#[cfg(test)] mod tests;`, which
resolves to `tests.rs` in the module's directory (`src/tests.rs` for `lib.rs` and `main.rs`,
`src/parser/tests.rs` for `src/parser.rs` or `src/parser/mod.rs`). Integration tests live in
`crates/*/tests/`, snapshots and golden files in `snapshots/` or `testdata/`, fuzz targets in
`fuzz/`. `.claude/test-paths.txt` tells the guards exactly that.

**The tester cannot write the `mod tests;` line**: it sits in a production file. A new `tests.rs`
without it is never compiled, so `cargo test` stays silent: no red, and the red-tests commit would
freeze a dead file. The same holds for a dev-dependency (`insta`, `proptest`) or a `[[bench]]`
entry in a `Cargo.toml`. So:

1. The `tester` reports, under *Wiring needed*, each declaration its tests require.
2. **You add those lines yourself** (the guards do not bound the main session), nothing else.
3. You then run the suite to confirm the expected red, and the wiring lines **go into the red
   tests commit** with the tests: they are part of what the coder must not move.

Integration tests in `crates/*/tests/` are discovered by Cargo and need no wiring.

### Several `tester` passes for a complete suite

The test type is a **parameter** of the request made to the `tester`: unit, integration,
snapshot, fuzz target, differential spec-vs-impl. A step that deserves more than one type is
covered by **several passes of the same role**, one per type.

You can launch them **in parallel**, under three conditions:

1. **The scopes are disjoint, and you are the one who assigns them.** Tell each one which paths it
   writes to; compare the lists of files they report when they return. An overlap means work was
   lost, and the only answer is to relaunch.
2. **A single red tests commit, a single hash**, for all passes together.
3. **Nobody runs the complete suite while the others are writing.** Each runs only its own files
   (`cargo test -p <crate> --test <name>`, or `cargo test -p <crate> <module>::tests`); you run the
   whole suite once, after the single commit.

**If a step turns out to be too big along the way**, the split is at fault, not the coder. Send it
back to the `planner` rather than relaunching the coder a third time.

### Competing passes: two coders, one winner

On a step where the design is genuinely open, two `coder` passes each write a version against
**the same red tests**, the `auditor` compares them, and **only one wins**.

**It is only triggered on a step the plan marks** "Competing passes: yes", with its two axes.
Never on your own initiative, never on a `release-engineer` or `perf-engineer` step, never to
rescue a failed step. It costs roughly double the step: it is reserved for what sets a reused
pattern or an algorithm at the heart of the compiler (an IR lowering scheme, a proof obligation
encoding, a register of effects), not for code the directives already dictate.

**N is 2, and the two versions do not start from the same instruction.** Each receives **the axis**
the plan assigns it (by default, A: the simplest version that satisfies the tests; B: the one most
robust to malformed input and to solver or LLVM failures), and B runs on `model: opus`.

**Procedure**, after the step's red tests commit:

1. **Two worktrees, outside the repository**, starting from the exact state of the tree, including
   the uncommitted work of previous steps. The `worktree` isolation of the `Agent` tool is not
   suitable: it starts from `HEAD` and loses that work. First, once, the shared build directory
   (see *Worktrees* below for why):
   ```bash
   shelter=../$(basename "$PWD")-competing
   mkdir -p "$shelter/.cargo"
   printf '[build]\ntarget-dir = "%s/target"\n' "$PWD" > "$shelter/.cargo/config.toml"
   ```
   Then for each variant `X`:
   ```bash
   wt=$shelter/<KEY>-e<k>-X
   git worktree add --detach "$wt" HEAD
   git diff HEAD --binary | git -C "$wt" apply --allow-empty
   git ls-files --others --exclude-standard -z | rsync -a --from0 --files-from=- ./ "$wt"/
   git -C "$wt" add -A && git -C "$wt" commit -qm "competing base" --allow-empty
   git -C "$wt" rev-parse HEAD   # the variant's base
   ```
   This commit lives on a detached head of a throwaway copy and enters no branch: it is not
   subject to rule 7. The two variants then serialize on Cargo's build lock; that costs wall
   time, not correctness, and it is still far cheaper than two cold LLVM builds.
2. **The two `coder` passes in parallel**, each with the named step, the plan's path, **its axis**
   and **the absolute path of its copy**. Neither sees the other version.
3. **Check of each version, by you**: the complete suite in the copy, and `git -C "$wt" diff
   --stat <base> -- <test paths>` empty. A version that is red or that touched the tests is
   eliminated; if both are, the step goes back to the `coder` on the main tree, without competing
   passes, and you say so.
4. **The `auditor` compares**, in comparison mode: both paths, both bases, both axes and the
   plan's path. It returns **a winner** and **the precise ideas to carry over** from the other.
5. **No v3 by merging.** You carry the winning version over into the main tree:
   ```bash
   git -C "$wt" add -A && git -C "$wt" diff --cached --binary <base> | git apply
   ```
   then each retained idea goes to the `coder`, on the main tree, as a `RESERVATION`. An idea that
   requires a new test goes first through the `tester`. Pieces of the losing version that are
   worth it later go to the `ticket-writer`, one issue per piece (the snippet copied in, no
   ticket key or worktree path); a piece discarded is discarded in one line in `.claude/state/`.
6. **The usual integrity check** on the main tree (rule 4), then `git worktree remove --force` of
   both copies. You hand control back as after any step.

Record in the step's row of `.claude/state/<KEY>.md`: "competing, B wins (A: …)", and the ideas
carried over or discarded.

## The Lemma roles: design, proofs, performance

Three roles exist because of what Lemma is: a language whose functions carry a spec and an impl,
proven equivalent by Z3, compiled through LLVM for performance-critical code.

**`language-designer` goes before the `planner`** when a ticket touches the language itself:
syntax, typing, effects, the memory model, the spec / impl / equiv contract, what the compiler
promises. It is the guardian of `docs/adr/`, `docs/design.md` and of the scope set by
`docs/adr/0002-scope.md`. It answers "does this belong in Lemma, and in which form", writes or
supersedes the ADR, and updates `docs/design.md`; the plan then relies on those files, by path.
A proposal it judges out of scope stops there: you report it to the user, you do not plan it.
It writes only under `docs/`. Since the `coder` cannot write `docs/adr/` or `docs/design.md`, a
step that changes described behavior includes a `language-designer` pass, in the same ticket, so
that the documentation lands in the same commit (`AGENTS.md`, "Documentation stays true").

**`proof-engineer` judges what the prover is asked and whether its answers can be trusted.** Call
it:

- **at the exit gate**, in parallel with the `reviewer`, whenever the diff touches
  `crates/lemma-verify/`, any `spec`, `req`, `ens`, `eff` or `equiv` clause of a `.lm` file, or
  a `fn` signature line (its types and bounds are contract too);
- **before the `tester`**, when a step introduces a new spec in `examples/`: it writes the spec,
  and the list of known-wrong outputs the spec must reject, which the `tester` turns into
  differential tests;
- when a proof becomes slow, flaky or `unknown`.

It is advisory: it writes its reports in `.claude/proofs/<KEY>.md`, and may edit the specs of the
`.lm` files under `examples/`, nothing else. Its findings are settled like `RESERVATIONS`.

**The guard cannot keep it to the spec lines, nor the coder away from them.** A `.lm` function
holds its spec and its impl in the same file, and a file-level guard sees only the file. So this
boundary is held by the role prompts **and by a check you run**: the spec is a specification,
like a test, and it is frozen the same way (rule 4).

**`perf-engineer` measures, and decides whether a number moved.** Call it:

- **at the exit gate** for work labelled `perf` and for any change to `crates/lemma-codegen/` or
  `crates/lemma-jit/`, in parallel with the `reviewer`;
- **instead of the `coder`** on a plan step whose deliverable is a benchmark (it owns `benches/`
  and `crates/*/benches/`; the coder cannot write them);
- when a claim of speed-up, or a suspected regression, needs a number.

It follows a measurement protocol (Apple M1 Pro: performance versus efficiency cores, QoS,
median of repeated runs, `mach_absolute_time` when the cycle counters of `kperf` need root it does
not have) and writes its reports in `.claude/perf/<KEY>.md`. It never runs `sudo`: a measurement
that needs root is a command it hands you, for the user to run.

## CI and supply chain

- **`release-engineer` replaces the `coder`** on a step whose deliverable is a CI workflow or build
  policy file: `.github/` (`.github/workflows/`, `.github/dependabot.yml`), `justfile`,
  `cog.toml`, `deny.toml`, `clippy.toml`, `rustfmt.toml`, `rust-toolchain.toml`,
  `.cargo/config.toml`, `taplo.toml`, `_typos.toml`, `.editorconfig`. These files decide what the
  build and the CI accept, so the `coder` is refused them: a coder that loosens a lint turns a red
  check green without fixing anything. The step's verification criterion is a command
  (`actionlint`, `cog check`, `just check` or one of its recipes) that you rerun yourself at the
  gate. `actionlint` must be installed (`brew install actionlint`); without it, say that the
  workflow has not been validated.
  The workspace lints themselves live in the root `Cargo.toml`, which the coder can write: a
  change to `[workspace.lints]` is a point the `reviewer` must flag.
- **`devsecops` fixes nothing**: it covers the supply chain (dependencies and their provenance,
  `cargo deny`, the CI workflows' exposure, secrets) and writes `.claude/security/<KEY>.md`. You
  settle its findings like `RESERVATIONS`, toward the role it designates. Call it when a
  dependency is added or bumped, when `.github/workflows/` changes, and before the first release. It
  does not replace the `auditor`: one looks at the delivery chain, the other at the code.

## The `debugger` is not in the chain, it plugs into it

You call it when **something behaves in a way nobody can explain**:

- **The `coder` gives up** after three attempts on the same failure.
- **A failure whose cause seems foreign to the code**: a solver that times out on one machine
  only, an LLVM assertion, a link error against Homebrew's `llvm@22` or `z3`, a JIT crash.
- **A bug to reproduce** before the `tester` writes a non-regression test.

**It diagnoses, it does not repair.** Its report (`.claude/diagnostics/<topic>.md`) ends with
"whose move it is", with a single recipient. You then call that role, giving it the report's path.

Two recipients are not roles, and that is where you step in yourself. *The environment*: a
Homebrew package to reinstall, a toolchain to update, a stale `target/` to clean. Acting on a real
environment remains yours, or the user's when there is a choice or a risk. *Nothing to fix*: a
transient external cause is a legitimate result, you monitor instead of searching. **Always record
what you did to the environment**, in `.claude/state/<KEY>.md`.

What gets repaired **in the test harness** (a solver rlimit raised in a test, a snapshot filter)
remains the `tester`'s business, since the file concerned is a test file.

## The `researcher` fetches what the repository does not know

You call it when **a decision depends on an external fact**: what `inkwell` 0.10 exposes for a
given LLVM attribute, how Z3 handles a quantifier pattern, what changed in a crate version, what
an LLVM error message means. **It reports, it does not decide**, and it cites what it opened. Do
not call it for what the repository knows about itself: the installed version is in `Cargo.lock`,
and a `grep` answers better and for free.

## What the guards enforce in your place

Two `PreToolUse` hooks, wired in `.claude/settings.json`, refuse writes and commands depending on
the role requesting them (the `agent_type` of the event):

| Role | Cannot write | Held by |
| --- | --- | --- |
| `coder` | test files; `.claude/plans/`; build and CI policy files; `benches/`; `docs/adr/`, `docs/design.md`; no snapshot blessing | `path-guard.sh` |
| `tester` | anything that is not a test (it alone may bless snapshots) | `path-guard.sh` |
| `planner` | outside `.claude/plans/` | `path-guard.sh` |
| `reviewer` | outside `.claude/reviews/` | `path-guard.sh` |
| `debugger` | outside `.claude/diagnostics/` | `path-guard.sh` |
| `release-engineer` | outside `.github/`, `justfile`, `cog.toml`, `deny.toml`, `clippy.toml`, `rustfmt.toml`, `rust-toolchain.toml`, `.cargo/config.toml`, `taplo.toml`, `_typos.toml`, `.editorconfig`; in the shell, `gh` only as `gh run list/view/watch` | `role-guard.sh` |
| `devsecops` | outside `.claude/security/`; in the shell, nothing that writes or installs | `role-guard.sh` |
| `proof-engineer` | outside `.claude/proofs/` and `examples/**/*.lm`; in the shell, nothing that writes outside its reports | `role-guard.sh` |
| `perf-engineer` | outside `.claude/perf/`, `benches/`, `crates/*/benches/`; no `sudo` | `role-guard.sh` |
| `language-designer` | outside `docs/`; in the shell, nothing that writes | `role-guard.sh` |
| `ticket-brancher`, `committer`, `auditor`, `researcher` | nothing at all: no write tool | `tools:` |
| `ticket-reader` | no file; in the shell, only `gh` and `git` reads | `role-guard.sh` |
| `ticket-writer` | no file except drafts in `.claude/tmp/`; in the shell, a closed list of `gh` commands (`gh issue view/list/create/edit/comment`, `gh label list/create`, `gh api` GETs, comment PATCH); no assignee | `role-guard.sh` |
| `pr-writer` | no file except drafts in `.claude/tmp/`; in the shell, `gh pr view/list/create/edit/comment` (`create` only with `--head`, never `--dry-run`) and `git` reads | `role-guard.sh` |
| `pr-reviewer` | outside `.claude/reviews/pr/`; in the shell, `gh pr view/list/diff/checks/comment/merge`, `gh run list/view/watch`, `git` reads, and the checks (`just check` and its read-only recipes, `cargo fmt --check`, `cargo clippy`, `cargo nextest run`, `cargo test`, `cargo doc`, `cargo deny check`, `cog check`); a merge only with `--squash` or `--rebase` and `--match-head-commit`; never `--admin`, `--auto`, `--yes`, `--force` or `--fix` | `role-guard.sh` |

For **every** subagent, whatever its name:

- no write to the setup: `.claude/settings*.json`, `hooks/`, `agents/`, `skills/`, `tools/`,
  `rules/`, `orchestration.md`, `project.conf`, `test-paths.txt`, nor `CLAUDE.md` and
  `AGENTS.md`, which are the main session's instructions;
- no `ast-grep` with `-U` or `-i`: a mass rewrite the guard would see no path of;
- except the `ticket-brancher`, nothing that moves HEAD or rewrites the shared working tree:
  `git switch`, `git checkout <branch>`, `git stash` (but `list`/`show`), `git reset --hard`
  (or `--soft`, `--keep`, `--merge`), `git clean -f`, a checkout or restore of the whole tree
  (`.`, `:/`), `rebase`, `merge`, `pull`, `cherry-pick`, `revert`, `bisect`. Every role works in
  the same checkout, and one that switches branch drags another's uncommitted work along.
  Restoring named files (`git checkout <rev> -- <paths>`, `git restore <paths>`) stays allowed,
  and so does a mixed `git reset`, which the `committer` needs to unstage (`git reset HEAD
  <path>`): it keeps the working tree, but with a commit it also moves the branch, which only the
  role prompts forbid. A role that needs another branch says
  so, and you give it a worktree;
- no snapshot blessing (`cargo insta accept|review`, `INSTA_UPDATE`, `UPDATE_EXPECT`, `BLESS`,
  `TRYBUILD=overwrite`, `--bless`) except by the `tester`;
- no `gh` outside the four GitHub roles, except `gh run list/view/watch` for the
  `release-engineer`;
- outside the `coder` and the `tester`, no `cargo fmt`, `just fmt` or `taplo fmt` without
  `--check`, no `cargo fix`, no `--fix`, no `typos -w`, no `just setup`, no
  `cargo add|remove|update|install`.

**The guards fail closed.** Without `jq`, on an unreadable event, without a project anchor, or on
any internal error, they refuse. A guard that is absent without anyone knowing would be worse than
a noisy guard. `.claude/hooks/tests/run.sh` feeds them sample events; **rerun it after touching the
hooks, `test-paths.txt` or the wiring**: it is the only way to tell a guard that protects from one
that no longer exists.

**The shell remains a speed bump, not a wall.** The guards catch `sed -i`, a redirection, `mv`,
`cp`: the obvious forms. The real guarantee is the diff of the tests and specs against their
reference commit, rule 4.

**A guard refusal is not a breakdown.** If a role reports "write refused", the setup worked: read
what it wanted to do, and treat it as a signal. A `coder` that runs into a test file is telling you
it believes that test is wrong: that is rule 5.

You, on the other hand, are subject to nothing: the guards apply only to subagents. Restoring a
file, adding a `mod tests;` line, fixing the setup: that is your office.

## A permission refusal is not bypassed, it is escalated

When the harness refuses a read or a write, **it is a decision by the user, not a technical
obstacle**. Stop and escalate it. Do not look for another phrasing that would slip under the
filter: writing the content into a third-party file and then applying it with a command whose text
does not name the target; going through `git apply`, `sed`, a redirection or a variable; splitting
a refused path into pieces glued back together at execution. A filter that judges the command
string can always be bypassed; that does not make the bypass legitimate. **And declaring the bypass
after the fact does not redeem it**: you must stop *before*.

## Work in a worktree follows the same rules as here

A worktree is not a place where the setup loosens. Same guards, same exit gate, same passage
through the `committer`, same memory.

Since the setup is committed, a `git worktree add` already checks out the roles and the guards
(of the branch's base: a base older than the setup has none, and `worktree.sh` says so). What it
does **not** bring is the working memory, which is local: a bare worktree has no plan, no state,
no review, and a ticket would end up with two states.

**The tool: `.claude/tools/worktree.sh`**, `create <branch> [base]`, `remove <branch>`, `list`. It
creates the copy in `../clod-worktrees/`, then:

- **links**, one by one, the memory directories of the main tree: `.claude/plans`, `state`,
  `reviews`, `diagnostics`, `security`, `proofs`, `perf`, `tmp`, plus `settings.local.json` and
  `.env` when they exist;
- **shares the build directory**: it writes `../clod-worktrees/.cargo/config.toml` with
  `[build] target-dir = "<main tree>/target"`. Cargo merges the configuration files of every
  parent directory, the committed `.cargo/config.toml` sets no target directory, and so every
  worktree builds into the main tree's `target/`: `llvm-sys`, `inkwell` and `z3` are compiled once,
  not once per worktree. An environment variable would not do: it does not survive from one agent
  command to the next. The price is that two builds at the same time wait on Cargo's lock;
  `WORKTREE_TARGET_DIR` overrides the directory, `CARGO_TARGET_DIR` still wins over both;
- **checks** what is missing, and says so.

On removal, **the links go before `git worktree remove`**: git must not reach the main tree's
shared memory through them. The script does it.

**What is project-specific goes into a hook**: `.claude/tools/worktree-local.sh`, if it exists and
is executable, is called with `create|remove <branch> <worktree-path>`.

The main tree's **uncommitted** work does not follow into a new copy, which is why competing
passes have their own recipe, above.

## What leaves the repository: issues and pull requests

**A single token, hence a guard rather than an absent tool.** `gh` carries the account's write
rights for anyone who has the shell. What separates the roles is `role-guard.sh`: the
`ticket-reader` only has reads, the `ticket-writer` and the `pr-writer` a closed list of commands,
the `pr-reviewer` its checks and the merge, and **every other role is refused `gh`**, except
`gh run list/view/watch` for the `release-engineer`.

**The confirmation is the harness's permission prompt, and that is intended.** Only the reads
`gh issue view`, `gh issue list`, `gh pr view`, `gh pr list`, `gh pr checks`, `gh run list`,
`gh run view`, and `actionlint`, are in `allow`. **Do not add any write `gh` command** (issue
create/edit/comment, pr create/edit/comment/merge, `gh api` with a method or a field) to an
`allow` list: the prompt is
what submits each issue, each description and each merge to the user before it goes out. A refusal
by the user is a decision: the role stops and escalates it.

**A missing command is not a refusal.** When a role stops because it lacks a command, the fix is in
its prompt and in the guard's allowlist, not in a command you run yourself.

**Note the created keys in `.claude/state/`.** The `ticket-writer` looks for a duplicate before
creating, but that protection dies with its pass.

## A finished ticket goes all the way: pull request, review, merge

A ticket is not finished when its code is committed: it is finished when it is in `main`.

```
exit gate → committer → push → pr-writer → pr-reviewer → merge
```

- **Chain on without waiting to be asked.** The finished ticket calls for its pull request, the
  pull request for its review, the review for the merge.
- **You push the branch yourself.** An ordinary push is shown to the user before it goes out;
  force push, `--force-with-lease` and remote branch deletion are denied by `settings.json`.
- **`pr-writer` opens the pull request** and describes what the commits deliver, each acceptance
  criterion with how it was verified, what remains open, and `Closes #<n>`.
- **`pr-reviewer` decides whether it goes into `main`**, through seven checks in order: draft,
  base branch, CI checks, open review conversations, the checks it runs itself, agreement between
  description and diff, absence of conflict. Verdict `MERGEABLE`, `RESERVATIONS` or `BLOCKED`.
- **The merge is submitted to the user, and only after their confirmation**: `gh pr merge` is in
  no `allow` list, and the guard refuses the `pr-reviewer` any option that would skip the
  confirmation or bypass the branch protection (`--admin`, `--auto`, `--yes`, `--force`). The
  repository disallows merge commits: `--squash` or `--rebase` only, never guessed, with
  `--match-head-commit <sha>` so that only the commit that was checked lands, and a squash
  subject that is a Conventional Commit. The head branch is not deleted without an explicit
  request.
- **Code from a fork is untrusted.** The repository is public: a pull request from a fork is never
  checked out, built or tested on this machine, which holds the maintainer's credentials and whose
  guards are scripts read from the working tree. The `pr-reviewer` reads its diff and relies on
  CI, and a change to a policy path (`.github/`, `justfile`, `.claude/`, …) is never merged on
  green checks alone, since CI ran the pull request's own version of those files.
- **The existence of a CI is checked, not assumed** (`git show origin/main:.github/workflows/ci.yml`),
  and a pull request with no checks reported is not a green one.

## Commits carry the Claude co-author trailer

**Rule set by the user.** Every commit written through this setup ends with the trailer

```
Co-Authored-By: Claude <noreply@anthropic.com>
```

after a blank line, as a Conventional Commits footer (`cog verify` accepts it). The `committer`
adds it; it is the only attribution line. No `Generated with`, no session link, no mention of a
model in the subject or the body: the message talks about the change, the trailer says who wrote it.

## What goes out to GitHub carries no em dash

A style preference of the user's for what bears their name once published: commit messages,
pull request titles and descriptions, GitHub issues and comments contain no em dash (U+2014). A
comma, a colon, a parenthesis or an en dash say the same thing. The roles that write to the outside
carry it in their prompt; reread what you dictate to them. It does not apply to files of the
repository.

## Everything is written in English

`AGENTS.md`: all repository content is in English, code, comments, docs and commit messages. The
prompts you write to the roles are in English too: a repository drifts into the language of its
prompts, one identifier at a time. The research notes and reports kept outside Git are not
repository content.

## A forest of `if` / `else if` is a decision nobody has named

Three branches on the same question, or three levels of nested control flow (`if`, `match`,
`for`, `while`, `loop`): the chain must go before the commit. A single value compared against
constants becomes an exhaustive `match`, where the compiler checks that no case is missing.
Heterogeneous conditions become guards that exit at the top. Nesting is undone by early return,
`let … else` and `?`, and by nothing else.

The counterweight matters just as much: a table for two cases hides a line behind an indirection,
and `AGENTS.md` asks not to split a function only to make it shorter. What is required of a branch
is that it bear a **name**, not that it become a function. Below the threshold, the chain often
remains the clearest form.

**It only applies to code written in the pass.** A pre-existing forest the step passes through
stays in place; the `coder` reports it under *Points raised*, and you turn it into a ticket or
leave it.

On the test side, the same requirement, plus one of its own: no branch or loop in a `#[test]`
function. Sets of values go through a table of cases fed to one body, or distinct tests.

## A code comment does not narrate the workflow

What you did, at which step of the plan, under which directive: all of that goes into the report
and the commit message, never into the file. Same fate for the comment that announces its own
change ("added for...", "refactored") and for the paraphrase of the line below it.

What remains: the *why* that cannot be read in the code, a non-obvious invariant, a workaround with
its reason. **Above the function, the block or the constant, not in the body**, one line, two at
most, matching the density of the surrounding file. Two exceptions, both required by `AGENTS.md`:
doc comments (`///`, `//!`) on every public item, and the **`// SAFETY:` comment right above every
`unsafe` block**, inside the body by necessity, as long as the invariant needs.

## The ticket key is not written into files

Not in a comment, not in a test name, not in a file name, not in a doc, an ADR or a changelog.
`LEM-13` means nothing to whoever reads the repository in six months. What the key stood for is
said in plain terms, or not said at all. The ticket's trace lives in the branch name, the merge
request, and the loop's working files (`.claude/plans/`, `.claude/state/`, `.claude/reviews/`).

## The shape rules are checked by a command

`.claude/rules/check.sh <reference>` applies the rules above to the lines changed since the
reference only, on Rust files: `unwrap()` / `expect()` outside test files, an `unsafe` block
without its `// SAFETY:` comment, three levels of nested control flow, chains of three conditions,
a comment in a body, a branch or a loop in a test; and, on every file, the ticket key. File types
it has no rules for (`.lm`, `.toml`, `.md`…) are listed under "Not checked", never counted as
clean. The `coder` and the `tester` run it with `HEAD` before handing in, the `reviewer` with the
merge-base. A "new" finding is fixed in the pass; a "touched" finding is information.

Clippy enforces the same core rules over the whole workspace (`unwrap_used`, `expect_used`,
`undocumented_unsafe_blocks`, `missing_docs`, in `[workspace.lints]`; tests may unwrap, by
`clippy.toml`). `check.sh` gives the roles the answer on their own lines, without a build.

After any change to the rules, `ast-grep test -c .claude/rules/sgconfig.yml --skip-snapshot-tests`
must pass: a badly written rule returns nothing, silently. No role can modify them.

## Your memory: `.claude/state/<TICKET-KEY>.md`

A session ends, gets compacted, or closes at the wrong moment. **Keep a ticket state up to date**,
short, that the next session will read to resume:

```markdown
# LEM-13 state

Branch: LEM-13-<slug>, started from LEM-12-<slug> (merge-base <hash>)
Plan: .claude/plans/LEM-13.md
Open reservations: see .claude/reviews/LEM-13.md
Explicitly discarded: <what the user decided, one line each>

| Step | State | Tests commit |
| --- | --- | --- |
| 1. Parse loop carries | green, 41/41 | <hash> |
| 2. Lower carries to phi nodes | red tests committed | <hash> |
| 3. Prove carry bounds | to do | — |

Where we are: step 2, coder not launched; stop between steps, waiting
```

One row per step, with a state among "to do", "red tests committed", "green", "reworked", and
**the tests commit hash in its step's row**: it is the reference for rule 4, and it changes at
every step. Update it after the tests commit, after a coder pass, after a verdict.

It stays **local**, like the plan, the reviews and the reports: `.gitignore` covers them. None of
this goes into a commit.

**When you resume, compute before you trust.** A state file says what was true when it was
written. `.claude/tools/status.sh` prints what is true now: branch and distance to `origin/main`,
uncommitted changes, worktrees, open pull requests and their checks, the last CI run on `main`,
and the "Where we are" line of each state file. `.claude/tools/ticket-graph.sh` lists the open
tickets with no open blocker, from GitHub's "blocked by" relationships, those that unblock the
most first; a blocker written only in an issue's prose does not show there, so the
`ticket-reader` still reads the ticket you pick.

## Resuming a subagent or launching a fresh one

An instance serves **one role for one ticket**, and `SendMessage` does not change the role.

**Resume for the continuation of the same work:** the `coder` and the `tester` from one step to the
next of the same plan; the `planner` to revise the plan of the same code; the `reviewer` for its
follow-up pass, where its review file is what counts.

**Launch a fresh one** when the ticket changes; when the task changes nature; when judgment must
stay independent (the `reviewer`, the `auditor` and the `proof-engineer` are never an instance that
saw the code being born, and the `tester` and the `coder` are never the same instance); or when the
context is polluted.

## Handoff rules

1. **The `ticket-reader`'s report is authoritative.** The following roles do not go back to query
   GitHub. If the report is insufficient, relaunch the `ticket-reader`.
2. **The plan is written to a file, and that file is what circulates.** The `planner` writes
   `.claude/plans/<TICKET-KEY>.md`; **submit it to the user and wait for their review** before
   anything downstream consumes it. Then pass the **path** to the `tester`, the `coder` and the
   `reviewer`, not a paraphrase. The plan is neither committed nor pushed.

   **A plan written in advance is revalidated before being tackled**: reread its "current state"
   section and check the anchors it names (`path:line`, decision and where it is recorded). If an
   anchor has moved, send it back to the `planner` with what changed.

   Without a ticket key, it writes no file and you pass its report along as text.
3. **You do not work on the default branch.** Before launching the `tester` or the `coder`, check
   `git branch --show-current`. If it is `main`, **stop**: launch the `ticket-brancher` first, or
   ask the user whether writing directly is intended.
4. **Check between the `tester` and the `coder`: tests and specs are frozen.** After the coder's
   pass, run `.claude/tools/frozen-check.sh <tests-commit>` (exit 0 frozen, 1 moved). Against the
   working tree, untracked files included, it checks three things:
   ```bash
   git diff --stat <tests-commit> -- <test paths>                  # 1. the test files
   git diff <tests-commit> | grep -E '^-[[:space:]]*(#\[cfg\(test\)\]|mod tests;)'   # 2. the wiring
   # 3. the contract of every .lm function, from its fn line to its impl line
   ```
   The first covers the test files the commit froze; the second a removed wiring line, in a
   production file the coder otherwise edits freely; the third the contract of every `.lm`
   function: the signature with its types and bounds, and the `spec`, `req`, `ens`, `eff` and
   `equiv` clauses, which the coder can write (the impl lives beside the spec) but must not
   change. It compares each function's contract with comments removed and whitespace collapsed,
   so a signature or a clause spread over several lines is covered whole, and a pure layout change
   is not a move; a contract keyword found outside every contract (a clause after an `impl`) counts
   too, and a changed path found neither at the commit nor in the tree stops the check (exit 2). A new function shows as a new contract, and `frozen-check.sh` exits 1 on it too:
   you read it, and accept it only if the plan's step called for that function. **If any of the
   three moved, except for a new function accepted as above, the pass is rejected**: restore from
   that commit
   (`git checkout <tests-commit> -- <paths>`, or the spec lines by hand) and relaunch the coder,
   telling it so. The guard covers the tests, not the specs, and the shell imperfectly: this check
   is what proves it, and it costs a second. If one day it bites, that is information of the first
   importance.
5. **A test or a spec the `coder` reports as wrong is a decision, not a detail.** It cannot fix it,
   and its instruction is to stop. Decide yourself, or send the question back to the `planner`. If
   a test must change, the `tester` fixes it; if a spec must change, the `proof-engineer` does,
   with the `language-designer` if the change touches semantics. Then have them recommitted and
   **note the new hash**.
6. The `committer` steps in on work that is finished, green and reviewed, never as the first
   response to "commit". Its only intervention mid-loop is the red tests commit.
7. **Every commit goes through the `committer`.** Do not commit yourself: a second path to `git
   commit` would make the exit gate negotiable instead of verifiable.

## The red tests commit

When the `tester` has finished and its tests fail for the right reason, **have them committed
alone, before launching the `coder`.** It is the only commit that does not go through the exit
gate. It makes the tests a Git object: `git diff <tests-commit> -- <test paths>` can be checked by
anyone, at any moment.

**The exception is triggered by a state, never by a request.** Three conditions: the `tester` has
just run, **its pass touched only test paths**, and the suite is red with the expected failure.
What it contains: the paths the `tester` reports, the wiring lines you added for them (`mod
tests;`, a dev-dependency), and, when the step brings a new spec, the `.lm` file the
`proof-engineer` wrote. Never `git add -A`.

A red suite in Rust usually means **a build error**: a test that calls a function which does not
exist yet does not compile. That is the expected failure for a surface to be created, provided the
error names exactly the symbol of the directives; say that the assertions have not bitten yet.

**Before that commit, look at the volume.** If the pass adds more than twice as many lines of tests
as the step announces lines of code, ask the `tester` to justify the gap test by test.

Call the `committer` telling it what this is: single commit, the listed paths only, expected red
suite. **Note the hash** in the step's row of `.claude/state/`.

**If a hook refuses the commit because the suite is red**, there is nothing to force and
`--no-verify` is not an answer. (Today the only hook is cocogitto's `commit-msg`, which checks the
message, not the suite.)

## The exit gate, before any commit

**It is triggered by the intent to commit, not after the loop.** Whether the request arrives at
the end of the chain or from a cold "commit", the order is the same: the checks, then the
`committer`.

**First, which branch are you on?** If it is `main`, **stop**: work is committed on a ticket branch
and goes out as a pull request. Launch the `ticket-brancher`, or ask the user, and if they confirm,
**tell the `committer`**, which will otherwise refuse.

Then, in this order. None is skipped because "it looks fine".

1. **The repository's checks pass**, run by you and read, not taken from the coder's report. It
   is the single command of `AGENTS.md`, the one whose recipes the CI runs:
   ```bash
   just check    # fmt-check, lint, test, doc, deny, spell, rules
   ```
   When it fails, rerun the failing recipe alone (`just lint`, `just test`, …) to read its output.
   Red ⇒ back to the `coder`, with the error output.
2. **Tests and specs are intact** since the red tests commit (rule 4).
3. **The `reviewer` returns its verdict.** Pass it the ticket report, the plan's path, the base and
   the tests commit hash.

   `NON-COMPLIANT` ⇒ back to the `coder` with its list. `RESERVATIONS` ⇒ up to you to decide **point
   by point**: retained ones go to the `coder` (or the `tester` for a test), discarded ones are
   discarded explicitly in `.claude/state/`. Then call the `reviewer` again: it reopens its memory.
   **`INCOMPLETE` is not a green light**: it could not perform the check.

**In parallel with the `reviewer`**, as the change calls for them; they deliver findings, not
verdicts, settled like `RESERVATIONS`:

- the **`auditor`**: unsafe code and the JIT, a new IR lowering, anything concurrent, a large or
  tangled diff, user input reaching the parser or the solver, or a request for an audit. It gets
  the scope only (ticket, base, plan path), never the coder's reasoning;
- the **`proof-engineer`**: `crates/lemma-verify/` or a contract clause of a `.lm` file changed;
- the **`perf-engineer`**: perf-labelled work, or a change to `crates/lemma-codegen/` or
  `crates/lemma-jit/`;
- the **`devsecops`**: a dependency added or bumped, `.github/workflows/` changed.

The `reviewer` answers "does this do what the ticket asked"; the others "is it well done", "can the
proof be trusted", "is it as fast as claimed", "is the supply chain sound". Perfectly compliant
code can be bad.

### Proportionate to the change

- **No code touched** (documentation, comments): neither suite nor reviewer; a glance at the
  `git diff`, and `cargo doc` if a doc comment moved.
- **Code touched, outside a ticket**: run the checks, skip the `reviewer`.
- **Work coming out of the chain**: everything above, without exception.

### The waiver is explicit, or it does not exist

The user can ask to override: "commit directly", "without review". Do it then, and **say in one
line what was not verified**. What is never allowed is skipping the exit gate on your own
initiative.

Only once these checks are cleared, or explicitly lifted, is the `committer` called.
