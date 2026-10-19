# Lemma

A language designed to be written by an AI: low-level in control, high-level in semantics.

Every function carries a **spec** (what it computes, in the obvious way) and an **impl** (how, optimized).
The compiler proves they are equivalent before generating native code, like lemmas stacking on one another.

```
fn dot[n:u64] (a: &f32[n] @a128 ro, b: &f32[n] @a128 ro) -> f32
  eff  pure
  req  n % 16 == 0
  spec Σ(i<n) a[i]*b[i]
  impl ...
  equiv spec ≈ impl  allow(fp_reassoc)
```

## Principles

- **Pure SSA**: every value is defined once, no hidden state.
- **Types carry memory layout**: vector width, alignment, bounds (`i: u64 < n`).
- **Explicit effects**: `pure`, `ro`, `wo`, disjoint regions. No alias analysis guesswork.
- **Separate spec / impl**, equivalence proven by SMT (Z3).
- **Nothing implicit**: floating-point reassociation, overflow and traps are opt-in.
- **Hash-addressed functions**, keyed by their AST.
- **Typed CPU resources**, like memory: cores, caches, vector units (see [docs/design.md](docs/design.md)).
- **Two levels of effects**: static and free for compute kernels, runtime-reversible (Cordis paradigm) for hot-reloaded components.

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

Requirements (macOS):

```
brew install llvm@22 z3
cargo test
```

`inkwell` 0.10 supports LLVM up to version 22. Homebrew paths are set in `.cargo/config.toml`.
