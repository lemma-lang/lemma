---
name: lemma-checks
description: Use before committing, pushing, opening a pull request, or claiming that checks pass on a Lemma branch. Says which command proves what (just check and its recipes, the shape rules of tools/rules/check.sh, cog for commit messages), how to read a failure, and what to verify about tests and specs before handing work over.
---

# Lemma checks

The [`justfile`](../../../justfile) is the single source of truth for commands, and CI runs the
same recipes. A change is done when `just check` passes on it, run by you and read, not assumed
from an earlier run.

## Before you start

```sh
just setup            # installs the cocogitto git hooks, checks the toolchain
git fetch origin      # branch from an up-to-date origin/main, never a stale local main
```

macOS needs `brew install llvm@22 z3`; the paths are set in `.cargo/config.toml`. The hooks
check the commit message on commit, formatting and spelling before commit, and run `just check`
before push.

## The checks

| Command | Proves | Run it when |
| --- | --- | --- |
| `just check` | everything CI runs: `fmt-check lint test doc deny spell rules` | before every push, and before saying the work is done |
| `just fmt` | formats Rust (`cargo fmt`) and TOML (`taplo fmt`) | before `just check`; it is the only recipe that writes |
| `just lint` | `cargo clippy --workspace --all-targets -D warnings` | a warning is an error in CI |
| `just test` | `cargo nextest run` plus doc-tests | for one crate while iterating: `cargo nextest run -p <crate>` |
| `just doc` | rustdoc with `-D warnings`; `missing_docs` is denied | after touching a public item or a doc comment |
| `just deny` | licenses, advisories, sources of dependencies | after any change to `Cargo.toml` or `Cargo.lock` |
| `just spell` | `typos` over the tree | prose changed |
| `just rules` | the shape rules' own test cases | after editing `tools/rules/` |
| `tools/rules/check.sh <ref>` | the shape rules on the lines changed since `<ref>` | before handing in: `HEAD` for uncommitted work, the merge-base for a whole branch |
| `cog verify "<message>"` / `cog check <base>..HEAD` | Conventional Commits | before committing / before pushing |

When `just check` fails, rerun the failing recipe alone (`just lint`, `just test`, …) to read its
output. Quote error codes as the tool prints them (`E0308`, `clippy::unwrap_used`).

## The shape rules

`tools/rules/check.sh <ref>` applies the code-style rules of [`AGENTS.md`](../../../AGENTS.md) to
the lines you changed only, without a build:

- `unwrap()` / `expect()` outside test files;
- an `unsafe` block without its `// SAFETY:` comment;
- three levels of nested control flow, or a chain of three conditions;
- a comment inside a function body;
- a branch or a loop inside a `#[test]` function;
- an issue key (the `LEM` prefix followed by a number) in a file's content or name.

A **new** finding (its first line is yours) is fixed before handing in. A **touched** finding is a
pre-existing problem your change passes through: mention it, do not widen the change to fix it.
File types without rules (`.lm`, `.toml`, `.md`) are listed under "Not checked": read those
yourself, "0 findings" does not cover them.

## Tests and specs are not moved to make the suite green

When tests were written before the implementation (see [lemma-testing](../lemma-testing/SKILL.md)),
they are the specification. Before handing in, check that the implementation pass left them
alone:

```sh
git diff --stat <tests-commit> -- '*tests.rs' '*/tests/*' '*/snapshots/*' '*/testdata/*' '*/fuzz/*'
git diff <tests-commit> -- '*.lm'     # read every fn signature and spec/req/ens/eff/equiv line
```

Both must be empty, except for what the change explicitly set out to alter. A weakened test, a
loosened spec or a removed `#[cfg(test)] mod tests;` line turns wrong work green.

## Commits and pull requests

- [Conventional Commits](https://www.conventionalcommits.org/), scope = crate name without the
  prefix: `feat(syntax): parse loop carries`. Types: `feat`, `fix`, `docs`, `refactor`, `perf`,
  `test`, `build`, `ci`, `chore`.
- Everything in English. No issue key in a file; reference the issue in the pull request
  (`Closes #12`).
- The repository accepts squash or rebase merges only, never merge commits.
- A pull request states what was verified and how, and what was not.
