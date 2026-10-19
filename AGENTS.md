# AGENTS.md

Instructions for AI agents working on this repository. Humans are welcome to read it too.

## What this is

Lemma is a language written and read by AI. See [README.md](README.md) for the vision
and [docs/design.md](docs/design.md) for the language design. Architecture decisions and their
reasons live in [docs/adr/](docs/adr/): read them before changing anything they cover.

## Repository map

| Path | Content |
|---|---|
| `crates/lemma-syntax` | Lexer, parser, AST |
| `crates/lemma-verify` | Type checking, proofs (Z3) |
| `crates/lemma-codegen` | LLVM IR emission (inkwell, LLVM 22) |
| `crates/lemma-jit` | In-memory JIT, hot replacement, cost measurement |
| `crates/lemma-cli` | `lemma` binary |
| `examples/` | Lemma source examples (`.lm`) |
| `docs/` | Design notes and ADRs |

## Commands

The [`justfile`](justfile) is the single source of truth for commands; run `just` to list them.

```
just check                   # everything CI runs: fmt, lint, test, doc, deny, spell, rules
just fmt                     # format Rust and TOML
just test                    # cargo nextest + doc-tests
cog verify "<message>"       # check a commit message
```

`just check` must pass before a change is considered done. CI calls the same recipes.

## Rules

### Language

All repository content is in English: code, comments, docs, commit messages.

### Commits

[Conventional Commits](https://www.conventionalcommits.org/), enforced by cocogitto
(`cog install-hook --all` installs the local hook). Types: `feat`, `fix`, `docs`, `refactor`,
`perf`, `test`, `build`, `ci`, `chore`. Scope is the crate name without the prefix when relevant:
`feat(syntax): parse loop carries`.

### Documentation stays true

- Every public item has a doc comment (`missing_docs` is denied in `[workspace.lints]`).
- Examples in doc comments are doc-tests: they run with `cargo test`, so they cannot rot.
- A change that alters behavior described in `README.md`, `docs/design.md` or this file updates
  that file in the same commit.
- A change that reverses or extends an architecture decision adds or supersedes an ADR.

### Code style

Write code an agent can read in one pass:

- Explicit, descriptive names. No abbreviations beyond the domain's own (`ssa`, `ir`, `smt`).
- One way of doing each thing; follow the pattern already used nearby.
- Keep related code together. Unit tests live in a sibling file: each module declares
  `#[cfg(test)] mod tests;`, which resolves to `tests.rs` in the module's directory
  (`src/tests.rs` for `lib.rs` and `main.rs`, `src/parser/tests.rs` for `src/parser.rs`).
  Integration tests go in `crates/*/tests/`, snapshot and golden files in `snapshots/` or
  `testdata/` directories.
- Prefer simple, direct code over layers of abstraction. Do not split a function only to make it shorter.
- Errors are structured values carrying a location and, for proofs, the Z3 counterexample.
  No `unwrap()` or `expect()` outside tests (`clippy::unwrap_used` and `clippy::expect_used` are
  denied; `clippy.toml` allows them in tests).
- `unsafe` only in `lemma-jit`, each block with a `// SAFETY:` comment
  (`clippy::undocumented_unsafe_blocks` is denied).

### Environment

macOS needs `brew install llvm@22 z3`; paths are set in `.cargo/config.toml`.
CI runs on GitHub Actions (see `.github/workflows/ci.yml`): Linux in a Debian trixie container
with LLVM 22 from apt.llvm.org, and macOS on Apple Silicon.

## Agent orchestration

Claude Code runs a chain of specialized subagents on this repository (ticket, plan, tests,
implementation, review, proofs, performance, pull request), defined in `.claude/agents/` and
held to their zones by the hooks in `.claude/hooks/`. The policy, the roles and the exit gate
before every commit are described in [.claude/orchestration.md](.claude/orchestration.md),
which `CLAUDE.md` imports. GitHub issue `#n` is ticket `LEM-n`.
