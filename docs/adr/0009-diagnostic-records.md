# 9. Diagnostic records: repairs as edit lists, the proof kinds, and the exact renderings

- Status: accepted
- Date: 2026-10-20

## Context

ADR 0006 makes every diagnostic a record (P5) and leaves the model's rendering of it to a
calibration between R, the record as one JSON object, and T, a familiar compiler text (C2). It names
the record's fields and their meaning. It does not fix what a producer, a renderer, the experiment
harness and a test need to agree on byte for byte: the type of each field, how a position is
counted, which fields a kind may carry, the exact bytes of each rendering, and the order of records.
Left to the code, each would be decided silently, and the harness of ADR 0004, which reads failure
classes from these records, would depend on an implementation detail.

Three questions go further than format:

- **A repair that touches two places.** P5 defines a repair as one edit, a span and its replacement
  text. `bound-in-clause` (`docs/grammar.md`, section 7) moves a bound from a `req` clause into a
  parameter's type: one insertion in the signature, one deletion of the clause, several lines apart.
  As one edit it must replace the whole text between them, contract lines it does not change
  included, and the text a writer applies is then mostly a copy of what is already there.
- **The kinds of the proofs, the solver and the toolchain.** The grammar lists the syntax kinds and
  the checker kinds of names and bounds, and ADR 0007 the export refusals. No kind exists yet for
  a failed bound proof, a failed equivalence, a failed `ens`, a solver `unknown` or timeout, or a
  fault of the compiler itself, though ADR 0004 counts each of these causes.
- **Where a failed `ens` is counted.** ADR 0004's classes include a bound proof and an equivalence
  proof, and an `ens` is neither a bound nor the `equiv` clause.

## Decision

The normative format is [`docs/diagnostics.md`](../diagnostics.md): fields and their types, span
conventions, repairs, the order of records, the R and T renderings, and the closed set of kinds with
their classes. The choices it makes beyond ADR 0006:

1. **A repair is a non-empty list of edits, applied together, all or none.** Each edit is a span of
   the text as submitted and its replacement. Edits are sorted by file, then start byte, and two
   edits in one file are separated: the first ends strictly before the second begins. So edits never
   overlap, two insertions never compete for one position, and an insertion that touches a replaced
   range is written as part of that edit; the list is applied from the last edit to the first, and
   every span refers to the text before any edit. This amends P5, which ADR 0006 records.
2. **A repair is certain as a whole, and the contract rule applies to the whole.** A repair is
   offered only when exactly one repair makes the text what the writer evidently meant, as
   `docs/grammar.md` section 7 states. On the contract, the list taken together changes no meaning:
   it respells, adds a missing delimiter, deletes a duplicate, or moves a statement to the place
   where it means the same. One edit of the list may alone change a meaning (the deletion of
   `req k < n` drops a precondition) when the list as a whole does not (the same bound is inserted
   into `k`'s type). This is how ADR 0006 already reads in practice: P2 repairs `equiv` to
   `equiv allow()`, an edit on the contract that changes no meaning, while "What this does not
   license" rules out repairs that change what is proven (a permission, a `req`, the spec).
   The compiler offers a repair and its parser's recovery assumes it; it never edits the source.
3. **Five kinds join the closed set**, in `docs/diagnostics.md`:
   - `bound-unproven`, class bound proof: an obligation that a value stays in a range fails, with
     a counterexample. It covers every range obligation ADR 0008 lists: an index or a `load` within
     its region's extent, an integer operation within its type, a nonzero divisor, a result within
     its declared bound, the spec's definedness (the same obligations raised in the spec), and the
     per-target pointer-width arithmetic. The obligation field names which one.
   - `equivalence-unproven`, class equivalence proof: the impl is not equivalent to the spec under
     the granted permissions; the counterexample carries what both compute.
   - `ens-unproven`, class equivalence proof (decision 4).
   - `solver-unknown`, class solver timeout: the solver answered `unknown` or reached its time or
     resource limit on an obligation. One kind for both, since the writer's next move is the same
     and ADR 0004 counts them as one cause; never rendered with a counterexample, never a success
     (P5). The message says which limit was reached.
   - `internal-error`, class toolchain bug: the compiler failed on its own account (a broken
     invariant, an LLVM or Z3 error the input does not explain, a solver model that does not
     satisfy the negated obligation).
4. **A failed `ens` is an equivalence-proof failure, with a kind of its own.** An `ens` states what
   the function computes, not whether an operation is defined: it is proven of the spec, and of the
   impl's result under a non-empty `allow` list (ADR 0008). Its failure means that two statements
   of what the function computes disagree, the spec against the `ens`, or the impl against it, which
   is the question the equivalence class counts; the bound class counts whether an operation is
   defined. A bounded result (`-> u64 <= n`) is proven like an `ens` but states a range, so it is a
   `bound-unproven`: the class follows the shape of the obligation, a range or a relation. The kind
   stays distinct from `equivalence-unproven` because P4 revises each redundancy check by counting
   the diagnostics it raises, and the redundant `ens` is one of those checks.
5. **Fields by class.** The proof classes always carry an obligation and a counterexample, the
   solver class an obligation and never a counterexample, the syntax and type or layout classes
   neither, and only those two classes carry a repair. A counterexample's outputs (the spec's and
   the impl's values) are required on `equivalence-unproven`, optional on `ens-unproven`, absent
   elsewhere. A parser of records rejects a record that breaks these rules.
6. **Positions.** Byte offsets are 0-based and half-open; lines are 1-based and split on LF only;
   columns are 1-based and count Unicode code points. A model reads code points, not bytes, and a
   column in bytes would put `Σ` two columns wide; byte offsets stay in the record for tools. On
   text that is not valid UTF-8 (the `invalid-encoding` case), a column counts the bytes that are
   not UTF-8 continuation bytes, which is the code point count wherever the text is valid.
7. **The R rendering carries a `class` key**, derived from `kind` and checked when a record is
   parsed. The harness classifies a failure from the record alone, without a copy of the kind table
   that could drift, and a record whose class disagrees with its kind is refused rather than
   miscounted.
8. **The expected tokens of `unexpected-token` are in the message**, not in a field. A field read
   by one kind would make the schema depend on the kind; the writer reads the message anyway.
9. **No severity.** Every record is an error, as C2's layout (`error[<kind>]`) already writes it:
   `lemma check` has no warnings, since a warning is a message the writer may ignore.
10. **T carries every field but the byte offsets**, which line and column determine in the file:
    the end of a span over several lines, or of a span whose snippet cannot be shown, is a labelled
    line, and related spans and edits show their start and end.
11. **The set of kinds is closed by the documents.** A kind exists when a table of
    `docs/diagnostics.md`, of `docs/grammar.md` section 7 or of ADR 0007 lists it, each kind in
    exactly one table, with its class; a test ties those tables to the code. Effect kinds are added
    by the change that implements the effect check, and the kinds of the JIT's spec-versus-impl
    differential tests by the change that implements those tests, each with its class decided
    there. "Wrong spec" has no kind: only the spec tests (P9), the spec review (P8) and the hidden
    tests reveal it.

## Consequences

- A leaf crate, `lemma-diagnostics`, holds the record, the kind set with its classes and both
  renderings, so that the classification is one table in one place and every phase (parser,
  checker, verifier, JIT) depends on it.
- The ADR 0004 harness reads `class` from the R rendering; the calibration of C2 compares R and T
  as written in `docs/diagnostics.md`, which therefore freezes with the grammar.
- P4's per-check revision can count `ens-unproven` directly, and P5's refutation metric counts
  `bound-unproven`, `ens-unproven` and `equivalence-unproven` as proof-class failures.
- For the verifier: every failed obligation produces exactly one of the proof kinds, with the
  obligation named by its construct and the clauses it assumes, and a counterexample whose inputs
  are the obligation's free variables in a fixed order; an `unknown` or a limit produces
  `solver-unknown`, never a counterexample; a model the solver returns that does not refute the
  obligation is an `internal-error`. No repair is attached to a proof failure.
- For the parser: `bound-in-clause` produces a repair of two edits, and `clause-order` may produce
  several; recovery applies the whole list. `unexpected-token` lists the expected tokens in its
  message.
- For codegen: no new LLVM fact. For the JIT: a lowering or JIT failure the input does not explain
  is an `internal-error`; the differential-test kinds are still to come.
- ADR 0006 carries a one-sentence amendment under P5 pointing here.
- Open: the literal notation of a counterexample value for a region's contents, which the grammar
  has no literal for, is fixed with the verifier's first counterexample over a region; until then
  a counterexample names scalar variables only.
