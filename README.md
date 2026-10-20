<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo-mark-dark.svg">
    <img src="assets/logo-mark-light.svg" width="112" alt="Lemma logo">
  </picture>
</p>

# Lemma

A language written and read by AI: low-level in control, high-level in semantics.

Every function carries a **spec** (what it computes, in the obvious way) and an **impl** (how, optimized).
The compiler proves they are equivalent before generating native code, like lemmas stacking on one another.

Lemma targets performance-critical code that must be proven correct (control loops, kernels, signal
processing, cryptography) and is callable from any language through the C ABI or WebAssembly.
See [ADR 0002](docs/adr/0002-scope.md).
Exported functions check their preconditions at the boundary and return a status; an `_unchecked`
variant skips the checks for callers that guarantee them ([ADR 0005](docs/adr/0005-c-abi-boundary.md)).

```
export fn dot(n: u64, a: &f32[n] @a128 ro, b: &f32[n] @a128 ro) -> f32
  eff pure
  req n % 16 == 0
  spec sum(i: u64 in 0..<n) { a[i] * b[i] }
  equiv allow(fp_reassoc, fp_contract)
  impl { ... }
```

The prototype grammar is in [docs/grammar.md](docs/grammar.md).

## Principles

- **Pure SSA**: every value is defined once, no hidden state.
- **Types carry memory layout**: vector width, alignment, bounds (`i: u64 < n`).
- **Explicit effects**: `pure`, `ro`, `wo`, disjoint regions. No alias analysis guesswork.
- **Separate spec / impl**, equivalence proven by SMT (Z3).
- **The spec is what gets reviewed**: one line says *what* a function computes; the proof
  guarantees the optimized impl matches it, exactly or within the relaxations its `allow` list
  names, so the impl itself needs no review.
- **A wrong spec is caught too**: an independent model reviews each contract against the task, and the
  spec is executed against test vectors ([ADR 0006](docs/adr/0006-optimized-for-its-writer.md)).
- **Every surface optimized for the models that write and read it**, not for human taste.
- **Nothing implicit**: floating-point reassociation and contraction, overflow and traps are opt-in.
- **Hash-addressed functions**, keyed by their AST.
- **Typed CPU resources**, like memory: cores, caches, vector units (see [docs/design.md](docs/design.md)).
- **Two levels of effects**: static and free for compute kernels; runtime-reversible effects
  (Cordis paradigm) for hot-reloaded stateful components are a future direction, not designed yet.

## Development loop

```
source ─► verify (Z3) ─► IR ─► in-memory JIT ─► spec vs impl tests ─┐
   ▲                                                               │
   └──────── hot fix (only the changed function) ◄─────────────────┘
                 tests green ─► emit final binary (same target options)
```

## Layout

| Crate | Role |
|---|---|
| `lemma-syntax` | Lexer, parser, AST |
| `lemma-verify` | Type checking, bounds and equivalence proofs (Z3) |
| `lemma-codegen` | LLVM IR emission (`noalias` metadata, alignment) |
| `lemma-jit` | In-memory JIT daemon, hot replacement, performance counters |
| `lemma-cli` | `lemma` entry point |

## Status

Skeleton only: no crate implements its role yet.
LLVM (`inkwell`) and Z3 are wired in, each checked by a linking test.

## Development

Prerequisites (macOS, Apple Silicon):

```
brew install llvm@22 z3 just typos-cli taplo cargo-nextest cargo-llvm-cov ast-grep
cargo install --locked cocogitto cargo-deny
just setup                   # installs the git hooks
```

`inkwell` 0.10 supports LLVM up to version 22. Homebrew paths are set in `.cargo/config.toml`,
and the Rust toolchain is pinned in `rust-toolchain.toml`.

Every task goes through [`just`](justfile); `just` alone lists them.

| Command | What it does |
|---|---|
| `just check` | Everything CI runs: format, lint, tests, docs, `cargo deny`, spelling, agent rules |
| `just fmt` | Format Rust and TOML files |
| `just test` | Tests with `cargo nextest`, then doc-tests |
| `just cov` | Coverage report in `target/llvm-cov/html` |

Commits follow [Conventional Commits](https://www.conventionalcommits.org/), checked by cocogitto.
The git hooks check the message on commit, formatting and spelling before commit, and run
`just check` before push.

## Working with AI agents

Lemma is built by AI agents as much as for them. [`AGENTS.md`](AGENTS.md) holds the rules every
agent follows. Claude Code runs a chain of specialized roles, defined in `.claude/agents/`:
the main session dispatches them, failing tests are written and committed before any code,
and every change passes an exit gate (full suite green, tests untouched, review) before it is
committed. Besides the usual roles (planner, tester, coder, reviewer, committer…), three are
specific to Lemma: `proof-engineer` (Z3 obligations and spec quality), `perf-engineer`
(measurements on Apple Silicon) and `language-designer` (ADRs and scope).
Hooks in `.claude/hooks/` keep each role inside its zone. Details: [.claude/orchestration.md](.claude/orchestration.md).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project,
as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
