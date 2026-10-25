# AGENTS.md

Instructions for AI agents working on this repository. Humans are welcome to read it too.

## What this is

Lemma is a language written and read by AI. See [README.md](README.md) for the vision
and [docs/design.md](docs/design.md) for the language design. Architecture decisions and their
reasons live in [docs/adr/](docs/adr/): read them before changing anything they cover.

## Repository map

| Path | Content |
|---|---|
| `crates/lemma-diagnostics` | Diagnostic records, their JSON and text renderings |
| `crates/lemma-syntax` | Lexer, parser, AST |
| `crates/lemma-verify` | Type checking, proofs (Z3) |
| `crates/lemma-codegen` | LLVM IR emission (inkwell, LLVM 22) |
| `crates/lemma-jit` | In-memory JIT, hot replacement, cost measurement |
| `crates/lemma-cli` | `lemma` binary |
| `examples/` | Lemma source examples (`.lm`) |
| `docs/` | Design notes and ADRs |
| `.agents/skills/` | Task skills for agents (see [Skills](#skills)) |
| `tools/rules/` | Shape rules for Rust code, checked on changed lines |

## Commands

The [`justfile`](justfile) is the single source of truth for commands; run `just` to list them.

```
just check                   # everything CI runs: fmt, lint, test, doc, deny, spell, rules
tools/rules/check.sh HEAD    # shape rules on the lines changed since HEAD
just fmt                     # format Rust and TOML
just test                    # cargo nextest + doc-tests
cog verify "<message>"       # check a commit message
```

`just check` must pass before a change is considered done. CI calls the same recipes. Before
pushing, follow [lemma-checks](.agents/skills/lemma-checks/SKILL.md).

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
- A change that reverses or extends an architecture decision adds or supersedes an ADR
  ([lemma-language-design](.agents/skills/lemma-language-design/SKILL.md)).

### Code style

Write code an agent can read in one pass:

- Explicit, descriptive names. No abbreviations beyond the domain's own (`ssa`, `ir`, `smt`).
- One way of doing each thing; follow the pattern already used nearby.
- Keep related code together. Unit tests live in a sibling file: each module declares
  `#[cfg(test)] mod tests;`, which resolves to `tests.rs` in the module's directory
  (`src/tests.rs` for `lib.rs` and `main.rs`, `src/parser/tests.rs` for `src/parser.rs`).
  Integration tests go in `crates/*/tests/`, snapshot and golden files in `snapshots/` or
  `testdata/` directories ([lemma-testing](.agents/skills/lemma-testing/SKILL.md)).
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

## Skills

Task knowledge lives in [`.agents/skills/`](.agents/skills/), one `SKILL.md` per task, in the
open Agent Skills format; Claude Code finds them through the links in `.claude/skills/`. Load the
one that matches the task before starting it:

| Skill | Use when |
|---|---|
| [lemma-checks](.agents/skills/lemma-checks/SKILL.md) | before committing, pushing or claiming checks pass |
| [lemma-testing](.agents/skills/lemma-testing/SKILL.md) | writing, placing or reviewing tests |
| [lemma-specs-and-proofs](.agents/skills/lemma-specs-and-proofs/SKILL.md) | writing a contract in a `.lm` file, changing `lemma-verify`, a proof slow, flaky or unknown |
| [lemma-code-review](.agents/skills/lemma-code-review/SKILL.md) | reviewing a pull request or a branch |
| [lemma-language-design](.agents/skills/lemma-language-design/SKILL.md) | changing the language, an ADR, or described behavior |
| [lemma-perf](.agents/skills/lemma-perf/SKILL.md) | measuring, benchmarking, changing `lemma-codegen` or `lemma-jit` |

A skill applies this file's rules to one task; when the two disagree, this file wins and the
skill is fixed.
