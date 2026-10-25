---
name: lemma-testing
description: Use when writing, placing or reviewing tests in the Lemma repository (unit, integration, snapshot and golden files, fuzz targets, differential spec-versus-impl, non-regression), or when a new test file seems not to run. Covers the sibling tests.rs convention and its wiring, red-before-green, and what a test may and may not contain.
---

# Testing Lemma

## Where each test lives

| Kind | Place |
| --- | --- |
| Unit | a sibling file: the module declares `#[cfg(test)] mod tests;`, which resolves to `tests.rs` in the module's directory (`src/tests.rs` for `lib.rs` and `main.rs`, `src/parser/tests.rs` for `src/parser.rs`) |
| Integration | `crates/<crate>/tests/<surface>.rs`, one file per surface, through the crate's public API |
| Snapshot, golden | `snapshots/` or `testdata/` directories (expected IR, expected diagnostics, `.lm` inputs) |
| Fuzz | `fuzz/`, cargo-fuzz layout |

**A new `tests.rs` is compiled only once its module declares `#[cfg(test)] mod tests;`.** Without
that line `cargo test` stays silent: no failure, no test. The same holds for a dev-dependency
(`insta`, `proptest`, `libfuzzer-sys`) in `Cargo.toml`. Add the wiring with the test, and check
that the test actually ran (its name appears in the output). Integration tests in `tests/` are
discovered by Cargo and need no wiring.

## Red before green

1. Read the code under test and its existing tests first: real signatures, error variants,
   fixtures, how `.lm` sources are embedded. Tests written against imagined signatures make the
   implementation chase a hallucination.
2. Search for a test that already proves the rule. Extend it (one more row in its table) rather
   than writing a second one.
3. Write the tests and run only them: `cargo nextest run -p <crate> <module>::tests`,
   `cargo nextest run -p <crate> --test <file>`.
4. Observe the failure. For a surface that does not exist yet, the only reachable red in Rust is a
   compile error (`E0425`, `E0432`) naming exactly the missing item: say that the assertions have
   not bitten yet. For an existing surface, expect an assertion failure.
5. Only then write the implementation, without touching the tests. When the tests are committed
   separately first, that commit is the reference [lemma-checks](../lemma-checks/SKILL.md) diffs
   against.

A non-regression test starts from a reproduced bug and must first fail the same way the bug does.

## What a test contains

- **One behavior per test**, named after the behavior. Setup, action, verification, in that order.
- **No branch or loop in a `#[test]` function** (`if`, `match`, `for`, `while`, `loop`;
  `tools/rules/check.sh` flags them). A set of values is a table of cases (a `const` array of
  tuples fed to one body, or one test per case calling a shared helper), each case named from its
  data. Values needing different assertions are distinct tests. Assert an error variant with
  `assert!(matches!(result, Err(Error::Bounds { .. })))`.
- **No weakening**: no tautological assertion, no `#[ignore]`, no `#[should_panic]` without
  `expected = "…"`, no `let _ =` that swallows the failure under test.
- **No redundancy**: one rule, one proof, at the lowest level that carries it (a verifier rule in
  `lemma-verify`, not again through the CLI).
- Tests may `unwrap()` and `expect()` (`clippy.toml` allows it): it says the setup cannot fail.
- No comment in a test body, no ticket key in a test or file name. One line above a non-obvious
  setup is the most a test file needs.
- Errors are asserted with their location: a diagnostic without its span is half a test.

## Kinds specific to Lemma

- **Snapshot**: review every new snapshot before accepting it (`cargo insta review`). A snapshot
  that freezes wrong output is a wrong test.
- **Fuzz target**: a `fuzz_target!` feeding arbitrary input to the parser or the verifier and
  asserting it never panics. Build with `cargo +nightly fuzz build`, run briefly to see it start,
  never commit a corpus that holds secrets.
- **Differential spec-versus-impl**: the impl run through the JIT against the spec evaluated on
  the same generated inputs, plus the known-wrong outputs the spec must reject (see
  [lemma-specs-and-proofs](../lemma-specs-and-proofs/SKILL.md)). For floating point, use exactly
  the tolerance the `equiv` clause allows, never a looser one.
