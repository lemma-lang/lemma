# AGENTS.md

Instructions for AI agents working on this repository. Humans are welcome to read it too.

## What this is

Lemma is a language designed to be written by an AI. See [README.md](README.md) for the vision
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

```
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps          # must build without warnings
cargo deny check
cog verify "<commit message>"            # check a commit message
```

All of them must pass before a change is considered done. CI runs the same checks.

## Rules

### Language

All repository content is in English: code, comments, docs, commit messages.

### Commits

[Conventional Commits](https://www.conventionalcommits.org/), enforced by cocogitto
(`cog install-hook --all` installs the local hook). Types: `feat`, `fix`, `docs`, `refactor`,
`perf`, `test`, `build`, `ci`, `chore`. Scope is the crate name without the prefix when relevant:
`feat(syntax): parse loop carries`.

### Documentation stays true

- Every public item has a doc comment (`missing_docs` is denied).
- Examples in doc comments are doc-tests: they run with `cargo test`, so they cannot rot.
- A change that alters behavior described in `README.md`, `docs/design.md` or this file updates
  that file in the same commit.
- A change that reverses or extends an architecture decision adds or supersedes an ADR.

### Code style

Write code an agent can read in one pass:

- Explicit, descriptive names. No abbreviations beyond the domain's own (`ssa`, `ir`, `smt`).
- One way of doing each thing; follow the pattern already used nearby.
- Keep related code together, with unit tests in the same file (`#[cfg(test)] mod tests`).
- Prefer simple, direct code over layers of abstraction. Do not split a function only to make it shorter.
- Errors are structured values carrying a location and, for proofs, the Z3 counterexample.
  Never `unwrap()` on user input.
- `unsafe` only in `lemma-jit`, each block with a `// SAFETY:` comment.

### Environment

macOS needs `brew install llvm@22 z3`; paths are set in `.cargo/config.toml`.
CI runs on Linux with LLVM 22 from apt.llvm.org (see `.gitlab-ci.yml`).
