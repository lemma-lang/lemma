# 6. Every surface optimized for the models that write and read it

- Status: accepted
- Date: 2026-10-19
- Amended: 2026-10-20, one sentence added under C1 extending it to the universal quantifier,
  approved by the maintainer; see ADR 0008
- Amended: 2026-10-20, one sentence added under P5 making a repair a list of edits, decided by the
  maintainer; see ADR 0009

## Context

Lemma is made by AI for AI: a model writes the code, and a model reads it, whether to review a
contract, to call a function or to change it. The surfaces drawn so far follow human taste:
`examples/dot.lm` and the README use the mathematical symbols `Σ`, `∈` and `≈`, `;` comments,
blocks delimited by indentation, clauses aligned in columns, coined abbreviations (`ld`, `hsum`),
and two notations for one operation (`*` in the spec, a prefix `add` in the impl). Nothing is
parsed yet. The grammar of the prototype subset is the next piece of work, so the criterion that
chooses between spellings must be fixed now, before the grammar fixes them by default.

The model learned from human code, so a familiar human convention is often the right choice for
the machine too. The two part ways where human convenience costs a model: sugar and synonyms
(each alternative is one more choice to get wrong), terse names, meaning carried by layout,
implicit conversions, and errors written as prose. A person avoids typing and dislikes verbosity;
a model does not tire of typing, pays for every alternative it must choose between, and gets more
out of data than out of prose.

One problem does not go away when no person reads the code. **A proof shows that the impl matches
the spec; it never shows that the spec matches the intent.** A wrong spec that verifies is a
silent bug, and ADR 0004 counts it as one. A trust story in which a person reads the spec answers
that problem by hand; Lemma does not rely on it, and answers it with two mechanisms a machine can
run (P8 and P9).

ADR 0004 treats "fewer tokens to a correct kernel" and "intuitive for a language model" as
hypotheses its experiment tests, not as design goals. This ADR does not change that. It sets the
criterion by which surfaces are chosen; ADR 0004 measures whether the choice paid off, and Lemma
claims no gain until it has.

Research inputs, outside Git and not decisions (a research report kept outside the repository):
published work reports that structured diagnostics and concrete counterexamples are the strongest
lever on a model's repair loop, with gains flattening after two or three rounds; that
compiler-generated, IR-like names hurt models; that a grammar without formatting tokens saves
tokens at an identical AST; and MoonBit's practice of testing syntax choices by having models
generate code with them.

## Decision

### Every surface is for a model

Every surface of Lemma is optimized for the models that write and read it and for the compiler that
answers them. None is designed for a human reader:

| Surface | Written by | Read by |
|---|---|---|
| The impl: statements, operations, loops and carried values | the writing model | the compiler; a model that changes it |
| The contract: the signature (parameters, types and layout, regions, result), `eff`, `req`, `ens`, `spec`, and `equiv` with its `allow` list | the writing model | the compiler, the spec reviewer (P8), the spec tests (P9), a model that calls the function |
| Lexical rules, keywords, operators, literals, comments | | every model above |
| The diagnostics `lemma check` returns, from the parser, the type and effect checks, the verifier and the JIT's differential tests | the compiler | the writing model |
| Canonical formatting | the compiler | every model above |
| The generated C header (ADR 0005) | the compiler | C compilers, binding generators, a model writing a caller |
| A function's documentation | the writing model | a model that calls or changes the function |

The canonical source text stays available for a human audit, and nothing is made hostile to a
person; but when the needs of a model and the habits of a person differ, the model's needs decide.

### Principles

Each principle is a hypothesis about the models, tied to the ADR 0004 metric it should move, with
the observation that would refute or revise it, read from the Lemma arm of the measured runs.
Like ADR 0004's two hypotheses, they gate nothing: a refuted principle is revised by a new ADR,
using the failure classification to name the construct at fault. The thresholds are fixed here,
before any run.

**P1. Familiar spellings, and no false friends.** Where mainstream languages have a frequent
spelling for a meaning, Lemma uses it (`u64`, `fma`, `splat`, `load`). The criterion is frequency
for that meaning, not length: a coined abbreviation is not used (`ld`, `hsum`), and a long name is
not preferred to an established short one (`fma`, not `fused_multiply_add`). The impl reads like
source, not like IR: values carry the names their writer gave them, and no surface a model reads
shows a compiler-generated name. A familiar spelling whose familiar meaning differs from Lemma's is
a **false friend** and is not used; the meaning is never bent to fit the spelling. Examples: `;`
as a comment marker (in the C family it ends a statement, so a model writing C habits would
comment out the rest of a line without any error), `~=` for approximate equality (it means "not
equal" in Lua and MATLAB), and any operator precedence mainstream languages disagree on (C's
bitwise operators below comparisons): operators combine without parentheses only under the
precedences every mainstream language shares (`*` over `+`, arithmetic over comparison over
logic), and parentheses are required everywhere else.

- Hypothesis: raises first-try success.
- Metric: first-try success, and the share of syntax-class failures.
- Refuted if syntax-class failures make up more than 25 % of Lemma's failed first submissions,
  pooled. Separately, a spelling borrowed from another language that recurs in more than 10 % of
  syntax-class failures names a Lemma spelling to revisit.

**P2. One canonical form per construct.** Each construct has one spelling and one structure: no
sugar, no alternative syntax, no optional part. The same operation has the same spelling in the
spec and in the impl. The canonical layout is line-oriented and alignment-free: one statement or
clause per line, fixed indentation, no column alignment, since alignment makes a one-line edit
touch its neighbors and spends tokens on whitespace. The formatter is the printer of the AST: it
is idempotent, parsing its output yields the same AST, and since functions are keyed by their AST,
formatting never changes a hash. `lemma check` accepts any layout (P6); the canonical form is what
is stored and shown back. A synonym a model is likely to try (another language's spelling, the
losing variant of a calibrated choice) is recognized only to be rejected, with the canonical
spelling as the diagnostic's repair.

The equivalence clause is the first application. Its two sides are always the spec and the impl,
so it names neither, and it carries no relation symbol: its content is the set of relaxations it
grants. Every function has exactly one, in one form, `equiv allow(<permissions>)`:

```
  equiv allow(fp_reassoc, fp_contract)   // equal up to reassociation and contraction
  equiv allow()                          // exactly equal
```

The list is always written, empty for exact equivalence, so exactness is stated rather than
inferred from an absence, and the permissions always sit in the same place. `equiv` without a
list is rejected with `equiv allow()` as its repair.

- Hypothesis: lowers tokens to a correct kernel, and makes repairs converge: a non-canonical
  spelling, once corrected, does not come back.
- Metric: tokens to a correct kernel; success within 3 rounds.
- Refuted if, in more than 10 % of the runs where a synonym is rejected, the same synonym appears
  again in a later submission of the same run: the repair is not taken, and the canonical spelling
  should become the one models keep writing.

**P3. Locality.** What determines the meaning of a line is in that line, in the function's
signature, and in the definitions of the values the line names (one each, by SSA); beyond the
function, only the signatures of the functions it calls by name. Hence: no implicit conversion; no
overloading resolved at a distance (an operation is chosen by its spelling and the types of its
operands, both visible in the function); no import that changes what a name means; no mode set
elsewhere, such as a module-wide fast-math flag (floating-point permissions live in the function's
`equiv` clause); and the type of every value visible on the line that defines it, by an annotation
or by an operation whose spelling fixes it (`load.f32x4`). The contract is local in a stronger
sense: it names nothing the impl defines, so it can be read, reviewed (P8) and evaluated (P9)
without the impl.

- Hypothesis: raises success within 3 rounds, because the span a diagnostic reports is where the
  fix belongs.
- Metric: success within 3 rounds.
- Refuted if, in more than 20 % of the Lemma submissions that fix their predecessor's failure, the
  edit that fixed it lies entirely outside the spans the diagnostic reported (measured by diffing
  consecutive submissions).

**P4. Redundancy the compiler checks is welcome, in the impl and in the contract alike;
redundancy it cannot check is not.** Stating a fact twice costs tokens, and in exchange an error is
caught where the two statements disagree rather than where a proof fails, far from the cause.
Effects and the result type are declared even where they could be inferred, and the compiler
checks them against the code; inside the impl, the types of carried values and the step of a loop
are stated and checked the same way. In the contract, an `ens` clause may state again, in another
form, what the spec implies (the result is zero when `n` is zero, or the reassociated-sum error
bound for a floating-point sum), and the verifier proves the two consistent: a spec that misreads
the intent then has to misread it twice, in two shapes, to verify. A statement the compiler cannot
check (a comment asserting an invariant, documentation restating the spec) is no substitute: it
drifts, and a model reading it may trust it.

- Hypothesis: raises success within 3 rounds and lowers silent bugs, at a token cost.
- Metric: success within 3 rounds, silent bugs, tokens to a correct kernel.
- Revised, per check, if more than half of the diagnostics a redundancy check raises are on
  submissions whose code was right and whose redundant statement alone was wrong: that check costs
  rounds without catching bugs, and the annotation it checks is dropped.

**P5. Diagnostics are data; prose is a rendering.** Every message of `lemma check` is first a
record, from which a text rendering is derived, never the reverse. A record has:

- `kind`: a stable identifier from a closed set. Each kind belongs to exactly one failure class of
  ADR 0004 (syntax, type or layout, effect, bound proof, equivalence proof, solver timeout,
  toolchain bug), so the experiment's classification of failures is mechanical, except "wrong
  spec", which only the spec tests (P9), the spec review (P8) and the hidden tests reveal;
- `span`: the file, the byte range, and the line and column of its start and end; related spans
  (the definition of a value, the clause an obligation comes from) as secondary spans;
- `message`: one sentence, in the terms of the reference card;
- for a failed proof, `obligation`: which obligation failed, named by the construct it is about
  and the clauses it assumes (the bound of the load at `a[i+12]`, under `req n % 16 == 0`); and
  `counterexample`: a concrete value for every free variable of the obligation, written as Lemma
  literals, with, for an equivalence, what the spec and the impl compute on those values;
- `repair`, optional: an edit (a span and its replacement text) when the compiler knows the fix
  for certain, such as a canonical spelling or a missing delimiter. Never a guess, and never on the
  contract (see "What this does not license").
  A repair is a list of one or more non-overlapping edits applied together, all or none, certain as
  a whole, and on the contract the list as a whole changes no meaning (ADR 0009).

A solver `unknown` or timeout is a kind of its own, never rendered as a failure with a
counterexample nor as a success. The output is deterministic (the same input gives the same bytes,
ordered by span), has no cascades (a diagnostic that only follows from an earlier one is not
emitted, so one mistake yields one diagnostic), and carries nothing ambient: no progress, timing
or color codes in what the model receives. Which rendering the model is shown is choice C2 below.

- Hypothesis: raises success within 3 rounds, most of all after proof failures, the ones prose
  errors help least.
- Metric: success within 3 rounds; tokens to a correct kernel.
- Refuted if the share of Lemma's proof-class failures repaired by the next submission is lower
  than the share of Verus's verification failures repaired by the next submission.

**P6. No meaning in layout.** Spaces, line breaks and indentation separate tokens and mean nothing
else; blocks and clauses are delimited by tokens. The grammar must satisfy one testable property:
once comments are removed, replacing every run of whitespace by a single space yields the same
AST. Line comments are the only construct that ends at a line break. Besides the cost to the
writer, a wrongly indented line that still parses moves a statement into or out of a loop without
any error, and in a spec no proof would catch it. Layout in a submission is never an error.

- Hypothesis: no failure and no silent bug has layout as its cause, and explicit delimiters do not
  become a source of failures of their own.
- Metric: silent bugs; the share of syntax-class failures.
- Refuted if unbalanced or misplaced delimiters cause more than 5 % of Lemma's failed submissions.

**P7. Regular lexing.** The lexer needs no feedback from the parser: keywords are reserved
everywhere, none is contextual, no token's kind depends on what precedes it, and the longest match
always wins. Identifiers and keywords are ASCII. Non-ASCII text appears only in comments and, if
choice C1 selects them, in a fixed set of contract symbols, each exactly one code point; its
lookalikes are rejected with a repair (`Σ` is U+03A3 GREEK CAPITAL LETTER SIGMA, so U+2211 N-ARY
SUMMATION is refused). The keyword list is kept short.

- Hypothesis: models do not trip over tokens whose meaning shifts with context; and the lexer
  stays simple to fuzz and to reuse in the formatter and tools.
- Metric: first-try success (syntax class).
- Revised if diagnostics for a reserved word used as a name exceed 5 % of syntax-class failures:
  the list is too long, or it claims a word models use as a name.

**P8. Independent spec review.** A function's contract is checked against the task statement by a
model that did not write it, preferably from another model family, so that the reviewer does not
share the writer's misreadings. The reviewer sees the task statement and the contract's canonical
source text, and nothing else: not the impl, which would anchor it on what was built rather than on
what was asked, and not the writer's comments or documentation, which carry the writer's reading
of the task. It answers whether the contract states what the task asks, and if not, which clause
departs from it and on which input. The review is a step of the workflow around the compiler, not
of the compiler: the toolchain's part is to emit a function's contract alone, in canonical form,
comments removed. A review finding is a finding, never an automatic repair.

- Hypothesis: lowers silent bugs caused by a wrong spec.
- Metric: silent bugs, the "wrong spec" class.
- Measured outside the loop, so that ADR 0004's protocol is unchanged: the reviewer is run on the
  contract of every accepted Lemma submission of the measured runs, its verdicts are not fed back,
  and they are compared with the hidden tests' verdicts.
- Refuted if it flags fewer than half of the contracts the hidden tests show to be wrong, or flags
  more than 20 % of the contracts the hidden tests show to be right.

**P9. The spec is executable and tested.** The compiler can evaluate a spec on concrete inputs,
with the semantics the proofs use: every spec construct is bounded (sums and quantifiers range over
finite domains) and fully defined, down to the order and start value of a floating-point sum. A
spec can therefore be run against test vectors drawn from the task (known-answer vectors, edge
cases, the hidden and visible tests), so a spec that is consistent with its impl but wrong is
caught by testing, not only by proof, and can be caught before any impl exists. When a test fails,
evaluating both sides tells a wrong spec from a wrong impl mechanically. The README's development
loop already runs spec-against-impl tests, which presumes an executable spec; this principle makes
it a requirement of every spec construct, and adds tests of the spec against the task.

- Hypothesis: wrong specs that verify become rare.
- Metric: silent bugs, the "wrong spec" class; the failure classification.
- Measured with ADR 0004's existing tests: in the Lemma arm, the toolchain evaluates the spec on
  the visible test's cases as well as the impl, and the harness evaluates it on the hidden test set
  to attribute each silent bug to the spec or to the impl.
- Refuted if more than half of Lemma's silent bugs are wrong specs.

### Candidates reshaped or rejected

- **A human-readable contract**, read by a person who checks it: rejected. The reader of the
  contract is a model. The problem it answered, a spec that verifies but does not match the
  intent, is answered by P8 and P9, which a machine can run on every function.
- **"Explicit rather than terse names"**: reshaped into P1. Explicitness is not what helps a
  model; frequency is. `fma` beats `fused_multiply_add`, and `load` beats `ld`, for the same
  reason.
- **"Tokenizer cost" as a criterion**: rejected as a primary criterion, kept as the last
  tie-breaker. A spelling saves at most a token per use; one extra round of feedback costs the
  whole kernel again, and ADR 0004 counts tokens over all submissions of a run.
- **Minimizing tokens as a goal**: rejected. ADR 0004 makes it a hypothesis, and a dense notation
  bought at the price of a round would lose on the very metric it targets.
- **Permissive parsing**, accepting the forms a model is likely to produce to raise first-try
  success: rejected. Every accepted synonym is a second form for the formatter, the card, the
  reviewer and every later tool. Likely forms are recognized and rejected with a repair instead
  (P2).
- **"Redundancy is welcome"** unqualified: reshaped into P4. Only redundancy the compiler checks
  is welcome.
- **"Everything a line means is visible on that line or in the signature"**: reshaped into P3,
  which also admits the line defining each named value. Restating every type at every use would
  cost tokens without adding a check.
- **A relation symbol in the equivalence clause** (`spec ≈ impl`): rejected. Both sides are fixed
  and the relation is determined by the permissions, so the symbol carried no information, and its
  ASCII spellings are false friends (P2).

### What this does not license

- **Nothing a proof depends on is dropped or inferred for brevity**: types with their layout,
  alignment, regions, effects, `req` and the `allow` list stay explicit and written.
- **The contract is never repaired automatically, and `allow` is never suggested.** Adding a
  permission or a `req` clause, or editing the spec, changes what is proven. A diagnostic may say
  which permission would admit the impl, marked as a contract change; deciding it is the contract
  author's call, never the compiler's, and a spec review finding (P8) is a finding, not an edit.
- **Effects are not hidden.** Inference may check a declared effect; it never replaces it.
- **The contract is not weakened to be easier to write**: no inferred `req`, no default
  permission, no spec construct without an executable, fully defined meaning (P9).
- **Familiarity governs spellings, never semantics.** A familiar meaning that differs from Lemma's
  (wrapping integer overflow, C's `%` on negative operands, fast-math) is not adopted because a
  model expects it.
- **No tuning to the measured kernels**: the calibration below runs on kernels disjoint from
  ADR 0004's.

### Open choices, settled by a calibration before the syntax is frozen

Two choices are genuinely open; the principles do not settle them.

**C1. The contract's mathematical symbols**, the sum and range membership, at every occurrence, in
the spec and in the impl alike (P2):

| Variant | Spellings | For it |
|---|---|---|
| U, Unicode | `Σ`, `∈`, one code point each, lookalikes rejected | the standard mathematical notation; frequent in Lean, Agda and Isabelle; compact |
| A, ASCII | `sum`, `in` | frequent in code and in SMT-backed verifiers (Dafny, Verus and Why3 spell quantifiers in ASCII); no lookalikes; the spelling of iteration in the mainstream languages models write most |

Token counts per occurrence are close, and decide only the last tie. **Default on a tie: A.** The
reader is a model, and in code, familiarity wins.
The universal quantifier follows the same choice, `∀` (U+2200 FOR ALL) under U and `forall` under
A, selected by the same switch, so the calibration kernels must use it too.

**C2. The rendering of diagnostics the model is shown**:

| Variant | Form |
|---|---|
| R, record | the record itself, one JSON object per diagnostic |
| T, text | the rendering derived from the record, in the familiar compiler layout (`error[<kind>]: <message>`, `--> file:line:column`, the source line), followed by the obligation, the counterexample and the repair as labelled lines; it carries every field of the record |

Both carry the same information; only the form differs. **Default on a tie: T**, the familiar
form, and shorter. Whichever wins, the record stays the source and both renderings remain
available to tools.

The grammar may add a choice to the calibration only by naming its two variants and its default
before the calibration starts. Each added choice doubles the arms below.

**Protocol.**

- **When**: after the prototype toolchain works for the subset (parser, verifier, lowering, JIT),
  before it is frozen, before the reference cards are written, and before ADR 0004's pilot.
- **What**: Lemma only, with ADR 0004's model, rounds (one attempt and at most 3 rounds of
  feedback), visible test and hidden-test discipline. A change of model voids the calibration, as
  it voids the experiment's runs.
- **How the runs are executed**: possibly through Claude Code's headless mode (`claude -p`) on a
  subscription rather than through an API key, in which case the sampling settings may not be
  controllable. They are then whatever that mode uses, and they are made the same for every arm
  and every run, not set: one pinned Claude Code version and one model identifier for the whole
  calibration, both recorded with every run, and a change of either voids the runs made after it.
  Each run starts a new session in an empty working directory, with no project instructions, no
  memory and no tools beyond what the protocol gives, so an arm sees only its own card. Tokens are
  counted as ADR 0004 counts them; any constant prompt the harness adds is the same in every arm
  and reported.
- **Kernels**: five calibration kernels, disjoint from ADR 0004's seven, and for that reason never
  eligible as the three new kernels of a re-run (ADR 0004 requires those to be unused while the
  language changes). Together they must use every C1 symbol and produce every class of diagnostic
  C2 renders: at least one floating-point reduction under a non-empty `allow` list, one integer
  kernel under `allow()`, one elementwise loop, and one kernel with a `req`. Candidates: sum of
  squares, scale-and-clamp, maximum of an array, Fletcher-16 checksum, three-point stencil. Each
  gets a scalar reference and a hidden test set under ADR 0004's rules.
- **Design**: two by two (U or A, R or T), four arms whose reference cards differ only in the
  varied element; 4 runs per kernel per arm, 80 runs. Each choice compares 40 runs with 40, paired
  by kernel. In each arm the parser accepts only that arm's C1 spellings and rejects the other's
  with a dedicated kind and its repair, so how often a model reaches for the other spelling is
  counted too.
- **Decision rule, for each choice**, in order:
  1. a variant under which a silent bug is traced to the varied element loses;
  2. otherwise, a variant whose pooled success within 3 rounds is at least 15 points higher, and
     which wins on at least as many kernels as it loses, wins;
  3. otherwise, a variant whose median tokens to a correct kernel is at least 15 % lower wins;
  4. otherwise, the default.

  The rule asks for no statistical significance on purpose. Forty runs against forty bound a
  difference within about ±22 points; the calibration exists to catch a large difference, and
  when the difference is small either variant costs little, so the default decides.
- **Record**: the results and the choices go into a new ADR that freezes the prototype's grammar,
  before the pilot. Then the reference cards are written and frozen, and the toolchain is frozen,
  as ADR 0004 requires.

**How this fits ADR 0004.** No threshold, kernel, metric, run count or pilot of ADR 0004 changes.
Calibration runs are not runs of the experiment: other kernels, Lemma alone, made before the
toolchain and the cards are frozen, discarded, and gating nothing; ADR 0004 says so under "Runs",
in a sentence added for this ADR. ADR 0004's pilot keeps its sole purpose, debugging the harness,
and runs on the frozen syntax. The calibration gives Lemma a design iteration that Rust and Verus
do not get in the experiment; that is the design of the language under test, done before the
measurement, and the disjoint kernels are what keep it from fitting the measured ones. P8 and P9
are measured on ADR 0004's runs without changing its protocol: the spec review runs outside the
loop, and the spec evaluation is part of Lemma's toolchain output on the visible test.

## Consequences

- **For the grammar of the prototype subset**: it is written under P1 to P9. Concretely, comments
  are `//` to the end of the line, and a `;` is rejected with a repair; blocks are delimited by
  tokens, and the grammar passes the whitespace property of P6; one spelling per operation across
  spec and impl, so arithmetic and comparisons are infix under the shared precedences, and named
  operations keep a mainstream spelling where no infix exists (`fma`, `splat`, `load`, and a
  horizontal sum spelled as mainstream SIMD APIs spell it, not `hsum`); the type of every value is
  visible at its definition; identifiers and keywords are ASCII and keywords reserved; the
  equivalence clause is `equiv allow(...)`, required, with `allow()` for exact equivalence; every
  spec construct is bounded and has a defined evaluation; both C1 variants are written out
  exactly, with the list of rejected lookalikes; and each syntax diagnostic kind is listed with its
  ADR 0004 failure class.
- **`examples/dot.lm` is rewritten in that grammar.** The rewrite changes the spelling of its
  contract clauses, not their meaning (`equiv spec ≈ impl allow(fp_reassoc, fp_contract)` becomes
  `equiv allow(fp_reassoc, fp_contract)`), so the contract check between the tests commit and the
  implementation will show those lines; the change of spelling is expected, and the unchanged
  meaning is what is reviewed.
- **The meaning of a floating-point sum must be decided before the spec's sum is frozen**: its
  evaluation order, its start value (`+0.0` or `-0.0`, which decides the sign of a zero result) and
  the empty sum. P9 requires it, and `dot`'s proof depends on it.
- **For the parser**: a lexer with no parser feedback; property tests for the whitespace property
  (P6) and for the formatter (idempotent, round trip to the same AST); the C1 variant behind a
  switch, one variant active per run, the other's spellings recognized only to be rejected with a
  repair, and kept that way after the freeze for the losing variant; a diagnostic record carrying
  kind, span (byte range, line and column), related spans, message and optional repair, with both
  renderings derived from it, deterministic, and covered by snapshot tests in both forms; no
  cascades, for which at most one syntax diagnostic per function, resuming at the next function,
  is the simplest compliant strategy.
- **For the verifier**: every obligation carries a name tied to its source construct and the
  clauses it assumes; a counterexample assigns every free variable a value written as a Lemma
  literal, and an equivalence failure also shows what the spec and the impl compute on it; `unknown`
  and timeouts are a kind of their own; no repair is offered on the contract; redundant `ens`
  clauses are proven consistent with the spec.
- **For the toolchain**: an evaluator of specs on concrete inputs, sharing the proofs' semantics
  (in the JIT, or an interpreter), and a command that emits a function's contract alone, in
  canonical form, without comments, for the spec review.
- **For codegen**: no new LLVM fact. P3 rules out any module-wide floating-point mode: the flags
  `docs/design.md` lists are emitted per function, from that function's `allow` list only.
- **For the JIT**: a failing spec-versus-impl differential test is reported as a record, with the
  input values and both results, in the same two renderings.
- **For the experiment**: five more kernels with references and hidden tests, and 80 calibration
  runs, before the pilot; a spec reviewer and its prompt, frozen before the measured runs like the
  cards; spec evaluation on the visible and hidden tests. The failure classification of ADR 0004
  becomes mostly a reading of the diagnostic kinds and of the spec evaluation.
- **For the README**: the spellings of its snippet stay provisional until the calibration
  settles them. It must not claim that Lemma costs fewer tokens
  or is intuitive for a model: ADR 0004 tests those claims. It must not present a person reading
  the spec as Lemma's safeguard.
- The parser carries two spellings of the contract symbols until the freeze, and the losing
  spellings' rejection diagnostics afterwards.
